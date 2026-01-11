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

/// Check if an error is retryable with a fallback profile
fn is_retryable_error(err: &anyhow::Error) -> bool {
    let msg = err.to_string().to_lowercase();
    msg.contains("429")
        || msg.contains("500")
        || msg.contains("502")
        || msg.contains("503")
        || msg.contains("504")
        || msg.contains("rate limit")
        || msg.contains("too many requests")
        || msg.contains("timeout")
        || msg.contains("timed out")
        || msg.contains("connection refused")
        || msg.contains("connection reset")
        || msg.contains("network unreachable")
        || msg.contains("service unavailable")
}

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

    if args.make_config {
        println!("{}", crate::config::DEFAULT_CONFIG_TEMPLATE);
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

    if args.list_profiles {
        return list_profiles(&config);
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

    execute_with_fallback(&config, &args).await
}

async fn execute_with_fallback(config: &Config, args: &Args) -> Result<()> {
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
        let mut cfg = config.clone();
        if let Some(ref provider) = cmd.provider {
            cfg.default.provider = provider.clone();
        }
        if let Some(ref model) = cmd.model {
            cfg.default.model = model.clone();
        }
        cfg
    } else {
        config.clone()
    };

    let active_profile = config.active_profile(&args);
    let result = try_query(&config, &args, &full_query, custom_cmd.as_ref()).await;

    match result {
        Ok(()) => Ok(()),
        Err(err) if !args.no_fallback && is_retryable_error(&err) => {
            if let Some(ref profile_name) = active_profile {
                try_with_fallback(
                    &config,
                    &args,
                    &full_query,
                    custom_cmd.as_ref(),
                    profile_name,
                    &err,
                )
                .await
            } else {
                Err(err)
            }
        }
        Err(err) => Err(err),
    }
}

async fn try_with_fallback(
    _config: &Config,
    args: &Args,
    query: &str,
    custom_cmd: Option<&crate::config::CustomCommand>,
    current_profile: &str,
    original_err: &anyhow::Error,
) -> Result<()> {
    let mut tried_profiles = vec![current_profile.to_string()];
    let mut current = current_profile.to_string();
    let original_config = Config::load()?;

    while let Some(fallback_name) = original_config.fallback_profile(&current) {
        if tried_profiles.contains(&fallback_name) {
            break;
        }

        eprintln!(
            "{} {}",
            "Provider error, retrying with fallback profile:".yellow(),
            fallback_name.bright_white()
        );

        let mut fallback_args = args.clone();
        fallback_args.profile = Some(fallback_name.clone());
        let fallback_config = original_config.clone().with_cli_overrides(&fallback_args);

        let fallback_config = if let Some(cmd) = custom_cmd {
            let mut cfg = fallback_config;
            if let Some(ref provider) = cmd.provider {
                cfg.default.provider = provider.clone();
            }
            if let Some(ref model) = cmd.model {
                cfg.default.model = model.clone();
            }
            cfg
        } else {
            fallback_config
        };

        match try_query(&fallback_config, &fallback_args, query, custom_cmd).await {
            Ok(()) => return Ok(()),
            Err(err) if is_retryable_error(&err) => {
                tried_profiles.push(fallback_name.clone());
                current = fallback_name;
                continue;
            }
            Err(err) => return Err(err),
        }
    }

    Err(anyhow::anyhow!("{}", original_err))
}

async fn try_query(
    config: &Config,
    args: &Args,
    query: &str,
    custom_cmd: Option<&crate::config::CustomCommand>,
) -> Result<()> {
    let provider = create_provider(config)?;
    let formatter = OutputFormatter::new(args);

    handle_query(
        config,
        args,
        provider.as_ref(),
        query,
        &formatter,
        custom_cmd,
    )
    .await
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

    let (config_thinking_enabled, config_thinking_value) = config.get_thinking_config();

    let (thinking_enabled, thinking_value) = match args.think {
        Some(true) => (
            true,
            config_thinking_value.or_else(|| Some("medium".to_string())),
        ),
        Some(false) => (false, None),
        None => (config_thinking_enabled, config_thinking_value),
    };

    ProviderOptions {
        web_search,
        allowed_domains,
        blocked_domains,
        thinking_enabled,
        thinking_value,
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
    if args.verbose {
        let profile_name = config.active_profile(args);
        let (thinking_enabled, thinking_value) = config.get_thinking_config();
        eprintln!(
            "{} provider={}, model={}, profile={}, thinking={}",
            "[verbose]".bright_black(),
            provider.name().cyan(),
            provider.model().cyan(),
            profile_name.as_deref().unwrap_or("default").cyan(),
            if thinking_enabled {
                thinking_value.as_deref().unwrap_or("on").to_string()
            } else {
                "off".to_string()
            }
            .cyan()
        );
    }

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

fn list_profiles(config: &Config) -> Result<()> {
    let default_name = config.default_profile.as_deref();

    println!("{}", "Profiles".cyan().bold());
    println!();

    if config.profiles.is_empty() {
        println!(
            "  {} {} {}",
            "(default)".bright_black(),
            config.default.provider.bright_white(),
            config.default.model.bright_black()
        );
        println!();
        println!(
            "{}",
            "No custom profiles configured. Run 'ask init' to create one.".bright_black()
        );
        return Ok(());
    }

    let mut profile_names: Vec<_> = config.profiles.keys().collect();
    profile_names.sort();

    for name in profile_names {
        let profile = &config.profiles[name];
        let is_default = default_name == Some(name.as_str());
        let provider = profile
            .provider
            .as_ref()
            .unwrap_or(&config.default.provider);
        let model = profile.model.as_ref().unwrap_or(&config.default.model);
        let fallback = profile
            .fallback
            .as_ref()
            .map(|f| format!(" [fallback: {}]", f))
            .unwrap_or_default();
        let thinking = match provider.as_str() {
            "gemini" => profile
                .thinking_level
                .as_ref()
                .map(|v| format!("think:{}", v)),
            "openai" => profile
                .reasoning_effort
                .as_ref()
                .map(|v| format!("reason:{}", v)),
            "anthropic" => profile.thinking_budget.map(|v| format!("budget:{}", v)),
            _ => None,
        }
        .map(|s| format!(" [{}]", s))
        .unwrap_or_default();
        let web_search = if profile.web_search == Some(true) {
            " [search]"
        } else {
            ""
        };

        if is_default {
            println!(
                "  {} {} {}{}{}{}",
                name.green().bold(),
                provider.bright_white(),
                model.bright_black(),
                fallback.bright_black(),
                thinking.bright_black(),
                web_search.cyan()
            );
        } else {
            println!(
                "  {} {} {}{}{}{}",
                name.white(),
                provider.bright_white(),
                model.bright_black(),
                fallback.bright_black(),
                thinking.bright_black(),
                web_search.cyan()
            );
        }
    }

    if let Some(default) = default_name {
        println!();
        println!("Default profile: {}", default.green().bold());
    }

    println!();
    println!(
        "{}",
        "Use 'ask -P <profile>' to use a specific profile.".bright_black()
    );

    Ok(())
}
