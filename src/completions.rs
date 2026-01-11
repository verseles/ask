//! Shell completions generation

use clap::{Arg, Command};
use clap_complete::{generate, Shell};
use std::io;

/// Build a clap Command for shell completions
/// This mirrors our custom parser's flags
fn build_cli() -> Command {
    Command::new("ask")
        .about("Ask anything in plain text, get commands or answers instantly")
        .version(env!("CARGO_PKG_VERSION"))
        .arg(
            Arg::new("context")
                .short('c')
                .long("context")
                .help("Use/create context for current directory")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("command")
                .short('x')
                .long("command")
                .help("Force command mode (bypass auto-detection)")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("yes")
                .short('y')
                .long("yes")
                .help("Auto-execute commands without confirmation")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("think")
                .short('t')
                .long("think")
                .help("Enable/disable thinking mode (--think or --think=false)")
                .num_args(0..=1)
                .default_missing_value("true")
                .value_name("BOOL"),
        )
        .arg(
            Arg::new("model")
                .short('m')
                .long("model")
                .help("Override configured model")
                .value_name("MODEL"),
        )
        .arg(
            Arg::new("profile")
                .short('p')
                .long("profile")
                .help("Use named profile from config")
                .value_name("NAME"),
        )
        .arg(
            Arg::new("provider")
                .short('P')
                .long("provider")
                .help("Override configured provider")
                .value_name("NAME"),
        )
        .arg(
            Arg::new("api-key")
                .short('k')
                .long("api-key")
                .help("API key (for use with init -n)")
                .value_name("KEY"),
        )
        .arg(
            Arg::new("non-interactive")
                .short('n')
                .long("non-interactive")
                .help("Non-interactive init (use with -P, -m, -k)")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .help("Output in JSON format")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("markdown")
                .long("markdown")
                .help("Output rendered in Markdown")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("raw")
                .long("raw")
                .help("Output raw text without formatting")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("no-color")
                .long("no-color")
                .help("Disable colorized output")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("no-follow")
                .long("no-follow")
                .help("Disable result echo after execution")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("no-fallback")
                .long("no-fallback")
                .help("Disable fallback to other profiles on error")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("search")
                .short('s')
                .long("search")
                .help("Enable web search for this query")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("citations")
                .long("citations")
                .help("Show citations from web search results")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("update")
                .long("update")
                .help("Check and install updates")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("make-prompt")
                .long("make-prompt")
                .help("Export default prompt template to stdout")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("make-config")
                .long("make-config")
                .help("Export example config.toml to stdout")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .help("Show verbose output (profile, provider info)")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("clear")
                .long("clear")
                .help("Clear current directory context (use with -c)")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("history")
                .long("history")
                .help("Show context history (use with -c)")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("completions")
                .long("completions")
                .help("Generate shell completions")
                .value_name("SHELL")
                .value_parser(["bash", "zsh", "fish", "powershell", "elvish"]),
        )
        .arg(
            Arg::new("query")
                .help("Your question or command")
                .num_args(0..)
                .trailing_var_arg(true),
        )
        .subcommand(Command::new("init").about("Initialize configuration interactively"))
        .subcommand(Command::new("config").about("Initialize configuration interactively"))
        .subcommand(Command::new("profiles").about("List available profiles"))
}

/// Generate shell completions and print to stdout
pub fn generate_completions(shell: &str) {
    let mut cmd = build_cli();

    let shell = match shell.to_lowercase().as_str() {
        "bash" => Shell::Bash,
        "zsh" => Shell::Zsh,
        "fish" => Shell::Fish,
        "powershell" | "pwsh" => Shell::PowerShell,
        "elvish" => Shell::Elvish,
        _ => {
            eprintln!(
                "Unknown shell: {}. Supported: bash, zsh, fish, powershell, elvish",
                shell
            );
            return;
        }
    };

    generate(shell, &mut cmd, "ask", &mut io::stdout());
}

/// Print installation instructions for completions
#[allow(dead_code)]
pub fn print_completion_instructions(shell: &str) {
    match shell.to_lowercase().as_str() {
        "bash" => {
            println!("# Add to ~/.bashrc:");
            println!("eval \"$(ask --completions bash)\"");
            println!();
            println!("# Or save to a file:");
            println!("ask --completions bash > ~/.local/share/bash-completion/completions/ask");
        }
        "zsh" => {
            println!("# Add to ~/.zshrc:");
            println!("eval \"$(ask --completions zsh)\"");
            println!();
            println!("# Or save to a file (ensure fpath includes the directory):");
            println!("ask --completions zsh > ~/.zsh/completions/_ask");
        }
        "fish" => {
            println!("# Save to fish completions directory:");
            println!("ask --completions fish > ~/.config/fish/completions/ask.fish");
        }
        "powershell" | "pwsh" => {
            println!("# Add to your PowerShell profile:");
            println!("ask --completions powershell | Out-String | Invoke-Expression");
        }
        "elvish" => {
            println!("# Add to ~/.elvish/rc.elv:");
            println!("eval (ask --completions elvish | slurp)");
        }
        _ => {
            println!("Supported shells: bash, zsh, fish, powershell, elvish");
        }
    }
}
