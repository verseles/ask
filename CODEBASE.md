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
│   │   ├── traits.rs        # Provider trait + IntentClassifier
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
│   │   └── runner.rs        # Command execution
│   └── output/
│       ├── mod.rs           # Module exports
│       ├── formatter.rs     # Output formatting (JSON, raw, markdown)
│       ├── markdown.rs      # Terminal markdown rendering
│       └── colorize.rs      # Color scheme utilities
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
    pub context: bool,      // -c, --context
    pub command_mode: bool, // -x, --command
    pub yes: bool,          // -y, --yes
    pub model: Option<String>,
    pub provider: Option<String>,
    pub query: Vec<String>, // Free text parts
    // ...
}
```

The parser:
1. Iterates through arguments
2. Identifies flags (starting with `-`)
3. Collects remaining text as the query
4. Handles combined short flags (`-cy`)

### Configuration (`src/config/`)

Configuration is loaded with precedence:
1. CLI arguments
2. Environment variables (`ASK_*`)
3. Local config (`./ask.toml`)
4. Home config (`~/ask.toml`)
5. XDG config (`~/.config/ask/config.toml`)
6. Defaults

Key structures:
- `Config` - Main config container
- `DefaultConfig` - Default provider/model settings
- `ProviderConfig` - Per-provider API keys and URLs
- `BehaviorConfig` - Execution behavior settings
- `ContextConfig` - Context/history settings

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
- **GeminiProvider**: Google Gemini API
- **OpenAIProvider**: OpenAI and compatible APIs
- **AnthropicProvider**: Anthropic Claude API

The `IntentClassifier` uses the provider to classify user intent:
- `COMMAND` - User wants shell commands
- `QUESTION` - User wants information
- `CODE` - User wants code generation

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

## Data Flow

1. **Input**: User runs `ask how to list docker containers`
2. **Parsing**: `Args::parse_flexible()` extracts flags and query
3. **Config**: Load configuration with precedence
4. **Provider**: Create appropriate provider based on config
5. **Intent**: Classify intent (COMMAND/QUESTION/CODE)
6. **Generation**: Send to AI with appropriate system prompt
7. **Output**: Stream or display response
8. **Execution**: For commands, optionally execute with safety checks

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
- `clap`: CLI parsing (derive macros)
- `tokio`: Async runtime
- `reqwest`: HTTP client with streaming
- `serde` + `toml`: Configuration parsing
- `colored`: Terminal colors
- `indicatif`: Progress spinners
- `termimad`: Markdown rendering
- `dialoguer`: Interactive prompts
