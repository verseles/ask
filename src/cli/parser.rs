//! Flexible argument parser that allows flags before or after free text

use std::env;

#[derive(Debug, Clone, Default)]
pub struct Args {
    /// Use/create context for current directory
    pub context: bool,

    /// Force command mode (bypass auto-detection)
    pub command_mode: bool,

    /// Auto-execute commands without confirmation
    pub yes: bool,

    /// Override configured model
    pub model: Option<String>,

    /// Override configured provider
    pub provider: Option<String>,

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

    /// The actual query text (all non-flag arguments concatenated)
    pub query: Vec<String>,
}

impl Args {
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
                // Boolean flags (short)
                "-c" => result.context = true,
                "-x" => result.command_mode = true,
                "-y" => result.yes = true,
                "-t" => result.think = true,

                // Boolean flags (long)
                "--context" => result.context = true,
                "--command" => result.command_mode = true,
                "--yes" => result.yes = true,
                "--json" => result.json = true,
                "--markdown" => result.markdown = true,
                "--raw" => result.raw = true,
                "--no-color" => result.no_color = true,
                "--no-follow" => result.no_follow = true,
                "--think" => result.think = true,
                "--no-think" => result.no_think = true,
                "--update" => result.update = true,
                "--version" | "-V" => result.version = true,
                "--help" | "-h" => {
                    print_help();
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

                // Hidden internal flag for background injection
                "--inject-raw" => {
                    i += 1;
                    if i < args.len() {
                        result.inject_raw = Some(args[i].clone());
                    }
                }

                // Handle combined short flags like -cy
                arg if arg.starts_with('-') && !arg.starts_with("--") && arg.len() > 2 => {
                    for c in arg.chars().skip(1) {
                        match c {
                            'c' => result.context = true,
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

fn print_help() {
    println!(
        r#"ask - Ask anything in plain text, get commands or answers instantly

USAGE:
    ask [OPTIONS] <your question here>

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
    -h, --help            Show this help

SUBCOMMANDS:
    init                  Initialize configuration interactively
    --clear              Clear current directory context (use with -c)
    --history            Show context history (use with -c)

EXAMPLES:
    ask how to list docker containers
    ask -x delete old log files
    ask -c explain kubernetes
    ask -c what about pods?
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
        assert!(!args.context);
        assert!(!args.command_mode);
        assert!(args.query.is_empty());
    }
}
