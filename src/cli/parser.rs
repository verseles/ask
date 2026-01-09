//! Flexible argument parser that allows flags before or after free text

use std::env;

#[derive(Debug, Clone, Default)]
pub struct Args {
    /// Use/create context for current directory (value = TTL in minutes, 0 = permanent)
    /// None = no context, Some(minutes) = use context with TTL
    pub context: Option<u64>,

    /// Force command mode (bypass auto-detection)
    pub command_mode: bool,

    /// Auto-execute commands without confirmation
    pub yes: bool,

    /// Override configured model
    pub model: Option<String>,

    /// Override configured provider
    pub provider: Option<String>,

    /// Select named profile
    pub profile: Option<String>,

    /// Enable thinking mode (override config)
    pub think: bool,

    /// Disable thinking mode (override config)
    pub no_think: bool,

    /// Output in JSON format
    pub json: bool,

    /// Output rendered in Markdown
    pub markdown: bool,

    /// Output raw text without formatting
    pub raw: bool,

    /// Disable colorized output
    pub no_color: bool,

    /// Disable result echo after execution
    pub no_follow: bool,

    /// Disable fallback to other profiles on error
    pub no_fallback: bool,

    /// Check and install updates
    pub update: bool,

    /// Show version
    pub version: bool,

    /// Initialize configuration
    pub init: bool,

    /// Clear current context
    pub clear_context: bool,

    /// Show context history
    pub show_history: bool,

    /// INTERNAL: Inject command via uinput (hidden)
    pub inject_raw: Option<String>,

    /// Generate shell completions
    pub completions: Option<String>,

    /// The actual query text (all non-flag arguments concatenated)
    pub query: Vec<String>,
}

impl Args {
    /// Check if context is enabled
    pub fn has_context(&self) -> bool {
        self.context.is_some()
    }

    /// Get context TTL in minutes (default 30)
    pub fn context_ttl(&self) -> u64 {
        self.context.unwrap_or(30)
    }

