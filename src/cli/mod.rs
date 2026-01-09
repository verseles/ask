//! CLI module - handles argument parsing and command execution

mod parser;

pub use parser::*;

use anyhow::Result;
use colored::Colorize;

use crate::config::Config;
use crate::context::ContextManager;
use crate::executor::CommandExecutor;
use crate::output::OutputFormatter;
use crate::providers::{
    build_unified_prompt, create_provider, expand_prompt_variables, load_custom_prompt,
    PromptContext, ProviderOptions,
};

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

    if args.make_prompt {
        println!("{}", crate::providers::DEFAULT_PROMPT_TEMPLATE);
        return Ok(());
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
    if args.has_context() {
        let manager = ContextManager::with_ttl(&config, args.context_ttl())?;

        if args.clear_context {
            manager.clear_current()?;
            println!("{}", "Context cleared.".green());
            return Ok(());
        }

        if args.show_history {
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
        println!("  ask -c60 follow up question    # 60 min context");
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

    let formatter = OutputFormatter::new(&args);

    handle_query(
        &config,
        &args,
        provider.as_ref(),
        &full_query,
        &formatter,
        custom_cmd.as_ref(),
    )
    .await?;

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

fn build_provider_options(args: &Args, config: &Config) -> ProviderOptions {
    let web_search = args.search || config.get_profile_web_search();
    let (allowed_domains, blocked_domains) = config.get_profile_domain_filters();

    ProviderOptions {
        web_search,
        allowed_domains,
        blocked_domains,
    }
}

async fn handle_query(
    config: &Config,
    args: &Args,
    provider: &dyn crate::providers::Provider,
    query: &str,
    formatter: &OutputFormatter,
    custom_cmd: Option<&crate::config::CustomCommand>,
) -> Result<()> {
    let mut messages = Vec::new();

    if args.has_context() {
        let manager = ContextManager::with_ttl(config, args.context_ttl())?;
        messages.extend(manager.get_messages()?);
        manager.print_echo_if_needed()?;
    }

    let ctx = PromptContext::from_env(args.command_mode, args.markdown, !args.no_color);

    let system_prompt = if let Some(cmd) = custom_cmd {
        if let Some(custom_prompt) = load_custom_prompt(Some(&cmd.system)) {
            expand_prompt_variables(&custom_prompt, &ctx)
        } else {
            format!(
                "{}\n\nContext: OS={}, shell={}, cwd={}, locale={}, now={}",
                cmd.system, ctx.os, ctx.shell, ctx.cwd, ctx.locale, ctx.now
            )
        }
    } else if let Some(custom_prompt) = load_custom_prompt(None) {
        let mut prompt = expand_prompt_variables(&custom_prompt, &ctx);
        if args.command_mode {
            prompt = format!("IMPORTANT: User explicitly requested command mode. Return ONLY the shell command, nothing else.\n\n{}", prompt);
        }
        prompt
    } else {
        build_unified_prompt(&ctx)
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

    let options = build_provider_options(args, config);

    if config.default.stream && !args.json && !args.raw {
        use std::sync::{Arc, Mutex};
        let full_response = Arc::new(Mutex::new(String::new()));
        let response_clone = full_response.clone();

        let callback: crate::providers::StreamCallback = Box::new(move |chunk: &str| {
            print!("{}", chunk);
            std::io::Write::flush(&mut std::io::stdout()).ok();
            response_clone.lock().unwrap().push_str(chunk);
        });

        provider
            .stream_with_options(&messages, callback, &options)
            .await?;

        println!();

        let response_text = full_response.lock().unwrap().clone();

        if args.has_context() {
            let manager = ContextManager::with_ttl(config, args.context_ttl())?;
            manager.add_message("user", query)?;
            manager.add_message("assistant", &response_text)?;
        }

        maybe_execute_command(config, args, &response_text).await?;
    } else {
        let response = provider.complete_with_options(&messages, &options).await?;

        formatter.format(&response.text);

        if args.citations && !response.citations.is_empty() {
            println!();
            println!("{}", "Sources:".cyan());
            for (i, cite) in response.citations.iter().enumerate() {
                println!("  [{}] {} - {}", i + 1, cite.title, cite.url);
            }
        }

        if args.has_context() {
            let manager = ContextManager::with_ttl(config, args.context_ttl())?;
            manager.add_message("user", query)?;
            manager.add_message("assistant", &response.text)?;
        }

        maybe_execute_command(config, args, &response.text).await?;
    }

    Ok(())
}

async fn maybe_execute_command(config: &Config, args: &Args, response: &str) -> Result<()> {
    let response = response.trim();

    let looks_like_command = is_likely_command(response);

    if !looks_like_command {
        return Ok(());
    }

    let executor = CommandExecutor::new(config);

    if args.yes || (config.behavior.auto_execute && executor.is_safe(response)) {
        println!();
        println!("{} {}", "Running:".green(), response.bright_white().bold());
        println!();
        executor
            .execute_with_sudo_retry(response, !args.no_follow)
            .await?;
    } else if args.command_mode && crate::executor::can_inject() {
        match crate::executor::inject_command(response)? {
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
    }

    Ok(())
}

fn is_likely_command(text: &str) -> bool {
    let text = text.trim();

    if text.is_empty() || text.contains('\n') {
        return false;
    }

    if text.len() > 500 {
        return false;
    }

    let first_word = text.split_whitespace().next().unwrap_or("");
    let command_starters = [
        "ls",
        "cd",
        "rm",
        "cp",
        "mv",
        "mkdir",
        "touch",
        "cat",
        "echo",
        "grep",
        "find",
        "chmod",
        "chown",
        "sudo",
        "apt",
        "yum",
        "brew",
        "npm",
        "yarn",
        "cargo",
        "git",
        "docker",
        "kubectl",
        "systemctl",
        "service",
        "curl",
        "wget",
        "tar",
        "zip",
        "unzip",
        "ssh",
        "scp",
        "rsync",
        "ps",
        "kill",
        "top",
        "htop",
        "df",
        "du",
        "free",
        "ping",
        "traceroute",
        "netstat",
        "ss",
        "iptables",
        "ufw",
        "python",
        "python3",
        "node",
        "ruby",
        "perl",
        "php",
        "java",
        "go",
        "rustc",
        "gcc",
        "g++",
        "make",
        "cmake",
        "./",
        "/",
        "~",
    ];

    command_starters
        .iter()
        .any(|cmd| first_word.starts_with(cmd))
}
