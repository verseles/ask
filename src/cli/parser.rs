//! Flexible argument parser that allows flags before or after free text

use std::env;

#[derive(Debug, Clone, Default)]
pub struct Args {
    /// Use/create context for current directory (value = TTL in minutes, 0 = permanent)
    /// None = no context, Some(minutes) = use context with TTL
    pub context: Option<u64>,

    /// Force command mode (bypass auto-detection)
    /// None = auto-detect, Some(true) = -x/--command, Some(false) = --question
    pub command_mode: Option<bool>,

    /// Auto-execute commands without confirmation
    /// None = prompt, Some(true) = -y/--yes, Some(false) = --confirm
    pub yes: Option<bool>,

    /// Override configured model
    pub model: Option<String>,

    /// Override configured provider
    pub provider: Option<String>,

    /// Select named profile
    pub profile: Option<String>,

    /// Enable/disable thinking mode (--think or --think=true/false)
    /// None = use config default, Some(true) = enable, Some(false) = disable
    pub think: Option<bool>,

    /// Thinking level (minimal, low, medium, high, etc.)
    /// Used when --think=LEVEL or --think LEVEL is specified
    pub think_level: Option<String>,

    /// Output in JSON format
    pub json: bool,

    /// Output rendered in Markdown
    /// None = default, Some(true) = --markdown, Some(false) = --no-markdown
    pub markdown: Option<bool>,

    /// Output raw text without formatting
    pub raw: bool,

    /// Enable/disable colorized output
    /// None = default (enabled), Some(true) = --color, Some(false) = --no-color
    pub color: Option<bool>,

    /// Enable/disable result echo after execution
    /// None = default (enabled), Some(true) = --follow, Some(false) = --no-follow
    pub follow: Option<bool>,

    /// Enable/disable fallback to other profiles on error
    /// None = default (enabled), Some(true) = --fallback, Some(false) = --no-fallback
    pub fallback: Option<bool>,

    /// Enable/disable streaming responses
    /// None = use config, Some(true) = --stream, Some(false) = --no-stream
    pub stream: Option<bool>,

    /// Enable web search for this query
    /// None = use config, Some(true) = --search, Some(false) = --no-search
    pub search: Option<bool>,

    /// Show citations from web search results
    /// None = use config, Some(true) = --citations, Some(false) = --no-citations
    pub citations: Option<bool>,

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

    /// Show global history (across all directories)
    pub global: bool,

    /// INTERNAL: Inject command via uinput (hidden)
    pub inject_raw: Option<String>,

    /// Generate shell completions
    pub completions: Option<String>,

    /// Export default prompt template
    pub make_prompt: bool,

    /// Verbose mode - show profile and other debug info
    pub verbose: bool,

    /// List available profiles
    pub list_profiles: bool,

    /// Export example config template
    pub make_config: bool,

    /// Non-interactive mode for init (use with --provider, --model, --api-key)
    pub non_interactive: bool,

