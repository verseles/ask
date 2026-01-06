# ask

> Ask anything in plain text, get commands or answers instantly. No quotes needed.

[![License: AGPL v3](https://img.shields.io/badge/License-AGPL%20v3-blue.svg)](LICENSE)

A CLI tool that lets you interact with AI models using natural language, without the need for quotes around your questions.

## Features

- **Natural input**: Just type `ask how to list docker containers` - no quotes needed
- **Flexible flags**: Put options before or after your question - both work!
- **Smart intent detection**: Automatically detects if you want a command or an answer
- **Multiple providers**: Supports Gemini (default), OpenAI, and Anthropic Claude
- **Streaming responses**: Real-time token-by-token output
- **Thinking mode**: Enable AI reasoning for complex tasks (`-t` flag or config)
- **Context awareness**: Optional conversation memory per directory
- **Safe command execution**: Detects and warns about destructive commands
- **Flexible configuration**: TOML config with environment variable overrides
- **Piping support**: Works with `git diff | ask cm` style workflows

## Installation

### Unix/Linux/macOS

```bash
curl -fsSL https://raw.githubusercontent.com/verseles/ask/main/install.sh | sh
```

The installer will prompt you to configure your API keys automatically.

### Windows

```powershell
irm https://raw.githubusercontent.com/verseles/ask/main/install.ps1 | iex
```

### From source

```bash
cargo install --git https://github.com/verseles/ask
```

## Quick Start

```bash
# Initialize configuration (set up API keys)
ask init

# Ask questions naturally
ask how to list docker containers
ask what is the capital of France

# Flags can go before OR after your question
ask -x delete old log files
ask delete old log files -x

# Enable thinking mode for complex reasoning
ask -t explain the theory of relativity
ask solve this math problem step by step --think

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

## License

AGPL-3.0 - see [LICENSE](LICENSE)

## Contributing

Contributions are welcome! Please see the [CODEBASE.md](CODEBASE.md) for project structure and [ADR.md](ADR.md) for architectural decisions.
