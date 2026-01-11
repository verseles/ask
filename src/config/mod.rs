//! Configuration module - handles loading and merging configs

mod defaults;
mod loader;
mod thinking;

pub use defaults::*;
pub use thinking::{format_thinking_config, select_thinking_config};

use crate::cli::Args;
use anyhow::Result;
use colored::Colorize;
use dialoguer::{Confirm, Input, Select};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub default: DefaultConfig,

    #[serde(default)]
    pub providers: HashMap<String, ProviderConfig>,

    #[serde(default)]
    pub behavior: BehaviorConfig,

    #[serde(default)]
    pub context: ContextConfig,

    #[serde(default)]
    pub update: UpdateConfig,

    #[serde(default)]
    pub commands: HashMap<String, CustomCommand>,

    /// Named profiles for different configurations
    #[serde(default)]
    pub profiles: HashMap<String, ProfileConfig>,

    /// Default profile name (if not set, uses first profile or base config)
    #[serde(default)]
    pub default_profile: Option<String>,

    /// Command-line aliases (e.g., "q" = "--raw --no-color")
    #[serde(default)]
    pub aliases: HashMap<String, String>,
}

/// Default provider and model settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultConfig {
    #[serde(default = "default_provider")]
    pub provider: String,

    #[serde(default = "default_model")]
    pub model: String,

    #[serde(default = "default_true")]
    pub stream: bool,
}

/// Provider-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProviderConfig {
    pub api_key: Option<String>,

    #[serde(default)]
    pub base_url: Option<String>,

    #[serde(default)]
    pub model: Option<String>,
}

/// Behavior settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorConfig {
    #[serde(default)]
    pub auto_execute: bool,

    #[serde(default = "default_true")]
    pub confirm_destructive: bool,

    #[serde(default = "default_timeout")]
    pub timeout: u64,
}

/// Context/history settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextConfig {
    #[serde(default = "default_max_age")]
    pub max_age_minutes: u64,

    #[serde(default = "default_max_messages")]
    pub max_messages: usize,

    #[serde(default)]
    pub storage_path: Option<String>,
}

/// Auto-update settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateConfig {
    #[serde(default = "default_true")]
    pub auto_check: bool,

    #[serde(default = "default_check_interval")]
    pub check_interval_hours: u64,

    #[serde(default = "default_channel")]
    pub channel: String,

    #[serde(default = "default_true")]
    pub aggressive: bool,
}

/// Custom command definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomCommand {
    pub system: String,

    #[serde(default)]
    pub r#type: Option<String>,

    #[serde(default = "default_true")]
    pub inherit_flags: bool,

    #[serde(default)]
    pub auto_execute: Option<bool>,

    #[serde(default)]
    pub provider: Option<String>,

    #[serde(default)]
    pub model: Option<String>,
}

/// Named profile configuration - all settings for a profile
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProfileConfig {
    /// Provider name (gemini, openai, anthropic)
    #[serde(default)]
    pub provider: Option<String>,

    /// Model name
    #[serde(default)]
    pub model: Option<String>,

    /// API key for this profile
    #[serde(default)]
    pub api_key: Option<String>,

    /// Base URL (for OpenAI-compatible endpoints like Ollama)
    #[serde(default)]
    pub base_url: Option<String>,

    /// Enable streaming responses
    #[serde(default)]
    pub stream: Option<bool>,

    /// Fallback profile name ("none" to disable, "any" for first available)
    #[serde(default)]
    pub fallback: Option<String>,

    /// Thinking level for Gemini 3 (minimal, low, medium, high)
    #[serde(default)]
    pub thinking_level: Option<String>,

    /// Thinking budget for Gemini 2.5 (0, 1024-32768, -1 for dynamic)
    #[serde(default)]
    pub thinking_budget: Option<i64>,

    /// Reasoning effort for OpenAI (none, minimal, low, medium, high, xhigh)
    #[serde(default)]
    pub reasoning_effort: Option<String>,

    /// Enable web search for this profile
    #[serde(default)]
    pub web_search: Option<bool>,

    /// Show citations from web search results
    #[serde(default)]
    pub show_citations: Option<bool>,

    /// Allowed domains for web search (Anthropic only)
    #[serde(default)]
    pub allowed_domains: Option<Vec<String>>,

    /// Blocked domains for web search (Anthropic only)
    #[serde(default)]
    pub blocked_domains: Option<Vec<String>>,
}

// Default value functions
fn default_provider() -> String {
    "gemini".to_string()
}

fn default_model() -> String {
    "gemini-3-flash-preview".to_string()
}

fn default_true() -> bool {
    true
}

fn default_timeout() -> u64 {
    30
}

fn default_max_age() -> u64 {
    30
}

fn default_max_messages() -> usize {
    20
}

fn default_check_interval() -> u64 {
    24
}

fn default_channel() -> String {
    "stable".to_string()
}

impl Default for DefaultConfig {
    fn default() -> Self {
        Self {
            provider: default_provider(),
            model: default_model(),
            stream: true,
        }
    }
}

impl Default for BehaviorConfig {
    fn default() -> Self {
        Self {
            auto_execute: false,
            confirm_destructive: true,
            timeout: default_timeout(),
        }
    }
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            max_age_minutes: default_max_age(),
            max_messages: default_max_messages(),
            storage_path: None,
        }
    }
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self {
            auto_check: true,
            check_interval_hours: default_check_interval(),
            channel: default_channel(),
            aggressive: true,
        }
    }
}