    /// API key for non-interactive init
    pub api_key: Option<String>,

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
        let raw_args: Vec<String> = env::args().skip(1).collect();
        let args = Self::expand_aliases(raw_args);
        Self::parse_args(args)
    }

    fn expand_aliases(args: Vec<String>) -> Vec<String> {
        let aliases = crate::config::Config::load_aliases_only();
        if aliases.is_empty() {
            return args;
        }

        let mut expanded = Vec::new();
        for arg in args {
            if let Some(expansion) = aliases.get(&arg) {
                for part in expansion.split_whitespace() {
                    expanded.push(part.to_string());
                }
            } else {
                expanded.push(arg);
            }
        }
        expanded
    }

    fn parse_args(args: Vec<String>) -> Self {
        let mut result = Args::default();
        let mut query_parts: Vec<String> = Vec::new();
        let mut i = 0;

        // Check environment variables
        if env::var("NO_COLOR").is_ok() {
            result.color = Some(false);
        }

        while i < args.len() {
            let arg = &args[i];

            match arg.as_str() {
                // Context flag with optional value
                "-c" => result.context = Some(30), // default 30 minutes
                "--context" => result.context = Some(30),

                // Boolean flags (short)
                "-x" => result.command_mode = Some(true),
                "-y" => result.yes = Some(true),
                "-t" => {
                    if i + 1 < args.len() && is_think_level(&args[i + 1]) {
                        i += 1;
                        result.think = Some(true);
                        result.think_level = Some(args[i].clone());
                    } else {
                        result.think = Some(true);
                    }
                }
                "-s" => result.search = Some(true),

                // Boolean flags (long)
                "--command" => result.command_mode = Some(true),
                "--question" => result.command_mode = Some(false),
                "--yes" => result.yes = Some(true),
                "--confirm" => result.yes = Some(false),
                "--json" => result.json = true,
                "--markdown" => result.markdown = Some(true),
                "--no-markdown" => result.markdown = Some(false),
                "--raw" => result.raw = true,
                "--no-color" | "--color=false" => result.color = Some(false),
                "--color" | "--color=true" => result.color = Some(true),
                "--no-follow" => result.follow = Some(false),
                "--follow" => result.follow = Some(true),
                "--no-fallback" => result.fallback = Some(false),
                "--fallback" => result.fallback = Some(true),
                "--stream" | "--stream=true" => result.stream = Some(true),
                "--stream=false" | "--no-stream" => result.stream = Some(false),
                "--search" | "--search=true" => result.search = Some(true),
                "--search=false" | "--no-search" => result.search = Some(false),
                "--citations" | "--citations=true" => result.citations = Some(true),
                "--citations=false" | "--no-citations" => result.citations = Some(false),
                "--think" => {
                    if i + 1 < args.len() && is_think_level(&args[i + 1]) {
                        i += 1;
                        result.think = Some(true);
                        result.think_level = Some(args[i].clone());
                    } else {
                        result.think = Some(true);
                    }
                }
                "--think=true" => result.think = Some(true),
                "--think=false" | "--no-think" => result.think = Some(false),
                s if s.starts_with("--think=") => {
                    let value = s.strip_prefix("--think=").unwrap();
                    if value == "0" {
                        result.think = Some(false);
                    } else if value == "1" {
                        result.think = Some(true);
                    } else {
                        result.think = Some(true);
                        result.think_level = Some(value.to_string());
                    }
                }
                "--update" => result.update = true,
                "--make-prompt" => result.make_prompt = true,
                "--make-config" => result.make_config = true,
                "--non-interactive" | "-n" => result.non_interactive = true,
                "-v" | "--verbose" => result.verbose = true,
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
                "init" | "config" if query_parts.is_empty() => result.init = true,
                "profiles" if query_parts.is_empty() => result.list_profiles = true,
                "history" if query_parts.is_empty() => result.show_history = true,
                "--clear" => result.clear_context = true,
                "--history" => result.show_history = true,
                "--global" => result.global = true,

                // Flags with values
                "-m" | "--model" => {
                    i += 1;
                    if i < args.len() {
                        result.model = Some(args[i].clone());
                    }
                }
                "-P" | "--provider" => {
                    i += 1;
                    if i < args.len() {
                        result.provider = Some(args[i].clone());
                    }
                }
                "-p" | "--profile" => {
                    i += 1;
                    if i < args.len() {
                        result.profile = Some(args[i].clone());
                    }
                }
                "-k" | "--api-key" => {
                    i += 1;
                    if i < args.len() {
                        result.api_key = Some(args[i].clone());
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

                // Handle --markdown=true|false format
                s if s.starts_with("--markdown=") => {
                    let value = s.strip_prefix("--markdown=").unwrap();
                    result.markdown = Some(value == "true" || value == "1");
                }

                arg if arg.starts_with('-') && !arg.starts_with("--") && arg.len() > 2 => {
                    let chars: Vec<char> = arg.chars().skip(1).collect();
                    let mut j = 0;
                    while j < chars.len() {
                        let c = chars[j];
                        let remaining: String = chars[j + 1..].iter().collect();

                        match c {
                            'c' => {
                                if remaining.is_empty() {
                                    result.context = Some(30);
                                    break;
                                } else if let Ok(minutes) = remaining.parse::<u64>() {
                                    result.context = Some(minutes);
                                    break; // POSIX: value consumes rest
                                } else {
                                    result.context = Some(30);
                                    break;
                                }
                            }
                            't' => {
                                match remaining.as_str() {
                                    s if s == "0" || s == "false" => {
                                        result.think = Some(false);
                                        break; // POSIX: value consumes rest
                                    }
                                    s if s == "1" || s == "true" => {
                                        result.think = Some(true);
                                        break;
                                    }
                                    s if s.starts_with('0') => {
                                        result.think = Some(false);
                                        break;
                                    }
                                    s if s.starts_with('1') => {
                                        result.think = Some(true);
                                        break;
                                    }
                                    s if is_think_level(s) => {
                                        result.think = Some(true);
                                        result.think_level = Some(s.to_string());
                                        break;
                                    }
                                    _ => result.think = Some(true),
                                }
                            }
                            's' => match remaining.as_str() {
                                "0" | "false" => {
                                    result.search = Some(false);
                                    break;
                                }
                                "1" | "true" => {
                                    result.search = Some(true);
                                    break;
                                }
                                s if s.starts_with('0') => {
                                    result.search = Some(false);
                                    break;
                                }
                                s if s.starts_with('1') => {
                                    result.search = Some(true);
                                    break;
                                }
                                _ => result.search = Some(true),
                            },
                            'x' => result.command_mode = Some(true),
                            'y' => result.yes = Some(true),
                            'v' => result.verbose = true,
                            'V' => result.version = true,
                            'h' => {
                                print_help();
                                std::process::exit(0);
                            }
                            _ => {}
                        }
                        j += 1;
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

PROFILE & PROVIDER SELECTION:
    ASK_PROFILE               Select profile (like -p), mutually exclusive with ASK_PROVIDER
    ASK_PROVIDER              Ad-hoc mode provider (like -P), mutually exclusive with ASK_PROFILE
    ASK_MODEL                 Override model name

API KEYS (used with ASK_PROVIDER or as fallback):
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
    # Use a specific profile
    export ASK_PROFILE=work

    # Ad-hoc mode with Gemini (no config file needed)
    export ASK_PROVIDER=gemini
    export ASK_GEMINI_API_KEY=AIza...

    # Use Ollama locally via OpenAI-compatible API
    export ASK_PROVIDER=openai
    export ASK_OPENAI_BASE_URL=http://localhost:11434/v1
    export ASK_OPENAI_API_KEY=ollama
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
        --question        Force question mode (bypass auto-detection)
    -y, --yes             Auto-execute commands without confirmation
        --confirm         Always prompt for confirmation (override -y/config)
    -t, --think[=LEVEL]   Enable thinking mode (levels: low, medium, high)
        --no-think        Disable thinking mode
    -m, --model <MODEL>   Override configured model
    -p, --profile <NAME>  Use named profile from config
    -P, --provider <NAME> Override configured provider
    -k, --api-key <KEY>   API key (for use with init -n)
    -n, --non-interactive Non-interactive init (use with -P, -m, -k)
        --stream          Enable streaming responses
        --no-stream       Disable streaming responses
    -s, --search          Enable web search for this query
        --no-search       Disable web search (override profile)
        --citations       Show citations from web search results
        --no-citations    Hide citations (override profile)
        --fallback        Enable fallback to other profiles (default)
        --no-fallback     Disable fallback to other profiles
        --follow          Enable result echo after execution (default)
        --no-follow       Disable result echo after execution
        --json            Output in JSON format
        --markdown        Enable markdown rendering
        --no-markdown     Disable markdown rendering
        --raw             Output raw text without formatting
        --color           Enable colorized output (default)
        --no-color        Disable colorized output
        --make-prompt     Export default prompt template to stdout
        --make-config     Export example ask.toml to stdout
        --help-env        Show all environment variables
        --update          Check and install updates
        --completions <SHELL>  Generate shell completions (bash, zsh, fish, powershell, elvish)
    -v, --verbose         Show verbose output (profile, provider info)
    -V, --version         Show version
    -h, --help            Show this help

SUBCOMMANDS:
    init, config          Initialize/manage configuration interactively
    profiles              List all available profiles
    --clear              Clear current directory context (use with -c)
    --history            Show context history (use with -c)

EXAMPLES:
    ask how to list docker containers
    ask -x delete old log files
    ask --question what is kubernetes     # force question mode
    ask -c explain kubernetes             # 30 min context (default)
    ask -c60 what about pods?             # 60 min context
    ask -c0 long conversation             # permanent context
    ask --context=120 complex topic       # 2 hour context
    ask -p work important query           # use work profile
    ask -s what happened today            # web search
    ask --no-stream explain quantum       # disable streaming
    git diff | ask cm
    cat main.rs | ask explain

CONFIGURATION:
    Run 'ask init' or 'ask config' to set up your API keys and preferences.
    Configuration files are loaded from:
      1. ./ask.toml or ./.ask.toml (project local)
      2. ~/ask.toml (home directory)
      3. ~/.config/ask/ask.toml (XDG config)

CUSTOM PROMPTS:
    Create ask.md in the config search path to customize the system prompt.
    Use 'ask --make-prompt > ask.md' to export the default template.

Run 'ask --help-env' for all environment variables.
"#
    );
}

fn is_think_level(s: &str) -> bool {
    let lower = s.to_lowercase();
    matches!(
        lower.as_str(),
        "none" | "minimal" | "low" | "medium" | "high" | "xhigh"
    ) || s.parse::<i64>().is_ok()
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
        assert!(args.command_mode.is_none());
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

    #[test]
    fn test_parse_think_with_level_equals() {
        let args = Args::parse_args(vec!["--think=minimal".into(), "hello".into()]);
        assert_eq!(args.think, Some(true));
        assert_eq!(args.think_level, Some("minimal".to_string()));
        assert_eq!(args.query, vec!["hello"]);
    }

    #[test]
    fn test_parse_think_with_level_space() {
        let args = Args::parse_args(vec!["--think".into(), "low".into(), "hello".into()]);
        assert_eq!(args.think, Some(true));
        assert_eq!(args.think_level, Some("low".to_string()));
        assert_eq!(args.query, vec!["hello"]);
    }

    #[test]
    fn test_parse_think_short_with_level() {
        let args = Args::parse_args(vec!["-t".into(), "medium".into(), "hello".into()]);
        assert_eq!(args.think, Some(true));
        assert_eq!(args.think_level, Some("medium".to_string()));
        assert_eq!(args.query, vec!["hello"]);
    }

    #[test]
    fn test_parse_think_numeric_level() {
        let args = Args::parse_args(vec!["--think=4096".into(), "hello".into()]);
        assert_eq!(args.think, Some(true));
        assert_eq!(args.think_level, Some("4096".to_string()));
    }

    #[test]
    fn test_parse_think_combined_tminimal() {
        let args = Args::parse_args(vec!["-tminimal".into(), "hello".into()]);
        assert_eq!(args.think, Some(true));
        assert_eq!(args.think_level, Some("minimal".to_string()));
        assert_eq!(args.query, vec!["hello"]);
    }

    #[test]
    fn test_parse_think_false_does_not_consume_next() {
        let args = Args::parse_args(vec!["--think=false".into(), "hello".into()]);
        assert_eq!(args.think, Some(false));
        assert_eq!(args.think_level, None);
        assert_eq!(args.query, vec!["hello"]);
    }

    #[test]
    fn test_is_think_level() {
        assert!(is_think_level("minimal"));
        assert!(is_think_level("low"));
        assert!(is_think_level("medium"));
        assert!(is_think_level("high"));
        assert!(is_think_level("xhigh"));
        assert!(is_think_level("none"));
        assert!(is_think_level("4096"));
        assert!(is_think_level("-1"));
        assert!(!is_think_level("hello"));
        assert!(!is_think_level("test"));
    }
}