    /// Parse arguments flexibly, allowing flags before or after text
    pub fn parse_flexible() -> Self {
        let args: Vec<String> = env::args().skip(1).collect();
        let mut result = Args::default();
        let mut query_parts: Vec<String> = Vec::new();
        let mut i = 0;

        // Check environment variables
        if env::var("NO_COLOR").is_ok() {
            result.no_color = true;
        }

        while i < args.len() {
            let arg = &args[i];

            match arg.as_str() {
                // Context flag with optional value
                "-c" => result.context = Some(30), // default 30 minutes
                "--context" => result.context = Some(30),

                // Boolean flags (short)
                "-x" => result.command_mode = true,
                "-y" => result.yes = true,
                "-t" => result.think = true,

                // Boolean flags (long)
                "--command" => result.command_mode = true,
                "--yes" => result.yes = true,
                "--json" => result.json = true,
                "--markdown" => result.markdown = true,
                "--raw" => result.raw = true,
                "--no-color" => result.no_color = true,
                "--no-follow" => result.no_follow = true,
                "--no-fallback" => result.no_fallback = true,
                "--think" => result.think = true,
                "--no-think" => result.no_think = true,
                "--update" => result.update = true,
                "--version" | "-V" => result.version = true,
                "--help" | "-h" => {
                    print_help();
                    std::process::exit(0);
                }
                "--help-env" => {
                    print_help_env();
                    std::process::exit(0);
                }

                // Subcommands
                "init" if query_parts.is_empty() => result.init = true,
                "--clear" => result.clear_context = true,
                "--history" => result.show_history = true,

                // Flags with values
                "-m" | "--model" => {
                    i += 1;
                    if i < args.len() {
                        result.model = Some(args[i].clone());
                    }
                }
                "-p" | "--provider" => {
                    i += 1;
                    if i < args.len() {
                        result.provider = Some(args[i].clone());
                    }
                }
                "-P" | "--profile" => {
                    i += 1;
                    if i < args.len() {
                        result.profile = Some(args[i].clone());
                    }
                }

                // Hidden internal flag for background injection
                "--inject-raw" => {
                    i += 1;
                    if i < args.len() {
                        result.inject_raw = Some(args[i].clone());
                    }
                }

                // Generate shell completions
                "--completions" => {
                    i += 1;
                    if i < args.len() {
                        result.completions = Some(args[i].clone());
                    }
                }

                // Handle --context=N format
                s if s.starts_with("--context=") => {
                    let value = s.strip_prefix("--context=").unwrap();
                    result.context = Some(value.parse().unwrap_or(30));
                }

                // Handle -c=N format
                s if s.starts_with("-c=") => {
                    let value = s.strip_prefix("-c=").unwrap();
                    result.context = Some(value.parse().unwrap_or(30));
                }

                // Handle --profile=NAME format
                s if s.starts_with("--profile=") => {
                    let value = s.strip_prefix("--profile=").unwrap();
                    result.profile = Some(value.to_string());
                }

                // Handle combined short flags like -cy or -c60
                arg if arg.starts_with('-') && !arg.starts_with("--") && arg.len() > 2 => {
                    let chars: Vec<char> = arg.chars().skip(1).collect();

                    // Check if it's -c followed by a number (like -c60)
                    if chars.first() == Some(&'c') {
                        let rest: String = chars[1..].iter().collect();
                        if let Ok(minutes) = rest.parse::<u64>() {
                            result.context = Some(minutes);
                        } else {
                            // It's combined flags like -cy
                            for c in chars {
                                match c {
                                    'c' => result.context = Some(30),
                                    'x' => result.command_mode = true,
                                    'y' => result.yes = true,
                                    't' => result.think = true,
                                    'V' => result.version = true,
                                    'h' => {
                                        print_help();
                                        std::process::exit(0);
                                    }
                                    _ => {}
                                }
                            }
                        }
                    } else {
                        // Regular combined flags like -xy
                        for c in chars {
                            match c {
                                'c' => result.context = Some(30),
                                'x' => result.command_mode = true,
                                'y' => result.yes = true,
                                't' => result.think = true,
                                'V' => result.version = true,
                                'h' => {
                                    print_help();
                                    std::process::exit(0);
                                }
                                _ => {}
                            }
                        }
                    }
                }

                // Unknown flags - treat as part of query if they look like words
                s if s.starts_with('-') => {
                    // Could be a typo or intentional word like "-1" or "--verbose"
                    // For now, add to query
                    query_parts.push(s.to_string());
                }

                // Regular text - add to query
                _ => {
                    query_parts.push(args[i].clone());
                }
            }

            i += 1;
        }

        result.query = query_parts;
        result
    }
}

