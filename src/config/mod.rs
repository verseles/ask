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

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    "gemini-2.0-flash".to_string()
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

impl Default for Config {
    fn default() -> Self {
        Self {
            default: DefaultConfig::default(),
            providers: HashMap::new(),
            behavior: BehaviorConfig::default(),
            context: ContextConfig::default(),
            update: UpdateConfig::default(),
            commands: HashMap::new(),
        }
    }
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
        self.providers
            .get(provider)
            .and_then(|p| p.api_key.clone())
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

/// Initialize configuration interactively
pub async fn init_config() -> Result<()> {
    println!("{}", "Welcome to ask configuration!".cyan().bold());
    println!();

    // Check if config already exists
    let home = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    let config_path = home.join("ask.toml");

    if config_path.exists() {
        let overwrite = Confirm::new()
            .with_prompt("Configuration already exists. Overwrite?")
            .default(false)
            .interact()?;

        if !overwrite {
            println!("{}", "Configuration unchanged.".yellow());
            return Ok(());
        }

        // Backup existing config
        let backup_path = home.join("ask.toml.bak");
        std::fs::copy(&config_path, &backup_path)?;
        println!("{}", format!("Backed up to {}", backup_path.display()).bright_black());
    }

    // Select provider
    let providers = vec!["Gemini (recommended)", "OpenAI", "Anthropic Claude"];
    let provider_idx = Select::new()
        .with_prompt("Select default provider")
        .items(&providers)
        .default(0)
        .interact()?;

    let (provider, default_model) = match provider_idx {
        0 => ("gemini", "gemini-2.0-flash"),
        1 => ("openai", "gpt-4o-mini"),
        2 => ("anthropic", "claude-3-5-sonnet-20241022"),
        _ => ("gemini", "gemini-2.0-flash"),
    };

    // Get API key
    let api_key: String = Input::new()
        .with_prompt(format!("Enter {} API key", provider))
        .interact_text()?;

    // Enable streaming?
    let stream = Confirm::new()
        .with_prompt("Enable streaming responses?")
        .default(true)
        .interact()?;

    // Build config
    let config_content = format!(
        r#"# ask configuration
# Generated by 'ask init'

[default]
provider = "{provider}"
model = "{default_model}"
stream = {stream}

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
