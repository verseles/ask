# ask

> Ask anything in plain text, get commands or answers instantly. No quotes needed.

[![License: AGPL v3](https://img.shields.io/badge/License-AGPL%20v3-blue.svg)](LICENSE)

A CLI tool that lets you interact with AI models using natural language, without the need for quotes around your questions.

**Free, no signup, just install and use.** Comes with 4 built-in free AI profiles — no API key needed:

| Profile | Model | Best for |
|---------|-------|----------|
| `faster` | gpt-oss:20b | Fast answers with good quality **(default)** |
| `talker` | gpt-4o | Conversation & general knowledge |
| `coder` | codestral-latest | Code generation & analysis |
| `vision` | GLM-4.6V-Flash | Image/vision tasks |

Want more? Add your own profiles with OpenAI, Gemini, Claude, or any OpenAI-compatible API.

## Features

- **Free out of the box**: 4 built-in profiles, no signup or API key required
- **Natural input**: Just type `ask how to list docker containers` - no quotes needed
- **Flexible flags**: Put options before or after your question - both work!
- **Smart command injection**: Commands are safely flattened into one-liners when possible and pasted directly to your terminal
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
# Zero-config: works immediately with built-in free profiles
ask what is rust

# Use specific free profiles
ask -p coder write a fibonacci function
ask -p talker explain quantum computing
ask -p vision describe this image

# Optional: add your own API keys for premium models
ask init

# Non-interactive init (for scripts/automation)
ask init -n -p gemini -k YOUR_API_KEY

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
    -c, --context[=MIN]   Use context for current directory (default: 30 min, 0 = permanent)
                          Examples: -c (30 min), -c60 (1 hour), --context=0 (permanent)
    -x, --command         Force command mode (bypass auto-detection)
    -y, --yes             Auto-execute commands without confirmation
    -t, --think[=VAL]     Enable thinking mode with optional level (min/low/med/high)
                          Examples: -t, --think, --think=high, -tlow
    -m, --model <MODEL>   Override configured model
    -p, --profile <NAME>  Use named profile (e.g., -p work, --profile=local)
    -P, --provider <NAME> Override configured provider
    -k, --api-key <KEY>   API key (for use with init -n)
    -n, --non-interactive Non-interactive init (use with -P, -m, -k)
        --no-fallback     Disable profile fallback for this query
    -s, --search          Enable web search for this query
        --citations       Show citations from web search results
        --json            Output in JSON format
        --markdown[=bool] Output rendered in Markdown (--markdown or --markdown=true)
        --raw             Output raw text without formatting
        --no-color        Disable colorized output
        --color=bool      Enable/disable colorized output
        --no-follow       Disable result echo after execution
        --make-prompt     Export default prompt template
        --make-config     Export example ask.toml template
        --update          Check and install updates
        --help-env        Show all environment variables
    -v, --verbose         Show verbose output (profile, provider, model info, debug flags)
    -V, --version         Show version
    -h, --help            Show help

SUBCOMMANDS:
    init, config          Initialize/manage configuration interactively
    profiles              List all available profiles
    --clear               Clear current directory context (use with -c)
    --history             Show context history (use with -c)
```

## Configuration

Run `ask init` or `ask config` to configure interactively:

```
? What would you like to do?
› View current config
  Manage profiles
  Exit
```

<details>
<summary>Interactive Menu Features</summary>

**Main Menu:**
- **View current config** - Display all settings in formatted output
- **Manage profiles** - Create, edit, delete, set default profiles

**Profile Management:**
- Create new profiles with custom provider, model, API key, base URL
- Add free AI profiles (llm7.io + ch.at) in one click
- Edit existing profiles (provider, model, API key, thinking, web search, fallback)
- Delete profiles
- Set default profile

</details>

Configuration is loaded from multiple sources (in order of precedence):

1. CLI arguments
2. Environment variables (`ASK_*`)
3. `./ask.toml` or `./.ask.toml` (project local)
4. `~/ask.toml` (home directory - legacy, still supported)
5. `~/.config/ask/ask.toml` (XDG config - recommended)
6. Default values

4 built-in free profiles are always available (`talker`, `coder`, `vision`, `faster`), even when you have your own profiles configured. Select with `ask -p <name>`.

### Example ask.toml

```toml
# All configuration lives in profiles
# default_profile takes precedence; otherwise first non-built-in profile is used
# Switch profiles with: ask -p <profile_name>

# Optional: explicitly set default profile
# default_profile = "work"

[profiles.main]
provider = "gemini"
model = "gemini-3-flash-preview"
api_key = "YOUR_API_KEY_HERE"
stream = true
# thinking_level = "low"      # For Gemini 3: minimal, low, medium, high
# web_search = false          # Enable web search by default
# fallback = "none"           # Profile to use on errors: "any", "none", or profile name

# Example: Work profile with OpenAI
# [profiles.work]
# provider = "openai"
# model = "gpt-5"
# api_key = "sk-..."
# reasoning_effort = "medium" # For o1/o3/gpt-5: none, minimal, low, medium, high

# Example: Local profile with Ollama
# [profiles.local]
# provider = "openai"
# base_url = "http://localhost:11434/v1"
# model = "llama3"
# api_key = "ollama"          # Dummy key for local servers

[behavior]
auto_execute = false
confirm_destructive = true
timeout = 30

[context]
max_age_minutes = 30
max_messages = 20

# Command-line aliases
[aliases]
# q = "--raw --no-color"
# fast = "-p fast --no-fallback"
# deep = "-t --search"

# Custom commands
[commands.cm]
system = "Generate concise git commit message based on diff"
type = "command"
auto_execute = false
```

### Environment Variables

All configuration options can be set via environment variables. Run `ask --help-env` for the complete reference.

<details>
<summary>Click to expand full environment variables list</summary>

```bash
# Profile/Provider selection
ASK_PROFILE=main             # Select profile (like -p)
ASK_PROVIDER=gemini          # Ad-hoc mode (like -P), mutually exclusive with ASK_PROFILE
ASK_MODEL=gemini-3-flash     # Override model

# API Keys (used with ASK_PROVIDER or as fallback)
ASK_GEMINI_API_KEY=...           # Gemini API key
ASK_OPENAI_API_KEY=sk-...        # OpenAI API key
ASK_ANTHROPIC_API_KEY=sk-ant-... # Anthropic API key

# Custom base URLs (for proxies or compatible APIs)
ASK_GEMINI_BASE_URL=https://...
ASK_OPENAI_BASE_URL=https://...   # e.g., for Ollama: http://localhost:11434/v1
ASK_ANTHROPIC_BASE_URL=https://...

# Behavior settings
ASK_AUTO_EXECUTE=false           # Auto-execute safe commands
ASK_CONFIRM_DESTRUCTIVE=true     # Confirm destructive commands
ASK_TIMEOUT=30                   # Request timeout in seconds

# Context settings
ASK_CONTEXT_MAX_AGE=30           # Context TTL in minutes
ASK_CONTEXT_MAX_MESSAGES=20      # Max messages in context
ASK_CONTEXT_PATH=~/.local/share/ask/contexts  # Custom storage path

# Update settings
| ASK_UPDATE_AUTO_CHECK | true | Enable background update checks |
| ASK_UPDATE_INTERVAL | 24 | Hours between checks (min 1h in aggressive mode) |
| ASK_UPDATE_CHANNEL | stable | Update channel |
ASK_NO_UPDATE=1                  # Disable all update checks

# Other
NO_COLOR=1                       # Disable colors
```

</details>

## Web Search

Enable real-time web search to get current information beyond the LLM's training data:

```bash
# Enable web search for a single query
ask -s what happened in the news today

# Show citations from search results
ask --search --citations latest rust 1.85 features
```

<details>
<summary>Web Search Configuration</summary>

```toml
[profiles.research]
web_search = true

# Domain filtering (Anthropic only)
allowed_domains = ["docs.rs", "stackoverflow.com"]
blocked_domains = ["pinterest.com"]
```

**Provider Notes:**
- **Gemini**: Uses Google Search grounding
- **OpenAI**: Uses Responses API (only works with official API, not compatible endpoints)
- **Anthropic**: Uses `web_search_20250305` tool with optional domain filtering

</details>

## Profiles

Named profiles let you switch between different configurations quickly, like rclone:

```bash
# List all profiles
ask profiles

# Use work profile
ask -p work how to deploy to kubernetes

# Use local profile (Ollama)
ask --profile=local explain this error

# Ad-hoc mode: use provider without config (requires API key)
ask -P gemini -k YOUR_KEY what is rust

# Disable fallback for a single query
ask --no-fallback -p work critical query

# Verbose mode shows which profile is active
ask -v -p work what is kubernetes
```

<details>
<summary>Profile Configuration Examples</summary>

```toml
# Optional: explicitly set default profile (otherwise first profile is used)
default_profile = "work"

# Work profile with cloud provider
[profiles.work]
provider = "openai"
model = "gpt-5"
api_key = "sk-..."
fallback = "personal"  # retry with personal on errors

# Personal profile with different provider
[profiles.personal]
provider = "anthropic"
model = "claude-haiku-4-5"
api_key = "sk-ant-..."
fallback = "none"  # don't retry with another profile

# Local profile for Ollama/LM Studio
[profiles.local]
provider = "openai"
base_url = "http://localhost:11434/v1"
model = "llama3"
api_key = "ollama"  # dummy key for local servers
```

**Profile Resolution**: `default_profile` is used when set. Otherwise, `ask` prefers the first non-built-in profile; `faster` is used automatically when no custom profile exists.

**Fallback Options**:
- `fallback = "profile-name"` - Use specific profile on provider errors
- `fallback = "any"` - Try any available profile
- `fallback = "none"` - Disable fallback (fail immediately)

</details>

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
[profiles.local]
provider = "openai"
api_key = "ollama"
base_url = "http://localhost:11434/v1"
model = "llama3"
```

### Built-in Free Profiles

These profiles are always available, no signup required:

```toml
# ask -p faster (default when no custom profiles)
[profiles.faster]
provider = "openai"
base_url = "https://api.llm7.io/v1"
model = "gpt-oss:20b"

# ask -p talker
[profiles.talker]
provider = "openai"
base_url = "https://ch.at/v1"
model = "gpt-4o"

# ask -p coder
[profiles.coder]
provider = "openai"
base_url = "https://api.llm7.io/v1"
model = "codestral-latest"

# ask -p vision
[profiles.vision]
provider = "openai"
base_url = "https://api.llm7.io/v1"
model = "GLM-4.6V-Flash"
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
| Anthropic | `thinking_budget` | Token count or level (`low`=4k, `medium`=8k, `high`=16k) |

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

## Command-Line Aliases

Define short aliases for common flag combinations:

```toml
[aliases]
q = "--raw --no-color"
fast = "-p fast --no-fallback"
deep = "-t --search"
```

Usage:
```bash
ask q what is rust           # Expands to: ask --raw --no-color what is rust
ask deep explain quantum     # Expands to: ask -t --search explain quantum
```

## Custom Prompts

Customize the AI's behavior by creating `ask.md` files. These files completely replace the default system prompt.

```bash
# Export the default prompt template
ask --make-prompt > ask.md

# Edit ask.md to customize behavior
# The file will be used automatically
```

<details>
<summary>Custom Prompt Configuration</summary>

**Search Order** (first found wins):
1. Recursive search for `./ask.md` or `./.ask.md` (traverses up from current directory to root)
2. `~/ask.md` (home directory)
3. `~/.config/ask/ask.md` (XDG config)

**Command-Specific Prompts**:
- `ask.cm.md` - Custom prompt for the `cm` command (also searched recursively)
- `ask.explain.md` - Custom prompt for the `explain` command (also searched recursively)

**Available Variables**:
| Variable | Description |
|----------|-------------|
| `{os}` | Operating system (linux, macos, windows) |
| `{shell}` | User's shell (/bin/bash, /bin/zsh, etc.) |
| `{cwd}` | Current working directory |
| `{locale}` | User's locale (en_US.UTF-8, etc.) |
| `{now}` | Current date and time |
| `{format}` | Formatting instruction (markdown/colors/plain) |

**Example ask.md**:
```markdown
You are a senior developer assistant. Respond in {locale}.

When asked for commands:
- Use {shell} syntax appropriate for {os}
- Consider the current directory: {cwd}

Current time: {now}
{format}
```

</details>

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
