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

# All configuration lives in profiles
# First profile is used by default unless default_profile is set
# Switch profiles with: ask -p <profile_name>
[profiles.main]
provider = "gemini"           # gemini, openai, anthropic
model = "gemini-3-flash-preview"
api_key = "YOUR_API_KEY"
stream = true                 # Stream responses token by token
# thinking_level = "low"      # For Gemini 3: minimal, low, medium, high
# thinking_budget = 1024      # For Gemini 2.5: 0 (off), 1024-32768, -1 (dynamic)
# web_search = false          # Enable web search by default
# fallback = "none"           # Profile to use on errors: "any", "none", or profile name

# Example: Work profile with OpenAI
# [profiles.work]
# provider = "openai"
# model = "gpt-5"
# api_key = "sk-..."
# reasoning_effort = "medium" # For o1/o3/gpt-5: none, minimal, low, medium, high, xhigh
# fallback = "main"

# Example: Local profile with Ollama
# [profiles.local]
# provider = "openai"
# base_url = "http://localhost:11434/v1"
# model = "llama3"
# api_key = "ollama"          # Dummy key for local servers
# fallback = "none"

# Example: Research profile with Claude
# [profiles.research]
# provider = "anthropic"
# model = "claude-sonnet-4-20250514"
# thinking_budget = 16000     # For Claude: 0 (off), 1024-128000
# web_search = true

# Behavior settings (global)
[behavior]
auto_execute = false          # Auto-execute safe commands without prompting
confirm_destructive = true    # Confirm before running destructive commands
timeout = 30                  # Request timeout in seconds

# Context/history settings (global)
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
# profile = "research"        # Use specific profile for this command

# Command-line aliases - expand short aliases to full flags
# Usage: ask q how to list files -> ask --raw --no-color how to list files
[aliases]
# q = "--raw --no-color"
# fast = "-p fast --no-fallback"
# deep = "-t --search"
"##;