impl Config {
    /// Apply CLI argument overrides and profile selection
    pub fn with_cli_overrides(mut self, args: &Args) -> Self {
        // First apply profile if specified (CLI > default_profile > first profile)
        let profile_name = args
            .profile
            .clone()
            .or_else(|| self.default_profile.clone())
            .or_else(|| self.profiles.keys().next().cloned());

        if let Some(ref name) = profile_name {
            if let Some(profile) = self.profiles.get(name) {
                self = self.apply_profile(profile.clone());
            }
        }

        // Then apply direct CLI overrides (these take precedence over profile)
        if let Some(ref provider) = args.provider {
            self.default.provider = provider.clone();
        }
        if let Some(ref model) = args.model {
            self.default.model = model.clone();
        }
        self
    }

    /// Apply profile settings over current config (inheritance)
    fn apply_profile(&mut self, profile: ProfileConfig) -> Self {
        if let Some(provider) = profile.provider {
            self.default.provider = provider;
        }
        if let Some(model) = profile.model {
            self.default.model = model;
        }
        if let Some(api_key) = profile.api_key {
            let provider_name = self.default.provider.clone();
            self.providers.entry(provider_name).or_default().api_key = Some(api_key);
        }
        if let Some(base_url) = profile.base_url {
            let provider_name = self.default.provider.clone();
            self.providers.entry(provider_name).or_default().base_url = Some(base_url);
        }
        self.clone()
    }

    /// Get active profile name (if any)
    pub fn active_profile(&self, args: &Args) -> Option<String> {
        // 1. CLI argument takes precedence
        if let Some(ref profile) = args.profile {
            return Some(profile.clone());
        }

        // 2. If only one profile exists, use it automatically
        if self.profiles.len() == 1 {
            return self.profiles.keys().next().cloned();
        }

        // 3. Use configured default_profile
        if let Some(ref default) = self.default_profile {
            if self.profiles.contains_key(default) {
                return Some(default.clone());
            }
        }

        // 4. Fall back to first available profile
        self.profiles.keys().next().cloned()
    }

    /// Get fallback profile for the active profile
    /// Returns None if fallback = "none", Some(name) for specific profile,
    /// or first available profile for fallback = "any" or default behavior
    pub fn fallback_profile(&self, active_profile: &str) -> Option<String> {
        let profile = self.profiles.get(active_profile)?;

        match profile.fallback.as_deref() {
            Some("none") => None,
            Some("any") => self
                .profiles
                .keys()
                .find(|k| k.as_str() != active_profile)
                .cloned(),
            Some(specific) => {
                if self.profiles.contains_key(specific) {
                    Some(specific.to_string())
                } else {
                    None
                }
            }
            None => self
                .profiles
                .keys()
                .find(|k| k.as_str() != active_profile)
                .cloned(),
        }
    }

    /// Get the active provider name
    pub fn active_provider(&self) -> &str {
        &self.default.provider
    }

    /// Get the active model
    pub fn active_model(&self) -> &str {
        &self.default.model
    }

    /// Get API key for the active provider
    pub fn api_key(&self) -> Option<String> {
        let provider = self.active_provider();

        // First check environment variable
        let env_key = format!("ASK_{}_API_KEY", provider.to_uppercase());
        if let Ok(key) = std::env::var(&env_key) {
            return Some(key);
        }

        // Then check providers config (which may have been set from profile)
        if let Some(key) = self.providers.get(provider).and_then(|p| p.api_key.clone()) {
            return Some(key);
        }

        // Finally check profile directly
        if let Some(profile_name) = &self.default_profile {
            if let Some(profile) = self.profiles.get(profile_name) {
                if let Some(ref key) = profile.api_key {
                    return Some(key.clone());
                }
            }
        }

        // Check first profile
        for profile in self.profiles.values() {
            if let Some(ref key) = profile.api_key {
                return Some(key.clone());
            }
        }

        None
    }

    /// Get base URL for the active provider
    pub fn base_url(&self) -> Option<String> {
        self.providers
            .get(self.active_provider())
            .and_then(|p| p.base_url.clone())
    }

    /// Get context storage path
    pub fn context_storage_path(&self) -> std::path::PathBuf {
        if let Some(ref path) = self.context.storage_path {
            let expanded = shellexpand::tilde(path);
            std::path::PathBuf::from(expanded.as_ref())
        } else {
            dirs::data_local_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("ask")
                .join("contexts")
        }
    }

    /// Get web_search setting from active profile
    pub fn get_profile_web_search(&self) -> bool {
        for profile in self.profiles.values() {
            if let Some(web_search) = profile.web_search {
                return web_search;
            }
        }
        false
    }

    /// Get domain filters from active profile (Anthropic)
    pub fn get_profile_domain_filters(&self) -> (Option<Vec<String>>, Option<Vec<String>>) {
        for profile in self.profiles.values() {
            if profile.allowed_domains.is_some() || profile.blocked_domains.is_some() {
                return (
                    profile.allowed_domains.clone(),
                    profile.blocked_domains.clone(),
                );
            }
        }
        (None, None)
    }

    pub fn get_thinking_level(&self) -> Option<String> {
        for profile in self.profiles.values() {
            if let Some(ref level) = profile.thinking_level {
                return Some(level.clone());
            }
        }
        None
    }

    pub fn get_reasoning_effort(&self) -> Option<String> {
        for profile in self.profiles.values() {
            if let Some(ref effort) = profile.reasoning_effort {
                return Some(effort.clone());
            }
        }
        None
    }

    pub fn get_thinking_budget(&self) -> Option<i64> {
        for profile in self.profiles.values() {
            if let Some(budget) = profile.thinking_budget {
                return Some(budget);
            }
        }
        None
    }

