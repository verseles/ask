# ask

> Ask anything in plain text, get commands or answers instantly. No quotes needed.

[![License: AGPL v3](https://img.shields.io/badge/License-AGPL%20v3-blue.svg)](LICENSE)

A CLI tool that lets you interact with AI models using natural language, without the need for quotes around your questions.

## Features

- **Natural input**: Just type `ask how to list docker containers` - no quotes needed
- **Flexible flags**: Put options before or after your question - both work!
- **Smart command injection**: Commands are pasted directly to your terminal for editing
- **Smart intent detection**: Automatically detects if you want a command or an answer
- **Multiple providers**: Supports Gemini (default), OpenAI, and Anthropic Claude
- **Streaming responses**: Real-time token-by-token output
- **Thinking mode**: Enable AI reasoning for complex tasks (`-t` flag or config)
- **Context awareness**: Optional conversation memory per directory
- **Safe command execution**: Detects and warns about destructive commands
- **Sudo retry**: Suggests retry with sudo on permission denied errors
- **Flexible configuration**: TOML config with environment variable overrides
- **Custom commands**: Define your own commands with custom system prompts
- **Piping support**: Works with `git diff | ask cm` style workflows
- **Auto-update**: Background update checks with notifications
- **Shell completions**: Bash, Zsh, Fish, PowerShell, Elvish support

## Installation

### Linux & macOS

```bash
curl -fsSL install.cat/verseles/ask | sh
```

### Windows (PowerShell)

```powershell
irm install.cat/verseles/ask | iex
```

### From source

```bash
cargo install --git https://github.com/verseles/ask
```

The installer will prompt you to configure your API keys automatically.

## Quick Start

```bash
# Initialize configuration (set up API keys)
ask init

# Ask questions naturally
ask how to list docker containers
ask what is the capital of France

# Commands are auto-detected and pasted to your terminal
ask delete old log files      # Command appears ready to edit/run
ask -y delete old log files   # Execute immediately

# Flags can go before OR after your question
ask -x delete old log files
ask delete old log files -x

# Enable thinking mode for complex reasoning
ask -t explain the theory of relativity

# Use context for follow-up questions
ask explain kubernetes -c
ask -c what about pods?

# Pipe input
git diff | ask cm
cat main.rs | ask explain this code
```

## Usage

Flags can be placed **before or after** your question - whatever feels natural.

```
ask [OPTIONS] <your question here>
ask <your question here> [OPTIONS]

OPTIONS:
    -c, --context         Use/create context for current directory
    -x, --command         Force command mode (bypass auto-detection)
    -y, --yes             Auto-execute commands without confirmation
    -t, --think           Enable thinking mode (override config)
        --no-think        Disable thinking mode (override config)
    -m, --model <MODEL>   Override configured model
    -p, --provider <NAME> Override configured provider
        --json            Output in JSON format
        --markdown        Output rendered in Markdown
        --raw             Output raw text without formatting
        --no-color        Disable colorized output
        --no-follow       Disable result echo after execution
        --update          Check and install updates
    -V, --version         Show version
    -h, --help            Show help

SUBCOMMANDS:
    init                  Initialize configuration interactively
    --clear               Clear current directory context (use with -c)
    --history             Show context history (use with -c)
```

## Configuration

Configuration is loaded from multiple sources (in order of precedence):

1. CLI arguments
2. Environment variables (`ASK_*`)
3. `./ask.toml` or `./.ask.toml` (project local)
4. `~/ask.toml` (home directory)
5. `~/.config/ask/config.toml` (XDG config)
6. Default values

### Example config.toml

```toml
[default]
provider = "gemini"
model = "gemini-3-flash-preview"
stream = true

[providers.gemini]
api_key = "YOUR_API_KEY_HERE"
thinking_level = "low"  # optional: none, low, medium, high

[providers.openai]
api_key = "sk-..."
reasoning_effort = "low"  # optional: none, minimal, low, medium, high

[providers.anthropic]
api_key = "sk-ant-..."
thinking_budget = 5000  # optional: token budget for reasoning

[behavior]
auto_execute = false
confirm_destructive = true
timeout = 30

[context]
max_age_minutes = 30
max_messages = 20

# Custom commands
[commands.cm]
system = "Generate concise git commit message based on diff"
type = "command"
auto_execute = false
```

### Environment Variables

```bash
ASK_PROVIDER=gemini
ASK_MODEL=gemini-3-flash-preview
ASK_GEMINI_API_KEY=...
ASK_OPENAI_API_KEY=sk-...
ASK_ANTHROPIC_API_KEY=sk-ant-...
ASK_STREAM=true
NO_COLOR=1  # Disable colors
```

## Providers

### Gemini (Default)

Google's Gemini models. Get your API key from [Google AI Studio](https://aistudio.google.com/).

### OpenAI

OpenAI's GPT models. Get your API key from [OpenAI Platform](https://platform.openai.com/).

### Anthropic Claude

Anthropic's Claude models. Get your API key from [Anthropic Console](https://console.anthropic.com/).

### OpenAI-Compatible

Any OpenAI-compatible API (e.g., Ollama, LM Studio):

```toml
[providers.openai_compatible]
api_key = "..."
base_url = "http://localhost:11434/v1"
model = "llama3"
```

## Thinking Mode

Enable AI reasoning/thinking for more complex tasks. Use `-t`/`--think` flag or configure in your config file:

```bash
# Enable thinking for a single query
ask -t explain quantum entanglement
ask how does RSA encryption work --think

# Disable thinking (if enabled in config)
ask --no-think what time is it
```

### Config Parameters

| Provider | Config Parameter | Values |
|----------|-----------------|--------|
| Gemini | `thinking_level` | `none`, `low`, `medium`, `high` |
| OpenAI | `reasoning_effort` | `none`, `minimal`, `low`, `medium`, `high` |
| Anthropic | `thinking_budget` | Token count (e.g., `5000`) |

Configure during `ask init` or manually in your config file.

## Safety Features

The CLI includes safety detection for potentially destructive commands:

- Commands like `rm -rf`, `sudo`, `dd`, etc. require explicit confirmation
- Use `-y` to bypass confirmation (use with caution)
- Safe commands like `ls`, `git status`, `docker ps` can auto-execute

## Context System

The optional context system (`-c` flag) maintains conversation history per directory:

```bash
# Start a conversation
ask -c how do I set up nginx

# Continue the conversation
ask -c what about SSL?

# Clear context
ask -c --clear

# View history
ask -c --history
```

Context is stored locally and automatically cleaned up after 30 minutes of inactivity.

## Custom Commands

Define reusable commands in your config file with custom system prompts:

```toml
[commands.cm]
system = "Generate a concise git commit message based on the diff provided"
type = "command"
auto_execute = false

[commands.explain]
system = "Explain this code in detail, including what it does and how it works"
inherit_flags = true

[commands.review]
system = "Review this code for bugs, security issues, and improvements"
provider = "anthropic"
model = "claude-3-opus"
```

Usage:
```bash
git diff | ask cm            # Generate commit message
cat main.rs | ask explain    # Explain code
cat api.py | ask review      # Code review
```

## Shell Completions

Generate shell completions for your preferred shell:

```bash
# Bash
ask --completions bash >> ~/.bashrc

# Zsh
ask --completions zsh >> ~/.zshrc

# Fish
ask --completions fish > ~/.config/fish/completions/ask.fish

# PowerShell
ask --completions powershell >> $PROFILE

# Elvish
ask --completions elvish >> ~/.elvish/rc.elv
```

## Auto-Update

The CLI automatically checks for updates in the background and notifies you on the next run when an update is available. To manually check and install updates:

```bash
ask --update
```

Set `ASK_NO_UPDATE=1` to disable automatic update checks.

## License

AGPL-3.0 - see [LICENSE](LICENSE)

## Contributing

Contributions are welcome! Please see the [CODEBASE.md](CODEBASE.md) for project structure and [ADR.md](ADR.md) for architectural decisions.
