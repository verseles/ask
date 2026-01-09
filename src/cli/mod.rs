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

    // Handle internal --inject-raw command first (used by background injection)
    if let Some(ref cmd) = args.inject_raw {
        std::thread::sleep(std::time::Duration::from_millis(150));
        return crate::executor::inject_raw_only(cmd);
    }

    // Handle special commands first
    if args.version {
        println!("ask {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    if args.update {
        return crate::update::check_and_update().await;
    }

    // Handle completions generation
    if let Some(ref shell) = args.completions {
        crate::completions::generate_completions(shell);
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

    // Check for custom command (first word of query)
    let first_word = args.query.first().map(|s| s.as_str()).unwrap_or("");
    let custom_cmd = config.commands.get(first_word).cloned();

    // Build the full query
    let (full_query, effective_args) = if let Some(ref cmd) = custom_cmd {
        // Custom command: use remaining query as input
        let remaining: Vec<String> = args.query.iter().skip(1).cloned().collect();
        let query_text = if let Some(ref stdin) = stdin_content {
            format!("Input:\n```\n{}\n```\n\n{}", stdin, remaining.join(" "))
        } else {
            remaining.join(" ")
        };

        // Apply custom command overrides
        let mut modified_args = args.clone();
        if cmd.inherit_flags {
            // Keep existing flags
        }
        if let Some(auto_exec) = cmd.auto_execute {
            modified_args.yes = auto_exec;
        }
        if cmd.r#type.as_deref() == Some("command") {
            modified_args.command_mode = true;
        }

        (query_text, modified_args)
    } else {
        // Regular query
        let query_text = if let Some(ref stdin) = stdin_content {
            format!(
                "Input:\n```\n{}\n```\n\nQuestion: {}",
                stdin,
                args.query.join(" ")
            )
        } else {
            args.query.join(" ")
        };
        (query_text, args.clone())
    };
    let args = effective_args;

    // Create provider (with custom command overrides if applicable)
    let config = if let Some(ref cmd) = custom_cmd {
        let mut cfg = config;
        if let Some(ref provider) = cmd.provider {
            cfg.default.provider = provider.clone();
        }
        if let Some(ref model) = cmd.model {
            cfg.default.model = model.clone();
        }
        cfg
    } else {
        config
    };

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
            handle_command_intent(
                &config,
                &args,
                provider.as_ref(),
                &full_query,
                &formatter,
                custom_cmd.as_ref(),
            )
            .await?;
        }
        IntentType::Question | IntentType::Code => {
            handle_question_intent(
                &config,
                &args,
                provider.as_ref(),
                &full_query,
                &formatter,
                custom_cmd.as_ref(),
            )
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
    custom_cmd: Option<&crate::config::CustomCommand>,
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

    let os = std::env::consts::OS;
    let cwd = std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| ".".to_string());
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "sh".to_string());
    let locale = std::env::var("LANG").unwrap_or_else(|_| "en_US.UTF-8".to_string());
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M").to_string();

    // Use custom command system prompt if available, otherwise default
    let system_prompt = if let Some(cmd) = custom_cmd {
        format!(
            "{}\n\nContext: OS={}, shell={}, cwd={}, locale={}, now={}\n\nRules:\n- NEVER use newlines - use && for multiple commands or \\ for line continuation\n- No markdown, no code blocks, no backticks\n- Use commands appropriate for the OS",
            cmd.system, os, shell, cwd, locale, now
        )
    } else {
        format!(
            r#"Generate shell commands. Output ONLY the command, no explanations.

Context: OS={}, shell={}, cwd={}, locale={}, now={}

Rules:
- NEVER use newlines - use && for multiple commands or \ for line continuation
- No markdown, no code blocks, no backticks
- Use commands appropriate for the OS"#,
            os, shell, cwd, locale, now
        )
    };

    messages.insert(
        0,
        crate::providers::Message {
            role: "system".to_string(),
            content: system_prompt,
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

    let executor = CommandExecutor::new(config);

    if args.yes || (config.behavior.auto_execute && executor.is_safe(command)) {
        println!("{} {}", "Running:".green(), command.bright_white().bold());
        println!();
        executor
            .execute_with_sudo_retry(command, !args.no_follow)
            .await?;
    } else if crate::executor::can_inject() {
        match crate::executor::inject_command(command)? {
            None => {}
            Some(edited_cmd) => {
                println!(
                    "{} {}",
                    "Running:".green(),
                    edited_cmd.bright_white().bold()
                );
                println!();
                executor
                    .execute_with_sudo_retry(&edited_cmd, !args.no_follow)
                    .await?;
            }
        }
    } else {
        println!("{} {}", "Command:".green(), command.bright_white().bold());
        if executor.is_destructive(command) {
            println!(
                "{}",
                "This command may be destructive. Use -y to execute.".yellow()
            );
        } else {
            println!("{}", "Run with -y to execute automatically.".bright_black());
        }
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
    custom_cmd: Option<&crate::config::CustomCommand>,
) -> Result<()> {
    let mut messages = Vec::new();

    if args.context {
        let manager = ContextManager::new(config)?;
        messages.extend(manager.get_messages()?);
    }

    let locale = std::env::var("LANG").unwrap_or_else(|_| "en_US.UTF-8".to_string());
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M").to_string();

    // Use custom command system prompt if available
    let system_prompt = if let Some(cmd) = custom_cmd {
        format!("{}\n\nLocale: {}, Now: {}", cmd.system, locale, now)
    } else if args.markdown {
        format!(
            "Be brief and direct. Use markdown for formatting. Locale: {}, Now: {}",
            locale, now
        )
    } else {
        format!(
            "Be brief and direct. 1-3 sentences max. Plain text only, no formatting codes. Locale: {}, Now: {}",
            locale, now
        )
    };

    messages.insert(
        0,
        crate::providers::Message {
            role: "system".to_string(),
            content: system_prompt,
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