fn print_help_env() {
    println!(
        r#"ask - Environment Variables Reference

All configuration options can be set via environment variables with the ASK_ prefix.
These override config file values but are overridden by CLI arguments.

PROVIDER & MODEL:
    ASK_PROVIDER              Default provider (gemini, openai, anthropic)
    ASK_MODEL                 Default model name
    ASK_STREAM                Enable streaming (true/false, 1/0)

API KEYS:
    ASK_GEMINI_API_KEY        Gemini API key
    ASK_OPENAI_API_KEY        OpenAI API key
    ASK_ANTHROPIC_API_KEY     Anthropic API key

CUSTOM BASE URLS (for proxies or OpenAI-compatible APIs like Ollama):
    ASK_GEMINI_BASE_URL       Custom Gemini API endpoint
    ASK_OPENAI_BASE_URL       Custom OpenAI API endpoint (e.g., http://localhost:11434/v1)
    ASK_ANTHROPIC_BASE_URL    Custom Anthropic API endpoint

BEHAVIOR:
    ASK_AUTO_EXECUTE          Auto-execute safe commands without prompting (true/false)
    ASK_CONFIRM_DESTRUCTIVE   Confirm before running destructive commands (true/false)
    ASK_TIMEOUT               Request timeout in seconds (default: 30)

CONTEXT SETTINGS:
    ASK_CONTEXT_MAX_AGE       Context TTL in minutes (default: 30)
    ASK_CONTEXT_MAX_MESSAGES  Maximum messages to keep in context (default: 20)
    ASK_CONTEXT_PATH          Custom path for context storage

UPDATE SETTINGS:
    ASK_UPDATE_AUTO_CHECK     Enable background update checks (true/false)
    ASK_UPDATE_INTERVAL       Hours between update checks (default: 24)
    ASK_UPDATE_CHANNEL        Update channel (stable, beta, etc.)
    ASK_NO_UPDATE             Disable all update functionality (set to 1)

DISPLAY:
    NO_COLOR                  Disable colored output (standard env var)

EXAMPLES:
    # Set default provider and model
    export ASK_PROVIDER=anthropic
    export ASK_MODEL=claude-3-5-haiku

    # Use Ollama locally via OpenAI-compatible API
    export ASK_OPENAI_BASE_URL=http://localhost:11434/v1
    export ASK_OPENAI_API_KEY=ollama
    export ASK_PROVIDER=openai
    export ASK_MODEL=llama3

    # Disable update checks
    export ASK_NO_UPDATE=1
"#
    );
}

fn print_help() {
    println!(
        r#"ask - Ask anything in plain text, get commands or answers instantly

USAGE:
    ask [OPTIONS] <your question here>

OPTIONS:
    -c, --context[=MIN]   Use context for current directory (default: 30 min, 0 = permanent)
                          Examples: -c (30 min), -c60 (60 min), --context=120 (2 hours)
    -x, --command         Force command mode (bypass auto-detection)
    -y, --yes             Auto-execute commands without confirmation
    -t, --think           Enable thinking mode (override config)
        --no-think        Disable thinking mode (override config)
    -m, --model <MODEL>   Override configured model
    -p, --provider <NAME> Override configured provider
    -P, --profile <NAME>  Use named profile from config
        --json            Output in JSON format
        --markdown        Output rendered in Markdown
        --raw             Output raw text without formatting
        --no-color        Disable colorized output
        --no-follow       Disable result echo after execution
        --no-fallback     Disable fallback to other profiles on error
        --update          Check and install updates
        --completions <SHELL>  Generate shell completions (bash, zsh, fish, powershell, elvish)
    -V, --version         Show version
    -h, --help            Show this help

SUBCOMMANDS:
    init                  Initialize configuration interactively
    --clear              Clear current directory context (use with -c)
    --history            Show context history (use with -c)

EXAMPLES:
    ask how to list docker containers
    ask -x delete old log files
    ask -c explain kubernetes         # 30 min context (default)
    ask -c60 what about pods?         # 60 min context
    ask -c0 long conversation         # permanent context
    ask --context=120 complex topic   # 2 hour context
    git diff | ask cm
    cat main.rs | ask explain

CONFIGURATION:
    Run 'ask init' to set up your API keys and preferences.
    Configuration files are loaded from:
      1. ./ask.toml or ./.ask.toml (project local)
      2. ~/ask.toml (home directory)
      3. ~/.config/ask/config.toml (XDG config)

ENVIRONMENT VARIABLES:
    ASK_PROVIDER          Override default provider
    ASK_MODEL             Override default model
    ASK_GEMINI_API_KEY    Gemini API key
    ASK_OPENAI_API_KEY    OpenAI API key
    ASK_ANTHROPIC_API_KEY Anthropic API key
    NO_COLOR              Disable colored output
"#
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_query() {
        // This test would need to mock env::args
        // For now, just test the default
        let args = Args::default();
        assert!(args.context.is_none());
        assert!(!args.has_context());
        assert!(!args.command_mode);
        assert!(args.query.is_empty());
    }

    #[test]
    fn test_context_ttl_default() {
        let args = Args {
            context: Some(30),
            ..Default::default()
        };
        assert!(args.has_context());
        assert_eq!(args.context_ttl(), 30);
    }

    #[test]
    fn test_context_ttl_custom() {
        let args = Args {
            context: Some(60),
            ..Default::default()
        };
        assert_eq!(args.context_ttl(), 60);
    }

    #[test]
    fn test_context_permanent() {
        let args = Args {
            context: Some(0),
            ..Default::default()
        };
        assert!(args.has_context());
        assert_eq!(args.context_ttl(), 0);
    }
}
