//! CLI module - handles argument parsing and command execution

mod parser;

pub use parser::*;

use anyhow::Result;
use colored::Colorize;

use crate::config::Config;
use crate::context::ContextManager;
use crate::executor::CommandExecutor;
use crate::output::OutputFormatter;
use crate::providers::{create_provider, IntentClassifier, IntentType};

/// Main entry point for the CLI
pub async fn run() -> Result<()> {
    let args = Args::parse_flexible();

    // Handle special commands first
    if args.version {
        println!("ask {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    if args.update {
        println!("{}", "Checking for updates...".cyan());
        println!(
            "{}",
            "Auto-update not yet implemented. Please check GitHub releases.".yellow()
        );
        return Ok(());
    }

    // Load configuration
    let config = Config::load()?;
    let config = config.with_cli_overrides(&args);

    // Handle init command
    if args.init {
        return crate::config::init_config().await;
    }

    // Handle context commands
    if args.context {
        if args.clear_context {
            let manager = ContextManager::new(&config)?;
            manager.clear_current()?;
            println!("{}", "Context cleared.".green());
            return Ok(());
        }

        if args.show_history {
            let manager = ContextManager::new(&config)?;
            manager.show_history()?;
            return Ok(());
        }
    }

    // Check if we have a query
    if args.query.is_empty() {
        println!("{}", "Usage: ask [OPTIONS] <your question here>".cyan());
        println!();
        println!("Examples:");
        println!("  ask how to list docker containers");
        println!("  ask -x delete old log files");
        println!("  ask -c explain kubernetes");
        println!();
        println!("Run 'ask init' to configure your API keys.");
        println!("Run 'ask --help' for more options.");
        return Ok(());
    }

    // Get piped input if available
    let stdin_content = read_stdin_if_available();

    // Build the full query
    let full_query = if let Some(ref stdin) = stdin_content {
        format!(
            "Input:\n```\n{}\n```\n\nQuestion: {}",
            stdin,
            args.query.join(" ")
        )
    } else {
        args.query.join(" ")
    };

    // Create provider
    let provider = create_provider(&config)?;

    // Determine intent
    let intent = if args.command_mode {
        IntentType::Command
    } else {
        // Use classifier to determine intent
        let classifier = IntentClassifier::new(provider.as_ref());
        classifier
            .classify(&full_query)
            .await
            .unwrap_or(IntentType::Question)
    };

    // Create output formatter
    let formatter = OutputFormatter::new(&args);

    // Handle based on intent
    match intent {
        IntentType::Command => {
            handle_command_intent(&config, &args, provider.as_ref(), &full_query, &formatter)
                .await?;
        }
        IntentType::Question | IntentType::Code => {
            handle_question_intent(&config, &args, provider.as_ref(), &full_query, &formatter)
                .await?;
        }
    }

    Ok(())
}

fn read_stdin_if_available() -> Option<String> {
    use std::io::{self, IsTerminal, Read};

    if !io::stdin().is_terminal() {
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer).ok()?;
        if !buffer.is_empty() {
            return Some(buffer);
        }
    }
    None
}

/// Handle command generation intent
async fn handle_command_intent(
    config: &Config,
    args: &Args,
    provider: &dyn crate::providers::Provider,
    query: &str,
    _formatter: &OutputFormatter,
) -> Result<()> {
    use indicatif::{ProgressBar, ProgressStyle};

    // Show spinner while generating
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );
    spinner.set_message("Generating command...");
    spinner.enable_steady_tick(std::time::Duration::from_millis(100));

    // Build context messages
    let mut messages = Vec::new();

    // Add context if enabled
    if args.context {
        let manager = ContextManager::new(config)?;
        messages.extend(manager.get_messages()?);
    }

    // System prompt for command generation
    let system_prompt = r#"You are a command-line assistant. Generate shell commands for the user's request.

Rules:
1. Output ONLY the command, no explanations
2. Use standard Unix/Linux commands when possible
3. For complex tasks, chain commands with && or pipes
4. If multiple steps are needed, output them on separate lines
5. Never include markdown formatting or code blocks

Example outputs:
- "docker ps -a"
- "find . -name '*.log' -mtime +7 -delete"
- "git add . && git commit -m 'update'"
"#;

    messages.insert(
        0,
        crate::providers::Message {
            role: "system".to_string(),
            content: system_prompt.to_string(),
        },
    );

    messages.push(crate::providers::Message {
        role: "user".to_string(),
        content: query.to_string(),
    });

    // Generate command
    let response = provider.complete(&messages).await?;
    spinner.finish_and_clear();

    let command = response.trim();

    // Save to context if enabled
    if args.context {
        let manager = ContextManager::new(config)?;
        manager.add_message("user", query)?;
        manager.add_message("assistant", command)?;
    }

    // Display the command
    println!(
        "{} {}",
        "Generated command:".green(),
        command.bright_white().bold()
    );

    // Execute if auto-execute is enabled or -y flag
    let executor = CommandExecutor::new(config);

    if args.yes || (config.behavior.auto_execute && executor.is_safe(command)) {
        println!();
        executor.execute(command, !args.no_follow).await?;
    } else if executor.is_destructive(command) {
        println!(
            "{}",
            "This command may be destructive. Use -y to execute.".yellow()
        );
    } else {
        println!("{}", "Run with -y to execute automatically.".bright_black());
    }

    Ok(())
}

/// Handle question/code intent
async fn handle_question_intent(
    config: &Config,
    args: &Args,
    provider: &dyn crate::providers::Provider,
    query: &str,
    formatter: &OutputFormatter,
) -> Result<()> {
    // Build context messages
    let mut messages = Vec::new();

    // Add context if enabled
    if args.context {
        let manager = ContextManager::new(config)?;
        messages.extend(manager.get_messages()?);
    }

    // System prompt for questions
    let system_prompt = r#"You are a helpful AI assistant. Provide clear, concise answers.

Guidelines:
1. Be direct and informative
2. Use markdown formatting for better readability
3. Include code examples when relevant
4. Keep responses focused on the question
"#;

    messages.insert(
        0,
        crate::providers::Message {
            role: "system".to_string(),
            content: system_prompt.to_string(),
        },
    );

    messages.push(crate::providers::Message {
        role: "user".to_string(),
        content: query.to_string(),
    });

    // Stream response
    if config.default.stream && !args.json && !args.raw {
        use std::sync::{Arc, Mutex};
        let full_response = Arc::new(Mutex::new(String::new()));
        let response_clone = full_response.clone();

        let callback: crate::providers::StreamCallback = Box::new(move |chunk: &str| {
            print!("{}", chunk);
            std::io::Write::flush(&mut std::io::stdout()).ok();
            response_clone.lock().unwrap().push_str(chunk);
        });

        provider.stream(&messages, callback).await?;

        println!();

        // Save to context if enabled
        if args.context {
            let manager = ContextManager::new(config)?;
            let response_text = full_response.lock().unwrap().clone();
            manager.add_message("user", query)?;
            manager.add_message("assistant", &response_text)?;
        }
    } else {
        // Non-streaming response
        let response = provider.complete(&messages).await?;

        // Format and display
        formatter.format(&response);

        // Save to context if enabled
        if args.context {
            let manager = ContextManager::new(config)?;
            manager.add_message("user", query)?;
            manager.add_message("assistant", &response)?;
        }
    }

    Ok(())
}
