# Codebase Structure

## Overview

`ask` is a Rust CLI that allows users to interact with AI models using natural language without quotes. The architecture follows a modular design with clear separation of concerns.

## Directory Structure

```
ask/
├── src/
│   ├── main.rs              # Entry point
│   ├── http.rs              # Custom DNS and HTTP client setup
│   ├── completions.rs       # Shell completions generation
│   ├── cli/
│   │   ├── mod.rs           # CLI execution logic
│   │   └── parser.rs        # Flexible argument parsing
│   ├── config/
│   │   ├── mod.rs           # Config structs and init_config()
│   │   ├── loader.rs        # TOML config loading hierarchy
│   │   ├── defaults.rs      # Default constants
│   │   └── thinking.rs      # Thinking mode configuration helpers
│   ├── providers/
│   │   ├── mod.rs           # Provider factory
│   │   ├── traits.rs        # Provider trait + PromptContext
│   │   ├── gemini.rs        # Google Gemini integration
│   │   ├── openai.rs        # OpenAI integration
│   │   └── anthropic.rs     # Anthropic Claude integration
│   ├── context/
│   │   ├── mod.rs           # Module exports
│   │   ├── storage.rs       # JSON file storage
│   │   └── manager.rs       # Context lifecycle management
│   ├── executor/
│   │   ├── mod.rs           # Module exports
│   │   ├── safety.rs        # Destructive command detection
│   │   ├── runner.rs        # Command execution
│   │   └── injector.rs      # Terminal command injection (tmux/screen/clipboard)
│   ├── output/
│   │   ├── mod.rs           # Module exports
│   │   ├── formatter.rs     # Output formatting (JSON, raw, markdown)
│   │   ├── markdown.rs      # Terminal markdown rendering
│   │   ├── colorize.rs      # Color scheme utilities
│   │   └── spinner.rs       # Loading indicator (● blinking/streaming)
│   └── update/
│       └── mod.rs           # Auto-update from GitHub releases
├── tests/
│   ├── integration_test.rs  # CLI integration tests
│   └── fixtures/            # Test fixtures
├── .github/
│   └── workflows/
│       ├── ci.yml           # CI/CD pipeline (lint, test, build, release)
│       └── test.yml         # Tests
├── Makefile                 # Development commands (precommit, fmt, clippy, test, audit)
├── CLAUDE.md                # Claude AI assistant instructions
├── GEMINI.md                # Gemini AI assistant instructions
├── install.sh               # Unix installation script
├── install.ps1              # Windows installation script
├── Cargo.toml               # Rust dependencies
├── LICENSE                  # AGPL-3.0
├── README.md                # User documentation
├── ROADMAP.md               # Development roadmap
├── ADR.md                   # Architecture decisions
└── CODEBASE.md              # This file
```

## Key Components

### CLI Parser (`src/cli/parser.rs`)

Implements flexible argument parsing that allows flags before or after free text:

```rust
pub struct Args {
    pub context: Option<u64>, // -c, --context[=MIN]
    pub command_mode: Option<bool>, // -x, --command / --question
    pub yes: Option<bool>,    // -y, --yes / --confirm
    pub model: Option<String>,
    pub provider: Option<String>, // -P, --provider
    pub profile: Option<String>,  // -p, --profile
    pub think: Option<bool>,  // -t, --think / --no-think
    pub think_level: Option<String>, // thinking level/budget/effort
    pub json: bool,           // --json
    pub markdown: Option<bool>, // --markdown / --no-markdown
    pub raw: bool,            // --raw
    pub color: Option<bool>,  // --color / --no-color
    pub stream: Option<bool>, // --stream / --no-stream
    pub search: Option<bool>, // -s, --search / --no-search
    pub citations: Option<bool>, // --citations / --no-citations
    pub verbose: bool,        // -v, --verbose
    pub list_profiles: bool,  // profiles subcommand
    pub make_config: bool,    // --make-config
    pub query: Vec<String>,   // Free text parts
    // ...
}
```

The parser:
1. Expands aliases from config before parsing
2. Iterates through arguments
3. Identifies flags (starting with `-`)
4. Collects remaining text as the query
5. Handles combined short flags (`-cy`)

### Configuration (`src/config/`)

Configuration is loaded with precedence (Profile-Only Architecture):
1. CLI arguments (highest): `-p`, `-P`, `-m`, `-k`
2. Environment variables: `ASK_PROFILE`, `ASK_PROVIDER`, `ASK_MODEL`, `ASK_*_API_KEY`
3. Project local config (recursive search upwards for `ask.toml` or `.ask.toml`)
4. Home config (`~/ask.toml` - legacy, still supported)
5. XDG config (`~/.config/ask/ask.toml` - recommended for new installs)
6. Hardcoded defaults (lowest)
7. Built-in free profile `ch-at` is injected if missing and remains available as `-p ch-at`

