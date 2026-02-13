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
    build_unified_prompt, create_provider, expand_prompt_variables, flatten_command_if_safe,
    load_custom_prompt, PromptContext, ProviderOptions,
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
pub async fn run(update_notification: Option<crate::update::UpdateNotification>) -> Result<()> {
    let args = Args::parse_flexible();

    // Show update notification (unless JSON or raw mode)
    if let Some(ref notification) = update_notification {
        if !args.json && !args.raw {
            println!(
                "{} {} {} {}",
                "Updated:".green().bold(),
                notification.old_version.bright_black(),
                "→".bright_black(),
                notification.new_version.green()
            );
            if !notification.changelog.is_empty() {
                let changelog = crate::update::format_changelog(&notification.changelog, 10);
                for line in changelog.lines() {
                    println!("  {}", line.bright_black());
                }
            }
            println!();
        }
    }

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

    if args.profile.is_some() && args.provider.is_some() {
        anyhow::bail!(
            "Cannot use --profile (-p) and --provider (-P) together.\n\
             Use --profile to select a configured profile, or\n\
             Use --provider for ad-hoc mode (requires --api-key or ASK_{{PROVIDER}}_API_KEY)"
        );
    }

    let env_profile = std::env::var("ASK_PROFILE").ok();
    let env_provider = std::env::var("ASK_PROVIDER").ok();
    if env_profile.is_some() && env_provider.is_some() {
        anyhow::bail!(
            "Cannot use ASK_PROFILE and ASK_PROVIDER together.\n\
             Use ASK_PROFILE to select a configured profile, or\n\
             Use ASK_PROVIDER for ad-hoc mode"
        );
    }

    // Load configuration
    let config = Config::load()?;
    let config = config.with_cli_overrides(&args);

    // Handle init command
    if args.init {
        if args.non_interactive {
            return crate::config::init_config_non_interactive(
                args.provider.as_deref(),
                args.model.as_deref(),
                args.api_key.as_deref(),
            );
        }
        return crate::config::init_config().await;
    }

    if args.list_profiles {
        return list_profiles(&config);
    }

    if args.history_subcommand {
        return ContextManager::list_global(&config);
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

    let ad_hoc_provider = args
        .provider
        .clone()
        .or_else(|| std::env::var("ASK_PROVIDER").ok());
    if let Some(provider) = ad_hoc_provider {
        if config.active.api_key.is_none() {
            anyhow::bail!(
                "Ad-hoc mode requires an API key.\n\
                 Provide --api-key (-k) or set ASK_{}_API_KEY environment variable",
                provider.to_uppercase()
            );
        }
    }

    execute_with_fallback(&config, &args).await
}

async fn execute_with_fallback(config: &Config, args: &Args) -> Result<()> {
    // Get piped input if available
    let stdin_content = read_stdin_if_available();

    // Check for custom command (first word of query)
    let first_word = args.query.first().map(|s| s.as_str()).unwrap_or("");
    let mut custom_cmd = config.commands.get(first_word).cloned();
    if let Some(ref mut cmd) = custom_cmd {
        cmd.name = Some(first_word.to_string());
    }

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
            modified_args.yes = Some(auto_exec);
        }
        if cmd.r#type.as_deref() == Some("command") {
            modified_args.command_mode = Some(true);
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
            cfg.active.provider = provider.clone();
        }
        if let Some(ref model) = cmd.model {
            cfg.active.model = model.clone();
        }
        cfg
    } else {
        config.clone()
    };

    let active_profile = config.active_profile(&args);
    let result = try_query(&config, &args, &full_query, custom_cmd.as_ref()).await;

    match result {
        Ok(()) => Ok(()),
        Err(err) if args.fallback != Some(false) && is_retryable_error(&err) => {
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
                cfg.active.provider = provider.clone();
            }
            if let Some(ref model) = cmd.model {
                cfg.active.model = model.clone();
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
    let web_search = args
        .search
        .unwrap_or_else(|| config.get_profile_web_search());
    let (allowed_domains, blocked_domains) = config.get_profile_domain_filters();

    let (config_thinking_enabled, config_thinking_value) = config.get_thinking_config();

    let (thinking_enabled, thinking_value) = match args.think {
        Some(true) => (
            true,
            args.think_level
                .clone()
                .or(config_thinking_value)
                .or_else(|| Some("medium".to_string())),
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
        let options = build_provider_options(args, config);
        eprintln!(
            "{} provider={}, model={}, profile={}, thinking={}",
            "[verbose]".bright_black(),
            provider.name().cyan(),
            provider.model().cyan(),
            profile_name.as_deref().unwrap_or("default").cyan(),
            if options.thinking_enabled {
                options
                    .thinking_value
                    .as_deref()
                    .unwrap_or("on")
                    .to_string()
            } else {
                "off".to_string()
            }
            .cyan()
        );

        eprintln!(
            "{} flags: context={:?}, command_mode={:?}, yes={:?}, think={:?}, think_level={:?}, json={}, markdown={:?}, raw={}, color={:?}, follow={:?}, fallback={:?}, stream={:?}, search={:?}, citations={:?}, update={}, init={}, clear_context={}, show_history={}, make_prompt={}, make_config={}, list_profiles={}, non_interactive={}",
            "[verbose]".bright_black(),
            args.context,
            args.command_mode,
            args.yes,
            args.think,
            args.think_level,
            args.json,
            args.markdown,
            args.raw,
            args.color,
            args.follow,
            args.fallback,
            args.stream,
            args.search,
            args.citations,
            args.update,
            args.init,
            args.clear_context,
            args.show_history,
            args.make_prompt,
            args.make_config,
            args.list_profiles,
            args.non_interactive
        );
    }

    let mut messages = Vec::new();

    if args.has_context() {
        let manager = ContextManager::with_ttl(config, args.context_ttl())?;
        messages.extend(manager.get_messages()?);
        manager.print_echo_if_needed()?;
    }

    let ctx = PromptContext::from_env(
        args.command_mode.unwrap_or(false),
        args.markdown.unwrap_or(false),
        args.color.unwrap_or(true),
    );

    let system_prompt = if let Some(cmd) = custom_cmd {
        if let Some(custom_prompt) = load_custom_prompt(cmd.name.as_deref()) {
            expand_prompt_variables(&custom_prompt, &ctx)
        } else {
            format!(
                "{}\n\nContext: OS={}, shell={}, cwd={}, locale={}, now={}",
                cmd.system, ctx.os, ctx.shell, ctx.cwd, ctx.locale, ctx.now
            )
        }
    } else if let Some(custom_prompt) = load_custom_prompt(None) {
        let mut prompt = expand_prompt_variables(&custom_prompt, &ctx);
        if args.command_mode == Some(true) {
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

    // Determine if streaming should be enabled
    let should_stream = args.stream.unwrap_or(config.active.stream)
        && !args.json
        && !args.raw
        && !options.web_search;

    if should_stream {
        use crate::output::{Spinner, StreamingIndicator};
        use std::sync::{Arc, Mutex};

        let full_response = Arc::new(Mutex::new(String::new()));
        let response_clone = full_response.clone();

        // Start spinner while waiting for first chunk
        let spinner = Arc::new(Mutex::new(Some(Spinner::start())));
        let spinner_clone = spinner.clone();

        // Streaming indicator for showing ● at end of text
        let indicator = Arc::new(Mutex::new(StreamingIndicator::new()));
        let indicator_clone = indicator.clone();

        let callback: crate::providers::StreamCallback = Box::new(move |chunk: &str| {
            // Stop spinner on first chunk
            if let Some(mut s) = spinner_clone.lock().unwrap().take() {
                s.stop();
            }

            // Print chunk with indicator
            indicator_clone.lock().unwrap().print_chunk(chunk);
            response_clone.lock().unwrap().push_str(chunk);
        });

        provider
            .stream_with_options(&messages, callback, &options)
            .await?;

        // Finish indicator and add newline
        indicator.lock().unwrap().finish();
        println!();

        let response_text = full_response.lock().unwrap().clone();
        let response_text = if is_likely_command(&response_text) {
            flatten_command_if_safe(&response_text).unwrap_or(response_text)
        } else {
            response_text
        };

        // For sync injection (tmux/screen), clear the streamed command before injecting
        // For async injection (GUI paste), show a hint
        if crate::executor::can_inject() && is_likely_command(response_text.trim()) {
            if crate::executor::is_async_injection() {
                use colored::Colorize;
                println!("{}", "(disable streaming to hide this line)".bright_black());
            } else {
                // Sync injection: clear the command lines we just printed
                // Count lines in the response (including the newline we added)
                let line_count = response_text.lines().count() + 1;
                // Move cursor up and clear each line
                for _ in 0..line_count {
                    // Move up one line and clear it
                    print!("\x1b[A\x1b[2K");
                }
                use std::io::Write;
                std::io::stdout().flush().ok();
            }
        }

        if args.has_context() {
            let manager = ContextManager::with_ttl(config, args.context_ttl())?;
            manager.add_message("user", query)?;
            manager.add_message("assistant", &response_text)?;
        }

        maybe_execute_command(config, args, &response_text).await?;
    } else {
        use std::io::IsTerminal;

        // Show spinner while waiting for response (only in terminal, not raw/json)
        let use_spinner = !args.raw && !args.json && std::io::stdout().is_terminal();

        let spinner = if use_spinner {
            Some(crate::output::Spinner::start())
        } else {
            None
        };

        let response = provider.complete_with_options(&messages, &options).await?;
        let response_text = if is_likely_command(&response.text) {
            flatten_command_if_safe(&response.text).unwrap_or_else(|| response.text.clone())
        } else {
            response.text.clone()
        };

        // Stop spinner before output
        drop(spinner);

        // Skip echo if command will be injected into terminal
        let skip_echo = crate::executor::can_inject() && is_likely_command(response_text.trim());

        if !skip_echo {
            formatter.format(&response_text);
        }

        if args.citations == Some(true) && !response.citations.is_empty() {
            println!();
            println!("{}", "Sources:".cyan());
            for (i, cite) in response.citations.iter().enumerate() {
                println!("  [{}] {} - {}", i + 1, cite.title, cite.url);
            }
        }

        if args.has_context() {
            let manager = ContextManager::with_ttl(config, args.context_ttl())?;
            manager.add_message("user", query)?;
            manager.add_message("assistant", &response_text)?;
        }

        maybe_execute_command(config, args, &response_text).await?;
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

    if args.yes == Some(true) || (config.behavior.auto_execute && executor.is_safe(response)) {
        println!();
        println!("{} {}", "Running:".green(), response.bright_white().bold());
        println!();
        executor
            .execute_with_sudo_retry(response, args.follow != Some(false))
            .await?;
    } else if crate::executor::can_inject() {
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
                    .execute_with_sudo_retry(&edited_cmd, args.follow != Some(false))
                    .await?;
            }
        }
    }

    Ok(())
}

fn is_likely_command(text: &str) -> bool {
    let text = text.trim();

    if text.is_empty() {
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
    let effective_default = config.effective_default_profile();

    println!("{}", "Profiles".cyan().bold());
    println!();

    if config.profiles.is_empty() {
        println!(
            "  {}",
            "No profiles configured. Run 'ask init' to create one.".bright_black()
        );
        return Ok(());
    }

    let default_provider = "gemini".to_string();
    let default_model = "gemini-3-flash-preview".to_string();

    let mut profile_names: Vec<_> = config.profiles.keys().collect();
    profile_names.sort();

    for name in profile_names {
        let profile = &config.profiles[name];
        let is_default = effective_default.as_deref() == Some(name.as_str());
        let provider = profile.provider.as_ref().unwrap_or(&default_provider);
        let model = profile.model.as_ref().unwrap_or(&default_model);
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

    if let Some(default) = effective_default {
        println!();
        println!("Default profile: {}", default.green().bold());
    }

    println!();
    println!(
        "{}",
        "Use 'ask -p <profile>' to use a specific profile.".bright_black()
    );

    Ok(())
}
