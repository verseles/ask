#![allow(dead_code)]

pub const DEFAULT_GEMINI_BASE_URL: &str = "https://generativelanguage.googleapis.com";
pub const DEFAULT_OPENAI_BASE_URL: &str = "https://api.openai.com/v1";
pub const DEFAULT_ANTHROPIC_BASE_URL: &str = "https://api.anthropic.com";
pub const DEFAULT_PROVIDER: &str = "gemini";
pub const DEFAULT_MODEL: &str = "gemini-flash-lite-latest";
pub const DEFAULT_OPENAI_MODEL: &str = "gpt-5-nano";
pub const DEFAULT_ANTHROPIC_MODEL: &str = "claude-haiku-4-5";
pub const DEFAULT_TIMEOUT: u64 = 30;

pub const DEFAULT_CONFIG_TEMPLATE: &str = r##"# ask - Configuration File
# Place this file at: ~/.config/ask/config.toml or ~/ask.toml

# Default settings
[default]
provider = "gemini"           # gemini, openai, anthropic
model = "gemini-3-flash-preview"
stream = true                 # Stream responses token by token

# Provider-specific settings
[providers.gemini]
api_key = "YOUR_GEMINI_API_KEY"
# base_url = "https://generativelanguage.googleapis.com"  # Optional custom endpoint
# thinking_level = "low"      # none, low, medium, high

[providers.openai]
api_key = "YOUR_OPENAI_API_KEY"
# base_url = "https://api.openai.com/v1"  # Or use http://localhost:11434/v1 for Ollama
# reasoning_effort = "low"    # none, minimal, low, medium, high (for o-series models)

[providers.anthropic]
api_key = "YOUR_ANTHROPIC_API_KEY"
# base_url = "https://api.anthropic.com"
# thinking_budget = 5000      # Token budget for extended thinking

# Behavior settings
[behavior]
auto_execute = false          # Auto-execute safe commands without prompting
confirm_destructive = true    # Confirm before running destructive commands
timeout = 30                  # Request timeout in seconds

# Context/history settings
[context]
max_age_minutes = 30          # Context TTL (0 = permanent)
max_messages = 20             # Maximum messages to keep
# storage_path = "~/.local/share/ask/contexts"  # Custom storage path

# Auto-update settings
[update]
auto_check = true             # Check for updates in background
aggressive = true             # Check every execution (not every 24h)
check_interval_hours = 24     # Hours between checks (when aggressive=false)
channel = "stable"            # stable, beta

# Named profiles - switch with: ask -P <profile_name>
# Profiles inherit from [default] and [providers.*] sections

# [profiles.work]
# provider = "openai"
# model = "gpt-5"
# api_key = "sk-work-key..."
# fallback = "personal"       # Retry with this profile on errors

# [profiles.local]
# provider = "openai"
# base_url = "http://localhost:11434/v1"  # Ollama
# model = "llama3"
# api_key = "ollama"          # Dummy key for local servers
# fallback = "none"           # Don't retry with another profile

# [profiles.research]
# provider = "anthropic"
# model = "claude-sonnet-4-20250514"
# web_search = true           # Enable web search for this profile
# thinking_budget = 10000

# Set default profile (optional)
# default_profile = "work"

# Custom commands - use with: ask <command_name> or pipe: git diff | ask cm
[commands.cm]
system = "Generate a concise git commit message based on the diff provided. Output ONLY the commit message, nothing else."
type = "command"
auto_execute = false

[commands.explain]
system = "Explain this code in detail, including what it does and how it works."
inherit_flags = true

# [commands.review]
# system = "Review this code for bugs, security issues, and improvements."
# provider = "anthropic"
# model = "claude-sonnet-4-20250514"

# Command-line aliases - expand short aliases to full flags
# Usage: ask q how to list files -> ask --raw --no-color how to list files
[aliases]
# q = "--raw --no-color"
# fast = "-P fast --no-fallback"
# deep = "-t --search"
"##;
