//! Configuration module - handles loading and merging configs

mod defaults;
mod loader;

pub use defaults::*;

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

/// Named profile configuration - overrides default settings
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProfileConfig {
    /// Provider name (gemini, openai, anthropic)
    #[serde(default)]
    pub provider: Option<String>,

    /// Model name
    #[serde(default)]
    pub model: Option<String>,

    /// API key (overrides provider's api_key)
    #[serde(default)]
    pub api_key: Option<String>,

    /// Base URL (for OpenAI-compatible endpoints like Ollama)
    #[serde(default)]
    pub base_url: Option<String>,

    /// Fallback profile name ("none" to disable, "any" for first available)
    #[serde(default)]
    pub fallback: Option<String>,

    /// Thinking level for Gemini (none, low, medium, high)
    #[serde(default)]
    pub thinking_level: Option<String>,

    /// Reasoning effort for OpenAI (none, minimal, low, medium, high)
    #[serde(default)]
    pub reasoning_effort: Option<String>,

    /// Thinking budget for Anthropic (token count)
    #[serde(default)]
    pub thinking_budget: Option<u64>,

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
    #[allow(dead_code)]
    pub fn active_profile(&self, args: &Args) -> Option<String> {
        args.profile
            .clone()
            .or_else(|| self.default_profile.clone())
            .or_else(|| self.profiles.keys().next().cloned())
    }

    /// Get fallback profile for the active profile
    /// Returns None if fallback = "none", Some(name) for specific profile,
    /// or first available profile for fallback = "any" or default behavior
    #[allow(dead_code)]
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

        // Then check config
        self.providers.get(provider).and_then(|p| p.api_key.clone())
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
}

fn mask_api_key(key: &str) -> String {
    if key.len() <= 8 {
        return "*".repeat(key.len());
    }
    let suffix = &key[key.len() - 4..];
    format!("****{}", suffix)
}

/// Initialize configuration interactively
pub async fn init_config() -> Result<()> {
    println!("{}", "Welcome to ask configuration!".cyan().bold());
    println!();

    let home = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    let config_path = home.join("ask.toml");

    // Try to parse existing config as raw TOML for reading values
    let existing: Option<toml::Value> = if config_path.exists() {
        println!(
            "{}",
            format!("Existing config found: {}", config_path.display()).bright_black()
        );
        println!("{}", "Press Enter to keep current values.".bright_black());
        println!();

        let backup_path = home.join("ask.toml.bak");
        std::fs::copy(&config_path, &backup_path)?;

        std::fs::read_to_string(&config_path)
            .ok()
            .and_then(|s| toml::from_str(&s).ok())
    } else {
        None
    };

    // Helper to get nested TOML values
    let get_str = |keys: &[&str]| -> Option<String> {
        let mut val = existing.as_ref()?;
        for k in keys {
            val = val.get(*k)?;
        }
        val.as_str().map(|s| s.to_string())
    };

    let get_bool = |keys: &[&str], default: bool| -> bool {
        let mut val = match existing.as_ref() {
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
    };

    let get_int = |keys: &[&str]| -> Option<i64> {
        let mut val = existing.as_ref()?;
        for k in keys {
            val = val.get(*k)?;
        }
        val.as_integer()
    };

    let existing_provider = get_str(&["default", "provider"]);
    let existing_model = get_str(&["default", "model"]);
    let existing_stream = get_bool(&["default", "stream"], true);

    // Select provider
    let providers = vec!["Gemini (recommended)", "OpenAI", "Anthropic Claude"];
    let default_provider_idx = match existing_provider.as_deref() {
        Some("gemini") => 0,
        Some("openai") => 1,
        Some("anthropic") => 2,
        _ => 0,
    };

    let provider_idx = Select::new()
        .with_prompt("Select default provider")
        .items(&providers)
        .default(default_provider_idx)
        .interact()?;

    let (provider, default_model_for_provider) = match provider_idx {
        0 => ("gemini", defaults::DEFAULT_MODEL),
        1 => ("openai", defaults::DEFAULT_OPENAI_MODEL),
        2 => ("anthropic", defaults::DEFAULT_ANTHROPIC_MODEL),
        _ => ("gemini", defaults::DEFAULT_MODEL),
    };

    // Use existing model if same provider
    let model_default = if existing_provider.as_deref() == Some(provider) {
        existing_model.unwrap_or_else(|| default_model_for_provider.to_string())
    } else {
        default_model_for_provider.to_string()
    };

    let model: String = Input::new()
        .with_prompt("Model")
        .default(model_default)
        .interact_text()?;

    // Get existing API key for this provider
    let existing_api_key = get_str(&["providers", provider, "api_key"]).unwrap_or_default();

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

    let (thinking_param, thinking_default) = match provider {
        "gemini" => ("thinking_level", "low"),
        "openai" => ("reasoning_effort", "low"),
        "anthropic" => ("thinking_budget", "5000"),
        _ => ("thinking_level", "low"),
    };

    // Get existing thinking value
    let existing_thinking = if provider == "anthropic" {
        get_int(&["providers", provider, thinking_param]).map(|i| i.to_string())
    } else {
        get_str(&["providers", provider, thinking_param])
    };

    let thinking_existing = existing_thinking
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| thinking_default.to_string());

    let thinking_value: String = Input::new()
        .with_prompt(format!("Thinking mode ({}, 0 to disable)", thinking_param))
        .default(thinking_existing)
        .interact_text()?;

    let thinking_config = if thinking_value == "0" {
        String::new()
    } else if provider == "anthropic" {
        format!("\nthinking_budget = {}", thinking_value)
    } else {
        format!("\n{} = \"{}\"", thinking_param, thinking_value)
    };

    // Build config
    let config_content = format!(
        r#"# ask configuration
# Generated by 'ask init'

[default]
provider = "{provider}"
model = "{model}"
stream = {stream}

[providers.{provider}]
api_key = "{api_key}"{thinking_config}

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

# Custom commands example:
# [commands.cm]
# system = "Generate concise git commit message based on diff"
# type = "command"
# auto_execute = false
"#
    );

    std::fs::write(&config_path, config_content)?;

    println!();
    println!(
        "{} {}",
        "Created".green(),
        config_path.display().to_string().bright_white()
    );
    println!();
    println!("You're all set! Try: {}", "ask how to list files".cyan());

    Ok(())
}