Key structures:
- `Config` - Main config container with profiles, behavior, context, update, commands, aliases
- `ActiveConfig` - Runtime-resolved config (provider, model, api_key, base_url, stream, profile_name)
- `ProfileConfig` - Named profile settings (provider, model, api_key, base_url, stream, fallback, thinking settings, web search)
- `BehaviorConfig` - Execution behavior settings
- `ContextConfig` - Context/history settings
- `ConfigManager` - Internal helper for interactive config management
    - `get_any_str()` - Retrieve any TOML value as a String (handles Integers/Booleans)
    - **Safe-by-Default Navigation**: Menu defaults to "Back"/"Exit" after actions
    - **Smart Persistence**: Pre-selects existing values when editing profiles
- `aliases: HashMap<String, String>` - Command-line aliases

Key functions:
- `with_cli_overrides()` - Resolves active config from CLI args, ENV, and profiles
- `init_config()` - Interactive configuration menu
- `init_config_non_interactive()` - Non-interactive setup (for scripts)
- `load_aliases_only()` - Fast alias loading for early argument expansion
- `configure_profile()` - Configure a single profile
- `manage_profiles()` - Profile management submenu
- `get_thinking_config()` - Get unified thinking settings (enabled, value)
- `get_thinking_level()` - Get thinking level (Gemini/Anthropic) from active profile
- `get_reasoning_effort()` - Get OpenAI reasoning effort from active profile
- `get_thinking_budget()` - Get thinking budget (Gemini/Anthropic) from active profile
- `get_profile_web_search()` - Check if web search is enabled for active profile

### HTTP Client & DNS Resolver (`src/http.rs`)

`ask` uses a custom HTTP client setup to ensure reliability across all platforms, including Termux/Android:

- **Hickory DNS**: Uses `hickory-resolver` with Cloudflare DNS (1.1.1.1) to bypass system DNS issues.
- **Cross-Platform**: Works without `/etc/resolv.conf`, making it robust for mobile environments.
- **Reqwest**: Integrated with `reqwest` for all API calls (Gemini, OpenAI, Anthropic, GitHub).

### Providers (`src/providers/`)

All providers implement the `Provider` trait:

```rust
#[async_trait]
pub trait Provider: Send + Sync {
    async fn complete(&self, messages: &[Message]) -> Result<String>;
    async fn stream(&self, messages: &[Message], callback: StreamCallback) -> Result<()>;
    fn name(&self) -> &str;
    fn model(&self) -> &str;
}
```

Supported providers:
- **GeminiProvider**: Google Gemini API (Thinking via `thinkingConfig` or `thinkingBudget`)
- **OpenAIProvider**: OpenAI and compatible APIs (Reasoning via `reasoning_effort`)
- **AnthropicProvider**: Anthropic Claude API (Thinking via `thinking.budget_tokens`)

**Thinking Mode Support** (`src/config/thinking.rs`):
The system dynamically detects and selects the appropriate parameter for each provider/model:
- **Gemini 2.5/Pro**: `thinking_budget` (tokens)
- **Gemini 3**: `thinking_level` (minimal, low, medium, high)
- **OpenAI (o1/o3)**: `reasoning_effort` (none, low, medium, high)
- **Anthropic**: `thinking_budget` (tokens or levels)

The unified prompt system handles intent detection inline (command vs question vs code) without a separate API call. Key functions:
- `build_unified_prompt()` - Builds the system prompt with context
- `load_custom_prompt()` - Loads custom prompts from ask.md files
- `expand_prompt_variables()` - Replaces {os}, {shell}, {cwd}, {locale}, {now} variables
- `flatten_command()` - Sanitizes multiline commands into one-liners using `&&`

### Context Manager (`src/context/`)

Manages conversation history per directory:

- **Storage**: JSON files in `~/.local/share/ask/contexts/`
- **Key**: SHA256 hash of current directory path
- **Cleanup**: Automatic removal after TTL expires

```rust
pub struct ContextManager {
    storage: ContextStorage,
    context_id: String,
    max_messages: usize,
    max_age_minutes: u64,
}
```

### Command Executor (`src/executor/`)

Safe command execution with pattern-based safety detection:

**Safe patterns** (auto-execute):
- `ls`, `pwd`, `cat`, `grep`
- `git status`, `git log`
- `docker ps`, `docker images`

**Destructive patterns** (require confirmation):
- `rm -rf`, `rm -r`
- `sudo *`
- `dd`, `mkfs`
- `curl | sh`

**Command Injection** (`src/executor/injector.rs`):
Automatically detects the best injection method for the current environment:

```rust
pub enum InjectionMethod {
    GuiPaste,      // Wayland/X11/macOS/Windows with GUI
    TmuxSendKeys,  // Inside tmux session
    ScreenStuff,   // Inside GNU screen session
    Fallback,      // Headless terminal - enhanced prompt
}
```

Detection order:
1. `$TMUX` → `tmux send-keys -l` (literal mode)
2. `$STY` → `screen -X stuff`
3. `$DISPLAY`/`$WAYLAND_DISPLAY` → clipboard + paste simulation
4. Otherwise → enhanced fallback (visual print + editable prompt)

### Output Formatting (`src/output/`)

Handles output based on flags:
- `--json`: Structured JSON output
- `--raw`: Plain text without formatting
- `--markdown`: Terminal markdown rendering (default)

Automatically detects piping and disables colors/formatting.

**Loading Indicator** (`spinner.rs`):
- `Spinner`: Blinks ● (500ms on/off) while waiting for AI response
- `StreamingIndicator`: Shows ● at end of text during streaming
- Only active in terminal mode (not raw/json/piped)

### Auto-Update (`src/update/`)

Implements automatic update checking and installation:

- **Background check**: Spawns detached process to check GitHub releases.
- **Throttling**: 
    - **Aggressive mode**: Checks at most once per hour.
    - **Normal mode**: Respects `check_interval_hours` (default 24h).
- **Notification**: Saves update info for next run notification.
- **Download**: Fetches platform-specific binary from release assets
- **Atomic replace**: Safe binary replacement with backup

Disable with `ASK_NO_UPDATE=1` environment variable.

### Shell Completions (`src/completions.rs`)

Generates shell completions using clap_complete:

```bash
ask --completions bash    # Bash completions
ask --completions zsh     # Zsh completions
ask --completions fish    # Fish completions
ask --completions powershell  # PowerShell completions
ask --completions elvish  # Elvish completions
```

### Custom Commands (`src/config/`)

Supports user-defined commands in config:

```toml
[commands.cm]
system = "Generate commit message"
type = "command"
auto_execute = false
provider = "anthropic"  # Optional override
model = "claude-3-opus" # Optional override
```

## Data Flow

1. **Input**: User runs `ask how to list docker containers`
2. **Alias Expansion**: Aliases from config are expanded (e.g., `q` → `--raw --no-color`)
3. **Parsing**: `Args::parse_flexible()` extracts flags and query
4. **Config**: Load configuration with precedence
5. **Provider**: Create appropriate provider based on config
6. **Intent**: Classify intent (COMMAND/QUESTION/CODE)
7. **Generation**: Send to AI with appropriate system prompt
8. **Post-processing**: Flatten multi-line commands into one-liners for terminal robustness
9. **Output**: Stream or display response
10. **Execution**: For commands, optionally execute with safety checks

## Testing

```bash
# Run all checks before committing
make precommit

# Run all tests
cargo test

# Run with verbose output
cargo test -- --nocapture

# Run specific test
cargo test test_safe_commands
```

## Building

```bash
# Development build
cargo build

# Release build (optimized)
cargo build --release

# Cross-compile (requires cross)
cross build --release --target aarch64-unknown-linux-gnu
```

## Development Commands

The `Makefile` provides convenient development commands:

- `make precommit` - Run all checks (fmt, clippy, test, audit)
- `make fmt` - Check code formatting
- `make clippy` - Run linter
- `make test` - Run tests
- `make audit` - Security audit
- `make build` - Debug build
- `make release` - Release build
- `make clean` - Clean artifacts

## Key Design Decisions

See [ADR.md](ADR.md) for architectural decisions including:
- Why JSON storage over Native DB
- Flexible argument parsing approach
- Context opt-in design
- Safety detection patterns
- Provider abstraction

## Dependencies

Key crates:
- `clap`: CLI utility functions and shell completions
- `tokio`: Async runtime
- `reqwest`: HTTP client with streaming and rustls
- `hickory-resolver`: Custom DNS resolution for cross-platform compatibility
- `serde` + `toml`: Configuration parsing
- `colored`: Terminal colors
- `indicatif`: Progress spinners
- `termimad`: Markdown rendering
- `requestty`: Interactive CLI prompts
- `arboard`: Clipboard support for command injection (all platforms)
- `enigo`: Key simulation for Cmd+V/Ctrl+V paste (macOS/Windows/Linux x86_64)
- `mouse-keyboard-input`: Key simulation for Linux (uinput)
- `clap_complete`: Shell completions generation
- `anyhow` + `thiserror`: Error handling
- `serde_json`: JSON serialization for context and output
- `sha2`: SHA256 hashing for directory-based context IDs