    pub fn get_thinking_config(&self) -> (bool, Option<String>) {
        let provider = self.active_provider();
        match provider {
            "gemini" => {
                if let Some(level) = self.get_thinking_level() {
                    let enabled = level.to_lowercase() != "none" && level != "0";
                    (enabled, Some(level))
                } else {
                    (false, None)
                }
            }
            "openai" | "openai_compatible" => {
                if let Some(effort) = self.get_reasoning_effort() {
                    let enabled = effort.to_lowercase() != "none";
                    (enabled, Some(effort))
                } else {
                    (false, None)
                }
            }
            "anthropic" | "claude" => {
                if let Some(budget) = self.get_thinking_budget() {
                    let enabled = budget > 0;
                    (enabled, Some(budget.to_string()))
                } else {
                    (false, None)
                }
            }
            _ => (false, None),
        }
    }
}

fn mask_api_key(key: &str) -> String {
    if key.len() <= 8 {
        return "*".repeat(key.len());
    }
    let suffix = &key[key.len() - 4..];
    format!("****{}", suffix)
}

/// Helper for numbered selection menus
/// Formats items as "[1] item", "[2] item", etc. and returns the selected index
fn numbered_select<T: ToString>(prompt: &str, items: &[T], default: usize) -> Result<usize> {
    let numbered_items: Vec<String> = items
        .iter()
        .enumerate()
        .map(|(i, item)| format!("[{}] {}", i + 1, item.to_string()))
        .collect();

    let idx = Select::new()
        .with_prompt(prompt)
        .items(&numbered_items)
        .default(default)
        .interact()?;

    Ok(idx)
}

/// Helper struct for config management
struct ConfigManager {
    config_path: std::path::PathBuf,
    existing: Option<toml::Value>,
}

impl ConfigManager {
    fn new() -> Result<Self> {
        let home = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
        let config_path = home.join("ask.toml");

        let existing: Option<toml::Value> = if config_path.exists() {
            std::fs::read_to_string(&config_path)
                .ok()
                .and_then(|s| toml::from_str(&s).ok())
        } else {
            None
        };

        Ok(Self {
            config_path,
            existing,
        })
    }

    fn get_str(&self, keys: &[&str]) -> Option<String> {
        let mut val = self.existing.as_ref()?;
        for k in keys {
            val = val.get(*k)?;
        }
        val.as_str().map(|s| s.to_string())
    }

    fn get_bool(&self, keys: &[&str], default: bool) -> bool {
        let mut val = match self.existing.as_ref() {
            Some(v) => v,
            None => return default,
        };
        for k in keys {
            val = match val.get(*k) {
                Some(v) => v,
                None => return default,
            };
        }
        val.as_bool().unwrap_or(default)
    }

    fn get_profiles(&self) -> Vec<String> {
        self.existing
            .as_ref()
            .and_then(|e| e.get("profiles"))
            .and_then(|p| p.as_table())
            .map(|t| t.keys().cloned().collect())
            .unwrap_or_default()
    }

    fn backup(&self) -> Result<()> {
        if self.config_path.exists() {
            let home = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
            let backup_path = home.join("ask.toml.bak");
            std::fs::copy(&self.config_path, &backup_path)?;
        }
        Ok(())
    }

    fn reload(&mut self) -> Result<()> {
        self.existing = if self.config_path.exists() {
            std::fs::read_to_string(&self.config_path)
                .ok()
                .and_then(|s| toml::from_str(&s).ok())
        } else {
            None
        };
        Ok(())
    }
}

/// Configure default provider and model
fn configure_defaults(mgr: &ConfigManager) -> Result<(String, String, String, bool, String, bool)> {
    let existing_provider = mgr.get_str(&["default", "provider"]);
    let existing_model = mgr.get_str(&["default", "model"]);
    let existing_stream = mgr.get_bool(&["default", "stream"], true);
    let existing_web_search = mgr.get_bool(&["default", "web_search"], false);

    let providers = vec!["Gemini (recommended)", "OpenAI", "Anthropic Claude"];
    let default_provider_idx = match existing_provider.as_deref() {
        Some("gemini") => 0,
        Some("openai") => 1,
        Some("anthropic") => 2,
        _ => 0,
    };

    let provider_idx =
        numbered_select("Select default provider", &providers, default_provider_idx)?;

    let (provider, default_model_for_provider) = match provider_idx {
        0 => ("gemini", defaults::DEFAULT_MODEL),
        1 => ("openai", defaults::DEFAULT_OPENAI_MODEL),
        2 => ("anthropic", defaults::DEFAULT_ANTHROPIC_MODEL),
        _ => ("gemini", defaults::DEFAULT_MODEL),
    };

    let model_default = if existing_provider.as_deref() == Some(provider) {
        existing_model.unwrap_or_else(|| default_model_for_provider.to_string())
    } else {
        default_model_for_provider.to_string()
    };

    let model: String = Input::new()
        .with_prompt("Model")
        .default(model_default)
        .interact_text()?;

    let existing_api_key = mgr
        .get_str(&["providers", provider, "api_key"])
        .unwrap_or_default();

    let api_key: String = if !existing_api_key.is_empty() {
        let masked = mask_api_key(&existing_api_key);
        let new_key: String = Input::new()
            .with_prompt(format!("{} API key [{}] (Enter to keep)", provider, masked))
            .allow_empty(true)
            .interact_text()?;

        if new_key.is_empty() {
            existing_api_key
        } else {
            new_key
        }
    } else {
        Input::new()
            .with_prompt(format!("Enter {} API key", provider))
            .interact_text()?
    };

    let stream = Confirm::new()
        .with_prompt("Enable streaming responses?")
        .default(existing_stream)
        .interact()?;

    let thinking_config = if let Some((key, value)) = select_thinking_config(provider, &model)? {
        format_thinking_config(&key, &value)
    } else {
        String::new()
    };

    let web_search = Confirm::new()
        .with_prompt("Enable web search by default?")
        .default(existing_web_search)
        .interact()?;

    Ok((
        provider.to_string(),
        model,
        api_key,
        stream,
        thinking_config,
        web_search,
    ))
}

