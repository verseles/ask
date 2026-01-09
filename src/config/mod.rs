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
    /// Apply CLI argument overrides
    pub fn with_cli_overrides(mut self, args: &Args) -> Self {
        if let Some(ref provider) = args.provider {
            self.default.provider = provider.clone();
        }
        if let Some(ref model) = args.model {
            self.default.model = model.clone();
        }
        self
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
