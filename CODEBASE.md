# Codebase Structure

## Overview

`ask` is a Rust CLI that allows users to interact with AI models using natural language without quotes. The architecture follows a modular design with clear separation of concerns.

## Directory Structure

```
ask/
├── src/
│   ├── main.rs              # Entry point
│   ├── cli/
│   │   ├── mod.rs           # CLI execution logic
│   │   └── parser.rs        # Flexible argument parsing
│   ├── config/
│   │   ├── mod.rs           # Config structs and init_config()
│   │   ├── loader.rs        # TOML config loading hierarchy
│   │   └── defaults.rs      # Default constants
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
│   │   └── injector.rs      # Terminal command injection (uinput/enigo)
│   ├── output/
│   │   ├── mod.rs           # Module exports
│   │   ├── formatter.rs     # Output formatting (JSON, raw, markdown)
│   │   ├── markdown.rs      # Terminal markdown rendering
│   │   └── colorize.rs      # Color scheme utilities
│   ├── update/
│   │   └── mod.rs           # Auto-update from GitHub releases
│   └── completions.rs       # Shell completions generation
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
    pub command_mode: bool,   // -x, --command
    pub yes: bool,            // -y, --yes
    pub think: Option<bool>,  // -t, --think[=bool]
    pub model: Option<String>,
    pub provider: Option<String>, // -P, --provider
    pub profile: Option<String>,  // -p, --profile
    pub api_key: Option<String>,  // -k, --api-key
    pub non_interactive: bool,    // -n, --non-interactive
    pub verbose: bool,            // -v, --verbose
    pub list_profiles: bool,      // profiles subcommand
    pub make_config: bool,        // --make-config
    pub query: Vec<String>,       // Free text parts
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

Configuration is loaded with precedence (Profile-First):
1. CLI arguments (highest)
2. Profile settings (selected via `-p` or `default_profile`)
3. Environment variables (`ASK_*`)
4. Local config (`./ask.toml`)
5. Home config (`~/ask.toml`)
6. XDG config (`~/.config/ask/config.toml`)
7. Defaults (lowest)

Key structures:
- `Config` - Main config container
- `DefaultConfig` - Default provider/model settings
- `ProviderConfig` - Per-provider API keys and URLs
- `ProfileConfig` - Named profile settings (provider, model, api_key, base_url, fallback)
- `BehaviorConfig` - Execution behavior settings
- `ContextConfig` - Context/history settings
- `ConfigManager` - Helper struct for interactive config management
- `aliases: HashMap<String, String>` - Command-line aliases

Key functions:
- `init_config()` - Interactive configuration menu
- `init_config_non_interactive()` - Non-interactive setup (for scripts)
- `load_aliases_only()` - Fast alias loading for early argument expansion
- `configure_defaults()` - Configure default provider/model
- `configure_profile()` - Configure a single profile
- `manage_profiles()` - Profile management submenu
- `show_current_config()` - Display current config formatted
- `list_profiles()` - List all profiles with details
- `get_thinking_config()` - Get unified thinking settings (enabled, value)
- `get_thinking_level()` - Get Gemini thinking level from profile
- `get_reasoning_effort()` - Get OpenAI reasoning effort from profile
- `get_thinking_budget()` - Get Anthropic thinking budget from profile

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
- **GeminiProvider**: Google Gemini API (thinking via `thinkingConfig`)
- **OpenAIProvider**: OpenAI and compatible APIs (reasoning via `reasoning_effort`)
- **AnthropicProvider**: Anthropic Claude API (thinking via `thinking.budget_tokens`)

**Thinking Mode Support**:
Each provider implements thinking/reasoning differently:
- `ProviderOptions.thinking_enabled` - Whether thinking is active
- `ProviderOptions.thinking_value` - Provider-specific value (level/effort/budget)

The unified prompt system handles intent detection inline (command vs question vs code) without a separate API call. Key functions:
- `build_unified_prompt()` - Builds the system prompt with context
- `load_custom_prompt()` - Loads custom prompts from ask.md files
- `expand_prompt_variables()` - Replaces {os}, {shell}, {cwd}, {locale}, {now} variables

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

### Output Formatting (`src/output/`)

Handles output based on flags:
- `--json`: Structured JSON output
- `--raw`: Plain text without formatting
- `--markdown`: Terminal markdown rendering (default)

Automatically detects piping and disables colors/formatting.

### Auto-Update (`src/update/`)

Implements automatic update checking and installation:

- **Background check**: Spawns detached process to check GitHub releases
- **Notification**: Saves update info for next run notification
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
8. **Output**: Stream or display response
9. **Execution**: For commands, optionally execute with safety checks

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
- `clap_complete`: Shell completions generation
- `tokio`: Async runtime
- `reqwest`: HTTP client with streaming and rustls
- `hickory-resolver`: Custom DNS resolution for Termux/Android compatibility
- `serde` + `toml`: Configuration parsing
- `colored`: Terminal colors
- `indicatif`: Progress spinners
- `termimad`: Markdown rendering
- `requestty`: Interactive CLI prompts with number key selection
- `arboard`: Clipboard support for command injection (Windows fallback)
- `mouse-keyboard-input`: Command injection via `/dev/uinput` (Linux)
- `enigo`: Command injection via Accessibility/keystrokes (macOS/Windows)