/// Configure a single profile
fn configure_profile(mgr: &ConfigManager, profile_name: Option<&str>) -> Result<Option<String>> {
    let name: String = if let Some(n) = profile_name {
        n.to_string()
    } else {
        Input::new()
            .with_prompt("Profile name (e.g., work, personal, local)")
            .interact_text()?
    };

    if name.is_empty() {
        return Ok(None);
    }

    println!();
    println!("{}", format!("Configuring profile: {}", name).cyan());

    let providers = vec!["Gemini", "OpenAI", "Anthropic Claude"];
    let existing_provider = mgr.get_str(&["profiles", &name, "provider"]);

    let default_idx = match existing_provider.as_deref() {
        Some("gemini") => 0,
        Some("openai") => 1,
        Some("anthropic") => 2,
        _ => 0,
    };

    let provider_idx = numbered_select("Provider for this profile", &providers, default_idx)?;

    let (provider, default_model) = match provider_idx {
        0 => ("gemini", defaults::DEFAULT_MODEL),
        1 => ("openai", defaults::DEFAULT_OPENAI_MODEL),
        2 => ("anthropic", defaults::DEFAULT_ANTHROPIC_MODEL),
        _ => ("gemini", defaults::DEFAULT_MODEL),
    };

    let existing_model = mgr
        .get_str(&["profiles", &name, "model"])
        .unwrap_or_else(|| default_model.to_string());

    let model: String = Input::new()
        .with_prompt("Model")
        .default(existing_model)
        .interact_text()?;

    let existing_api_key = mgr
        .get_str(&["profiles", &name, "api_key"])
        .or_else(|| mgr.get_str(&["providers", provider, "api_key"]))
        .unwrap_or_default();

    let api_key: String = if !existing_api_key.is_empty() {
        let masked = mask_api_key(&existing_api_key);
        let new_key: String = Input::new()
            .with_prompt(format!("API key [{}] (Enter to keep/inherit)", masked))
            .allow_empty(true)
            .interact_text()?;

        if new_key.is_empty() {
            String::new()
        } else {
            new_key
        }
    } else {
        let key: String = Input::new()
            .with_prompt("API key (Enter to inherit from provider)")
            .allow_empty(true)
            .interact_text()?;
        key
    };

    let existing_base_url = mgr.get_str(&["profiles", &name, "base_url"]);
    let base_url: String = Input::new()
        .with_prompt("Base URL (Enter for default, or custom like http://localhost:11434/v1)")
        .default(existing_base_url.unwrap_or_default())
        .allow_empty(true)
        .interact_text()?;

    let existing_web_search = mgr.get_bool(&["profiles", &name, "web_search"], false);
    let web_search = Confirm::new()
        .with_prompt("Enable web search for this profile?")
        .default(existing_web_search)
        .interact()?;

    let thinking_config = if let Some((key, value)) = select_thinking_config(provider, &model)? {
        format_thinking_config(&key, &value)
    } else {
        String::new()
    };

    let fallback_options = vec![
        "Inherit from default",
        "Use any available profile",
        "No fallback (fail immediately)",
        "Specific profile...",
    ];

    let existing_fallback = mgr.get_str(&["profiles", &name, "fallback"]);
    let default_fallback_idx = match existing_fallback.as_deref() {
        Some("any") => 1,
        Some("none") => 2,
        Some(_) => 3,
        None => 0,
    };

    let fallback_idx =
        numbered_select("Fallback behavior", &fallback_options, default_fallback_idx)?;

    let fallback = match fallback_idx {
        0 => String::new(),
        1 => "any".to_string(),
        2 => "none".to_string(),
        3 => {
            let fb: String = Input::new()
                .with_prompt("Fallback profile name")
                .default(existing_fallback.unwrap_or_default())
                .interact_text()?;
            fb
        }
        _ => String::new(),
    };

    let mut profile_toml = format!(
        r#"
[profiles.{}]
provider = "{}"
model = "{}""#,
        name, provider, model
    );

    if !api_key.is_empty() {
        profile_toml.push_str(&format!("\napi_key = \"{}\"", api_key));
    }

    if !base_url.is_empty() {
        profile_toml.push_str(&format!("\nbase_url = \"{}\"", base_url));
    }

    if web_search {
        profile_toml.push_str("\nweb_search = true");
    }

    if !thinking_config.is_empty() {
        profile_toml.push_str(&thinking_config);
    }

    if !fallback.is_empty() {
        profile_toml.push_str(&format!("\nfallback = \"{}\"", fallback));
    }

    Ok(Some(profile_toml))
}

/// Show current configuration
fn show_current_config(mgr: &ConfigManager) {
    println!();
    println!("{}", "Current Configuration".cyan().bold());
    println!("{}", "─".repeat(50).bright_black());

    if mgr.existing.is_none() {
        println!("{}", "No configuration file found.".yellow());
        println!("Run {} to create one.", "'ask init'".cyan());
        return;
    }

    let provider = mgr
        .get_str(&["default", "provider"])
        .unwrap_or_else(|| "gemini".to_string());
    let model = mgr
        .get_str(&["default", "model"])
        .unwrap_or_else(|| "not set".to_string());
    let stream = mgr.get_bool(&["default", "stream"], true);
    let web_search = mgr.get_bool(&["default", "web_search"], false);
    let default_profile = mgr.get_str(&["default_profile"]);
    let fallback = mgr.get_str(&["default", "default_fallback"]);

    println!();
    println!("{}", "[default]".green().bold());
    println!(
        "  {} {}",
        "provider:".yellow(),
        provider.bright_white().bold()
    );
    println!("  {} {}", "model:".yellow(), model.cyan());
    println!(
        "  {} {}",
        "stream:".yellow(),
        if stream {
            "true".green()
        } else {
            "false".red()
        }
    );
    println!(
        "  {} {}",
        "web_search:".yellow(),
        if web_search {
            "true".green()
        } else {
            "false".bright_black()
        }
    );
    if let Some(dp) = default_profile {
        println!("  {} {}", "default_profile:".yellow(), dp.cyan().bold());
    }
    if let Some(fb) = fallback {
        println!("  {} {}", "default_fallback:".yellow(), fb.bright_black());
    }

    println!();
    println!("{}", "[providers]".green().bold());
    for p in &["gemini", "openai", "anthropic"] {
        let key_exists = mgr.get_str(&["providers", p, "api_key"]).is_some();
        let thinking = match *p {
            "gemini" => mgr.get_str(&["providers", p, "thinking_level"]),
            "openai" => mgr.get_str(&["providers", p, "reasoning_effort"]),
            "anthropic" => mgr
                .get_str(&["providers", p, "thinking_budget"])
                .map(|v| format!("{} tokens", v)),
            _ => None,
        };

        if key_exists {
            let key = mgr.get_str(&["providers", p, "api_key"]).unwrap();
            let thinking_str = thinking
                .map(|t| format!(" [think: {}]", t).bright_black().to_string())
                .unwrap_or_default();
            println!(
                "  {} {} {}{}",
                p.bright_white(),
                "✓".green(),
                mask_api_key(&key).bright_black(),
                thinking_str
            );
        } else {
            println!("  {} {}", p.bright_black(), "✗".red());
        }
    }

    let profiles = mgr.get_profiles();
    if !profiles.is_empty() {
        println!();
        println!("{}", "[profiles]".green().bold());
        for name in &profiles {
            let p_provider = mgr
                .get_str(&["profiles", name, "provider"])
                .unwrap_or_else(|| "inherited".to_string());
            let p_model = mgr
                .get_str(&["profiles", name, "model"])
                .unwrap_or_else(|| "inherited".to_string());
            let p_fallback = mgr
                .get_str(&["profiles", name, "fallback"])
                .unwrap_or_else(|| "default".to_string());
            let p_web_search = mgr
                .get_str(&["profiles", name, "web_search"])
                .map(|v| v == "true")
                .unwrap_or(false);

            let web_indicator = if p_web_search {
                " [search]".cyan().to_string()
            } else {
                String::new()
            };

            println!(
                "  {} {} {} {}{}",
                name.cyan().bold(),
                p_provider.bright_white(),
                p_model.bright_black(),
                format!("(fallback: {})", p_fallback).bright_black(),
                web_indicator
            );
        }
    }

    let commands: Vec<String> = mgr
        .existing
        .as_ref()
        .and_then(|doc| doc.get("commands"))
        .and_then(|c| c.as_table())
        .map(|t| t.keys().cloned().collect())
        .unwrap_or_default();

    if !commands.is_empty() {
        println!();
        println!("{}", "[commands]".green().bold());
        for cmd in &commands {
            let cmd_type = mgr
                .get_str(&["commands", cmd, "type"])
                .unwrap_or_else(|| "text".to_string());
            println!(
                "  {} {}",
                cmd.cyan(),
                format!("({})", cmd_type).bright_black()
            );
        }
    }

    println!();
    println!(
        "{}",
        format!("Config: {}", mgr.config_path.display()).bright_black()
    );
    println!();
}

fn manage_profiles(mgr: &mut ConfigManager) -> Result<()> {
    loop {
        println!();
        let profiles = mgr.get_profiles();

        let mut options = vec!["Create new profile".to_string()];
        if !profiles.is_empty() {
            options.push("Edit existing profile".to_string());
            options.push("Delete profile".to_string());
            options.push("Set default profile".to_string());
        }
        options.push("Back to main menu".to_string());

        let choice = numbered_select("Manage Profiles", &options, 0)?;

        let back_idx = options.len() - 1;

        if choice == back_idx {
            break;
        }

        match options[choice].as_str() {
            "Create new profile" => {
                if let Some(profile_toml) = configure_profile(mgr, None)? {
                    let content = std::fs::read_to_string(&mgr.config_path).unwrap_or_default();
                    let new_content = format!("{}\n{}", content, profile_toml);
                    std::fs::write(&mgr.config_path, new_content)?;
                    mgr.reload()?;
                    println!("{}", "Profile created!".green());
                }
            }
            "Edit existing profile" => {
                let profiles = mgr.get_profiles();
                if profiles.is_empty() {
                    println!("{}", "No profiles to edit.".yellow());
                    continue;
                }

                let mut items: Vec<String> = profiles.clone();
                items.push("Cancel".to_string());

                let idx = numbered_select("Select profile to edit", &items, 0)?;

                if idx < profiles.len() {
                    let profile_name = &profiles[idx];
                    if let Some(profile_toml) = configure_profile(mgr, Some(profile_name))? {
                        let content = std::fs::read_to_string(&mgr.config_path).unwrap_or_default();

                        let mut doc: toml::Value = toml::from_str(&content)?;
                        if let Some(profiles_table) = doc.get_mut("profiles") {
                            if let Some(table) = profiles_table.as_table_mut() {
                                table.remove(profile_name);
                            }
                        }

                        let new_content =
                            format!("{}\n{}", toml::to_string_pretty(&doc)?, profile_toml);
                        std::fs::write(&mgr.config_path, new_content)?;
                        mgr.reload()?;
                        println!("{}", "Profile updated!".green());
                    }
                }
            }
            "Delete profile" => {
                let profiles = mgr.get_profiles();
                if profiles.is_empty() {
                    println!("{}", "No profiles to delete.".yellow());
                    continue;
                }

                let mut items: Vec<String> = profiles.clone();
                items.push("Cancel".to_string());

                let idx = numbered_select("Select profile to delete", &items, 0)?;

                if idx < profiles.len() {
                    let profile_name = &profiles[idx];
                    let confirm = Confirm::new()
                        .with_prompt(format!("Delete profile '{}'?", profile_name))
                        .default(false)
                        .interact()?;

                    if confirm {
                        let content = std::fs::read_to_string(&mgr.config_path).unwrap_or_default();
                        let mut doc: toml::Value = toml::from_str(&content)?;
                        if let Some(profiles_table) = doc.get_mut("profiles") {
                            if let Some(table) = profiles_table.as_table_mut() {
                                table.remove(profile_name);
                            }
                        }
                        std::fs::write(&mgr.config_path, toml::to_string_pretty(&doc)?)?;
                        mgr.reload()?;
                        println!("{}", "Profile deleted!".green());
                    }
                }
            }
            "Set default profile" => {
                let profiles = mgr.get_profiles();
                if profiles.is_empty() {
                    println!("{}", "No profiles available.".yellow());
                    continue;
                }

                let current_default = mgr.get_str(&["default", "default_profile"]);
                let default_idx = current_default
                    .as_ref()
                    .and_then(|d| profiles.iter().position(|p| p == d))
                    .unwrap_or(0);

                let idx = numbered_select("Select default profile", &profiles, default_idx)?;

                let profile_name = &profiles[idx];

                let content = std::fs::read_to_string(&mgr.config_path).unwrap_or_default();
                let mut doc: toml::Value = toml::from_str(&content)?;

                if let Some(default_section) = doc.get_mut("default") {
                    if let Some(table) = default_section.as_table_mut() {
                        table.insert(
                            "default_profile".to_string(),
                            toml::Value::String(profile_name.clone()),
                        );
                    }
                }

                std::fs::write(&mgr.config_path, toml::to_string_pretty(&doc)?)?;
                mgr.reload()?;
                println!(
                    "{} {}",
                    "Default profile set to:".green(),
                    profile_name.cyan()
                );
            }
            _ => {}
        }
    }

    Ok(())
}

/// Initialize configuration interactively
pub async fn init_config() -> Result<()> {
    println!("{}", "ask configuration".cyan().bold());
    println!();

    let mut mgr = ConfigManager::new()?;

    if mgr.existing.is_some() {
        println!(
            "{}",
            format!("Config found: {}", mgr.config_path.display()).bright_black()
        );
    }

    loop {
        println!();
        let menu_options = if mgr.existing.is_some() {
            vec![
                "View current config",
                "Edit default settings",
                "Manage API keys",
                "Manage profiles",
                "Configure fallback behavior",
                "Exit",
            ]
        } else {
            vec!["Quick setup (recommended)", "Exit"]
        };

        let choice = numbered_select("What would you like to do?", &menu_options, 0)?;

        if mgr.existing.is_none() {
            match choice {
                0 => {
                    mgr.backup()?;

                    let (provider, model, api_key, stream, thinking_config, web_search) =
                        configure_defaults(&mgr)?;

                    let web_search_config = if web_search {
                        "\nweb_search = true"
                    } else {
                        ""
                    };

                    let config_content = format!(
                        r#"# ask configuration
# Generated by 'ask init'

# Default profile to use
default_profile = "first"

# All configuration lives in profiles
# Switch profiles with: ask -p <profile_name>
[profiles.first]
provider = "{provider}"
model = "{model}"
api_key = "{api_key}"
stream = {stream}{thinking_config}{web_search_config}

[behavior]
auto_execute = false
confirm_destructive = true
timeout = 30

[context]
max_age_minutes = 30
max_messages = 20

[update]
auto_check = true
aggressive = true
check_interval_hours = 24
channel = "stable"

# Custom commands example:
# [commands.cm]
# system = "Generate concise git commit message based on diff"
# type = "command"
# auto_execute = false
"#
                    );

                    std::fs::write(&mgr.config_path, config_content)?;
                    mgr.reload()?;

                    println!();
                    println!(
                        "{} {}",
                        "Created".green(),
                        mgr.config_path.display().to_string().bright_white()
                    );
                    println!();
                    println!("Profile '{}' created and set as default!", "first".cyan());
                    println!("Try: {}", "ask how to list files".cyan());
                }
                1 => {
                    println!("{}", "Goodbye!".bright_black());
                    break;
                }
                _ => {}
            }
        } else {
            match choice {
                0 => {
                    show_current_config(&mgr);
                }
                1 => {
                    mgr.backup()?;

                    let (provider, model, api_key, stream, thinking_config, web_search) =
                        configure_defaults(&mgr)?;

                    let content = std::fs::read_to_string(&mgr.config_path).unwrap_or_default();
                    let mut doc: toml::Value = toml::from_str(&content)?;

                    if let Some(default_section) = doc.get_mut("default") {
                        if let Some(table) = default_section.as_table_mut() {
                            table.insert(
                                "provider".to_string(),
                                toml::Value::String(provider.clone()),
                            );
                            table.insert("model".to_string(), toml::Value::String(model.clone()));
                            table.insert("stream".to_string(), toml::Value::Boolean(stream));
                            table
                                .insert("web_search".to_string(), toml::Value::Boolean(web_search));
                        }
                    }

                    if let Some(providers_section) = doc.get_mut("providers") {
                        if let Some(table) = providers_section.as_table_mut() {
                            let provider_table = table
                                .entry(provider.clone())
                                .or_insert(toml::Value::Table(toml::map::Map::new()));
                            if let Some(pt) = provider_table.as_table_mut() {
                                pt.insert("api_key".to_string(), toml::Value::String(api_key));
                                if !thinking_config.is_empty() {
                                    if thinking_config.contains("thinking_budget") {
                                        if let Some(val) = thinking_config
                                            .split('=')
                                            .nth(1)
                                            .and_then(|s| s.trim().parse::<i64>().ok())
                                        {
                                            pt.insert(
                                                "thinking_budget".to_string(),
                                                toml::Value::Integer(val),
                                            );
                                        }
                                    } else if thinking_config.contains("thinking_level") {
                                        if let Some(val) = thinking_config
                                            .split('=')
                                            .nth(1)
                                            .map(|s| s.trim().trim_matches('"').to_string())
                                        {
                                            pt.insert(
                                                "thinking_level".to_string(),
                                                toml::Value::String(val),
                                            );
                                        }
                                    } else if thinking_config.contains("reasoning_effort") {
                                        if let Some(val) = thinking_config
                                            .split('=')
                                            .nth(1)
                                            .map(|s| s.trim().trim_matches('"').to_string())
                                        {
                                            pt.insert(
                                                "reasoning_effort".to_string(),
                                                toml::Value::String(val),
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }

                    std::fs::write(&mgr.config_path, toml::to_string_pretty(&doc)?)?;
                    mgr.reload()?;
                    println!("{}", "Default settings updated!".green());
                }
                2 => {
                    mgr.backup()?;

                    let providers_list = vec!["Gemini", "OpenAI", "Anthropic Claude", "Back"];
                    let idx = numbered_select("Which provider API key?", &providers_list, 0)?;

                    if idx < 3 {
                        let provider = match idx {
                            0 => "gemini",
                            1 => "openai",
                            2 => "anthropic",
                            _ => continue,
                        };

                        let existing_key = mgr
                            .get_str(&["providers", provider, "api_key"])
                            .unwrap_or_default();

                        let new_key: String = if !existing_key.is_empty() {
                            let masked = mask_api_key(&existing_key);
                            Input::new()
                                .with_prompt(format!("API key [{}] (Enter to keep)", masked))
                                .allow_empty(true)
                                .interact_text()?
                        } else {
                            Input::new()
                                .with_prompt(format!("{} API key", provider))
                                .interact_text()?
                        };

                        let final_key = if new_key.is_empty() {
                            existing_key
                        } else {
                            new_key
                        };

                        if !final_key.is_empty() {
                            let content =
                                std::fs::read_to_string(&mgr.config_path).unwrap_or_default();
                            let mut doc: toml::Value = toml::from_str(&content)?;

                            if doc.get("providers").is_none() {
                                if let Some(table) = doc.as_table_mut() {
                                    table.insert(
                                        "providers".to_string(),
                                        toml::Value::Table(toml::map::Map::new()),
                                    );
                                }
                            }

                            if let Some(providers_section) = doc.get_mut("providers") {
                                if let Some(table) = providers_section.as_table_mut() {
                                    let provider_table = table
                                        .entry(provider.to_string())
                                        .or_insert(toml::Value::Table(toml::map::Map::new()));
                                    if let Some(pt) = provider_table.as_table_mut() {
                                        pt.insert(
                                            "api_key".to_string(),
                                            toml::Value::String(final_key),
                                        );
                                    }
                                }
                            }

                            std::fs::write(&mgr.config_path, toml::to_string_pretty(&doc)?)?;
                            mgr.reload()?;
                            println!("{}", "API key updated!".green());
                        }
                    }
                }
                3 => {
                    manage_profiles(&mut mgr)?;
                }
                4 => {
                    mgr.backup()?;

                    let fallback_options = vec![
                        "Use any available profile (recommended)",
                        "No fallback (fail immediately)",
                    ];

                    let existing_fallback = mgr.get_str(&["default", "default_fallback"]);
                    let default_idx = match existing_fallback.as_deref() {
                        Some("none") => 1,
                        _ => 0,
                    };

                    let idx = numbered_select(
                        "Default fallback behavior when provider fails?",
                        &fallback_options,
                        default_idx,
                    )?;

                    let fallback_value = match idx {
                        0 => "any",
                        _ => "none",
                    };

                    let content = std::fs::read_to_string(&mgr.config_path).unwrap_or_default();
                    let mut doc: toml::Value = toml::from_str(&content)?;

                    if let Some(default_section) = doc.get_mut("default") {
                        if let Some(table) = default_section.as_table_mut() {
                            table.insert(
                                "default_fallback".to_string(),
                                toml::Value::String(fallback_value.to_string()),
                            );
                        }
                    }

                    std::fs::write(&mgr.config_path, toml::to_string_pretty(&doc)?)?;
                    mgr.reload()?;
                    println!("{} {}", "Fallback set to:".green(), fallback_value.cyan());
                }
                5 => {
                    println!("{}", "Goodbye!".bright_black());
                    break;
                }
                _ => {}
            }
        }
    }

    Ok(())
}

pub fn init_config_non_interactive(
    provider: Option<&str>,
    model: Option<&str>,
    api_key: Option<&str>,
) -> Result<()> {
    let provider = provider.unwrap_or("gemini");
    let model = model.unwrap_or(match provider {
        "openai" => "gpt-4o",
        "anthropic" => "claude-sonnet-4-20250514",
        _ => "gemini-2.5-flash-preview-05-20",
    });

    let api_key = match api_key {
        Some(k) => k.to_string(),
        None => {
            let env_key =
                match provider {
                    "openai" => std::env::var("OPENAI_API_KEY")
                        .or_else(|_| std::env::var("ASK_OPENAI_API_KEY")),
                    "anthropic" => std::env::var("ANTHROPIC_API_KEY")
                        .or_else(|_| std::env::var("ASK_ANTHROPIC_API_KEY")),
                    _ => std::env::var("GEMINI_API_KEY")
                        .or_else(|_| std::env::var("ASK_GEMINI_API_KEY")),
                };
            env_key.map_err(|_| {
                anyhow::anyhow!(
                    "No API key provided. Use --api-key or set {}_API_KEY environment variable",
                    provider.to_uppercase()
                )
            })?
        }
    };

    let config_dir = dirs::config_dir()
        .map(|p| p.join("ask"))
        .unwrap_or_else(|| std::path::PathBuf::from("."));

    std::fs::create_dir_all(&config_dir)?;
    let config_path = config_dir.join("config.toml");

    let config_content = format!(
        r#"# ask configuration (generated by --non-interactive)

[default]
provider = "{provider}"
model = "{model}"
stream = true

[providers.{provider}]
api_key = "{api_key}"

[behavior]
auto_execute = false
confirm_destructive = true
timeout = 30

[context]
max_age_minutes = 30
max_messages = 20

[update]
auto_check = true
check_interval_hours = 24
channel = "stable"
"#
    );

    std::fs::write(&config_path, config_content)?;

    println!(
        "{} {}",
        "Created".green(),
        config_path.display().to_string().bright_white()
    );
    println!(
        "{} provider={}, model={}",
        "Configured:".green(),
        provider.cyan(),
        model.cyan()
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_profile_inheritance() {
        let mut config = Config::default();
        config.default.provider = "gemini".to_string();
        config.default.model = "gemini-flash".to_string();

        let profile = ProfileConfig {
            provider: Some("anthropic".to_string()),
            model: None, // Should keep default
            api_key: Some("test-key".to_string()),
            ..Default::default()
        };

        let new_config = config.apply_profile(profile);

        assert_eq!(new_config.default.provider, "anthropic");
        assert_eq!(new_config.default.model, "gemini-flash");
        assert_eq!(
            new_config.providers.get("anthropic").unwrap().api_key,
            Some("test-key".to_string())
        );
    }

    #[test]
    fn test_cli_overrides_precedence() {
        let mut config = Config::default();
        config.profiles.insert(
            "work".to_string(),
            ProfileConfig {
                provider: Some("openai".to_string()),
                model: Some("gpt-4".to_string()),
                ..Default::default()
            },
        );

        // Case 1: Just profile
        let args_profile = Args {
            profile: Some("work".to_string()),
            ..Default::default()
        };
        let cfg1 = config.clone().with_cli_overrides(&args_profile);
        assert_eq!(cfg1.default.provider, "openai");
        assert_eq!(cfg1.default.model, "gpt-4");

        // Case 2: Profile + Provider Override
        let args_override = Args {
            profile: Some("work".to_string()),
            provider: Some("anthropic".to_string()),
            ..Default::default()
        };
        let cfg2 = config.clone().with_cli_overrides(&args_override);
        assert_eq!(cfg2.default.provider, "anthropic"); // CLI wins
        assert_eq!(cfg2.default.model, "gpt-4"); // Profile keeps model

        // Case 3: Profile + Model Override
        let args_model = Args {
            profile: Some("work".to_string()),
            model: Some("claude-3".to_string()),
            ..Default::default()
        };
        let cfg3 = config.clone().with_cli_overrides(&args_model);
        assert_eq!(cfg3.default.provider, "openai");
        assert_eq!(cfg3.default.model, "claude-3"); // CLI wins
    }

    #[test]
    fn test_thinking_config_logic() {
        let mut config = Config::default();

        // Gemini Thinking
        config.default.provider = "gemini".to_string();
        config.profiles.insert(
            "thinker".to_string(),
            ProfileConfig {
                thinking_level: Some("high".to_string()),
                ..Default::default()
            },
        );
        let cfg_gem = config
            .clone()
            .apply_profile(config.profiles.get("thinker").unwrap().clone());
        let (enabled, value) = cfg_gem.get_thinking_config();
        assert!(enabled);
        assert_eq!(value, Some("high".to_string()));

        // Anthropic Thinking
        let mut config_anth = Config::default();
        config_anth.default.provider = "anthropic".to_string();
        config_anth.profiles.insert(
            "thinker".to_string(),
            ProfileConfig {
                thinking_budget: Some(2048),
                ..Default::default()
            },
        );
        let cfg_anth = config_anth
            .clone()
            .apply_profile(config_anth.profiles.get("thinker").unwrap().clone());
        let (enabled, value) = cfg_anth.get_thinking_config();
        assert!(enabled);
        assert_eq!(value, Some("2048".to_string()));

        // OpenAI Reasoning
        let mut config_oai = Config::default();
        config_oai.default.provider = "openai".to_string();
        config_oai.profiles.insert(
            "thinker".to_string(),
            ProfileConfig {
                reasoning_effort: Some("medium".to_string()),
                ..Default::default()
            },
        );
        let cfg_oai = config_oai
            .clone()
            .apply_profile(config_oai.profiles.get("thinker").unwrap().clone());
        let (enabled, value) = cfg_oai.get_thinking_config();
        assert!(enabled);
        assert_eq!(value, Some("medium".to_string()));
    }

    #[test]
    fn test_fallback_profile_selection() {
        let mut config = Config::default();
        config.profiles.insert(
            "p1".to_string(),
            ProfileConfig {
                fallback: Some("p2".to_string()),
                ..Default::default()
            },
        );
        config.profiles.insert(
            "p2".to_string(),
            ProfileConfig {
                fallback: Some("none".to_string()),
                ..Default::default()
            },
        );
        config.profiles.insert(
            "p3".to_string(),
            ProfileConfig {
                fallback: Some("any".to_string()),
                ..Default::default()
            },
        );

        // Specific fallback
        assert_eq!(config.fallback_profile("p1"), Some("p2".to_string()));

        // None fallback
        assert_eq!(config.fallback_profile("p2"), None);

        // Any fallback (should return another profile, e.g., p1 or p2)
        let fallback_any = config.fallback_profile("p3");
        assert!(fallback_any.is_some());
        assert_ne!(fallback_any.unwrap(), "p3");
    }
}
