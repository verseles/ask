//! Configuration module - handles loading and merging configs

mod defaults;
mod loader;
mod thinking;

pub use defaults::*;
pub use thinking::{format_thinking_config, select_thinking_config};

use crate::cli::Args;
use anyhow::Result;
use colored::Colorize;
use requestty::Question;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// Named profiles - all provider/model config lives here
    #[serde(default)]
    pub profiles: HashMap<String, ProfileConfig>,

    /// Default profile name (only set when user explicitly chooses)
    /// If not set, first profile is used automatically
    #[serde(default)]
    pub default_profile: Option<String>,

    #[serde(default)]
    pub behavior: BehaviorConfig,

    #[serde(default)]
    pub context: ContextConfig,

    #[serde(default)]
    pub update: UpdateConfig,

    #[serde(default)]
    pub commands: HashMap<String, CustomCommand>,

    /// Command-line aliases (e.g., "q" = "--raw --no-color")
    #[serde(default)]
    pub aliases: HashMap<String, String>,

    /// Active profile data (set after profile resolution, not from TOML)
    #[serde(skip)]
    pub active: ActiveConfig,
}

#[derive(Debug, Clone, Default)]
pub struct ActiveConfig {
    pub provider: String,
    pub model: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub stream: bool,
    pub profile_name: Option<String>,
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
    pub fn with_cli_overrides(mut self, args: &Args) -> Self {
        let ad_hoc_provider = args
            .provider
            .clone()
            .or_else(|| std::env::var("ASK_PROVIDER").ok());

        if let Some(ref provider) = ad_hoc_provider {
            self.active = ActiveConfig {
                provider: provider.clone(),
                model: args
                    .model
                    .clone()
                    .or_else(|| std::env::var("ASK_MODEL").ok())
                    .unwrap_or_else(|| self.default_model_for_provider(provider)),
                api_key: args.api_key.clone().or_else(|| self.env_api_key(provider)),
                base_url: self.env_base_url(provider),
                stream: true,
                profile_name: None,
            };
            return self;
        }

        // Profile mode: resolve profile (CLI -p > ENV > default_profile > first)
        let profile_name = args
            .profile
            .clone()
            .or_else(|| std::env::var("ASK_PROFILE").ok())
            .or_else(|| self.default_profile.clone())
            .or_else(|| self.profiles.keys().next().cloned());

        if let Some(ref name) = profile_name {
            if let Some(profile) = self.profiles.get(name) {
                let provider = profile.provider.clone().unwrap_or_else(default_provider);
                self.active = ActiveConfig {
                    provider: provider.clone(),
                    model: args
                        .model
                        .clone()
                        .or_else(|| profile.model.clone())
                        .unwrap_or_else(|| self.default_model_for_provider(&provider)),
                    api_key: profile
                        .api_key
                        .clone()
                        .or_else(|| self.env_api_key(&provider)),
                    base_url: profile
                        .base_url
                        .clone()
                        .or_else(|| self.env_base_url(&provider)),
                    stream: profile.stream.unwrap_or(true),
                    profile_name: Some(name.clone()),
                };
            }
        }

        self
    }

    fn default_model_for_provider(&self, provider: &str) -> String {
        match provider {
            "openai" => defaults::DEFAULT_OPENAI_MODEL.to_string(),
            "anthropic" => defaults::DEFAULT_ANTHROPIC_MODEL.to_string(),
            _ => defaults::DEFAULT_MODEL.to_string(),
        }
    }

    fn env_api_key(&self, provider: &str) -> Option<String> {
        let env_key = format!("ASK_{}_API_KEY", provider.to_uppercase());
        std::env::var(&env_key).ok()
    }

    fn env_base_url(&self, provider: &str) -> Option<String> {
        let env_key = format!("ASK_{}_BASE_URL", provider.to_uppercase());
        std::env::var(&env_key).ok()
    }

    pub fn active_profile(&self, args: &Args) -> Option<String> {
        if args.provider.is_some() {
            return None; // Ad-hoc mode has no profile
        }

        args.profile
            .clone()
            .or_else(|| std::env::var("ASK_PROFILE").ok())
            .or_else(|| {
                if self.profiles.len() == 1 {
                    self.profiles.keys().next().cloned()
                } else {
                    None
                }
            })
            .or_else(|| {
                self.default_profile
                    .clone()
                    .filter(|dp| self.profiles.contains_key(dp))
            })
            .or_else(|| self.profiles.keys().next().cloned())
    }

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

    pub fn active_provider(&self) -> &str {
        &self.active.provider
    }

    pub fn active_model(&self) -> &str {
        &self.active.model
    }

    pub fn api_key(&self) -> Option<String> {
        self.active.api_key.clone()
    }

    pub fn base_url(&self) -> Option<String> {
        self.active.base_url.clone()
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

    pub fn get_profile_web_search(&self) -> bool {
        if let Some(ref name) = self.active.profile_name {
            if let Some(profile) = self.profiles.get(name) {
                return profile.web_search.unwrap_or(false);
            }
        }
        false
    }

    pub fn get_profile_domain_filters(&self) -> (Option<Vec<String>>, Option<Vec<String>>) {
        if let Some(ref name) = self.active.profile_name {
            if let Some(profile) = self.profiles.get(name) {
                return (
                    profile.allowed_domains.clone(),
                    profile.blocked_domains.clone(),
                );
            }
        }
        (None, None)
    }

    pub fn get_thinking_level(&self) -> Option<String> {
        if let Some(ref name) = self.active.profile_name {
            if let Some(profile) = self.profiles.get(name) {
                return profile.thinking_level.clone();
            }
        }
        None
    }

    pub fn get_reasoning_effort(&self) -> Option<String> {
        if let Some(ref name) = self.active.profile_name {
            if let Some(profile) = self.profiles.get(name) {
                return profile.reasoning_effort.clone();
            }
        }
        None
    }

    pub fn get_thinking_budget(&self) -> Option<i64> {
        if let Some(ref name) = self.active.profile_name {
            if let Some(profile) = self.profiles.get(name) {
                return profile.thinking_budget;
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

/// Helper for numbered selection menus using requestty's raw_select
/// Items are displayed with number prefixes (1), 2), etc.) and can be selected by pressing the number key
fn numbered_select<T: ToString>(prompt: &str, items: &[T], default: usize) -> Result<usize> {
    let choices: Vec<String> = items.iter().map(|i| i.to_string()).collect();

    let question = Question::raw_select("menu")
        .message(prompt)
        .choices(choices)
        .default(default)
        .build();

    let answer = requestty::prompt_one(question)?;
    Ok(answer.as_list_item().unwrap().index)
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

/// Configure default provider and model (edits the default/first profile)
fn configure_defaults(mgr: &ConfigManager) -> Result<(String, String, String, bool, String, bool)> {
    let profiles = mgr.get_profiles();
    let default_profile = mgr
        .get_str(&["default_profile"])
        .or_else(|| profiles.first().cloned());

    let profile_name = default_profile.unwrap_or_else(|| "main".to_string());

    let existing_provider = mgr.get_str(&["profiles", &profile_name, "provider"]);
    let existing_model = mgr.get_str(&["profiles", &profile_name, "model"]);
    let existing_stream = mgr
        .get_str(&["profiles", &profile_name, "stream"])
        .map(|s| s == "true")
        .unwrap_or(true);
    let existing_web_search = mgr
        .get_str(&["profiles", &profile_name, "web_search"])
        .map(|s| s == "true")
        .unwrap_or(false);

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

    let model: String = {
        let question = Question::input("model")
            .message("Model")
            .default(model_default.as_str())
            .build();
        requestty::prompt_one(question)?
            .as_string()
            .unwrap_or_default()
            .to_string()
    };

    let existing_api_key = mgr
        .get_str(&["profiles", &profile_name, "api_key"])
        .unwrap_or_default();

    let api_key: String = if !existing_api_key.is_empty() {
        let masked = mask_api_key(&existing_api_key);
        let question = Question::input("api_key")
            .message(format!("{} API key [{}] (Enter to keep)", provider, masked))
            .build();
        let new_key = requestty::prompt_one(question)?
            .as_string()
            .unwrap_or_default()
            .to_string();

        if new_key.is_empty() {
            existing_api_key
        } else {
            new_key
        }
    } else {
        let question = Question::input("api_key")
            .message(format!("Enter {} API key", provider))
            .build();
        requestty::prompt_one(question)?
            .as_string()
            .unwrap_or_default()
            .to_string()
    };

    let stream = {
        let question = Question::confirm("stream")
            .message("Enable streaming responses?")
            .default(existing_stream)
            .build();
        requestty::prompt_one(question)?
            .as_bool()
            .unwrap_or(existing_stream)
    };

    let thinking_config = if let Some((key, value)) = select_thinking_config(provider, &model)? {
        format_thinking_config(&key, &value)
    } else {
        String::new()
    };

    let web_search = {
        let question = Question::confirm("web_search")
            .message("Enable web search by default?")
            .default(existing_web_search)
            .build();
        requestty::prompt_one(question)?
            .as_bool()
            .unwrap_or(existing_web_search)
    };

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
        let question = Question::input("profile_name")
            .message("Profile name (e.g., work, personal, local)")
            .build();
        requestty::prompt_one(question)?
            .as_string()
            .unwrap_or_default()
            .to_string()
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

    let model: String = {
        let question = Question::input("profile_model")
            .message("Model")
            .default(existing_model.as_str())
            .build();
        requestty::prompt_one(question)?
            .as_string()
            .unwrap_or_default()
            .to_string()
    };

    let existing_api_key = mgr
        .get_str(&["profiles", &name, "api_key"])
        .or_else(|| mgr.get_str(&["providers", provider, "api_key"]))
        .unwrap_or_default();

    let api_key: String = if !existing_api_key.is_empty() {
        let masked = mask_api_key(&existing_api_key);
        let question = Question::input("profile_api_key")
            .message(format!("API key [{}] (Enter to keep/inherit)", masked))
            .build();
        let new_key = requestty::prompt_one(question)?
            .as_string()
            .unwrap_or_default()
            .to_string();

        if new_key.is_empty() {
            String::new()
        } else {
            new_key
        }
    } else {
        let question = Question::input("profile_api_key")
            .message("API key (Enter to inherit from provider)")
            .build();
        requestty::prompt_one(question)?
            .as_string()
            .unwrap_or_default()
            .to_string()
    };

    let existing_base_url = mgr.get_str(&["profiles", &name, "base_url"]);
    let base_url: String = {
        let question = Question::input("profile_base_url")
            .message("Base URL (Enter for default, or custom like http://localhost:11434/v1)")
            .default(existing_base_url.as_deref().unwrap_or(""))
            .build();
        requestty::prompt_one(question)?
            .as_string()
            .unwrap_or_default()
            .to_string()
    };

    let existing_web_search = mgr.get_bool(&["profiles", &name, "web_search"], false);
    let web_search = {
        let question = Question::confirm("profile_web_search")
            .message("Enable web search for this profile?")
            .default(existing_web_search)
            .build();
        requestty::prompt_one(question)?
            .as_bool()
            .unwrap_or(existing_web_search)
    };

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
            let question = Question::input("fallback_profile")
                .message("Fallback profile name")
                .default(existing_fallback.as_deref().unwrap_or(""))
                .build();
            requestty::prompt_one(question)?
                .as_string()
                .unwrap_or_default()
                .to_string()
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

    let default_profile = mgr.get_str(&["default_profile"]);
    let profiles = mgr.get_profiles();

    if let Some(dp) = &default_profile {
        println!();
        println!(
            "{} {}",
            "default_profile =".yellow(),
            format!("\"{}\"", dp).cyan().bold()
        );
    } else if !profiles.is_empty() {
        println!();
        println!(
            "{}",
            "(First profile will be used by default)".bright_black()
        );
    }

    if !profiles.is_empty() {
        println!();
        println!("{}", "[profiles]".green().bold());
        for name in &profiles {
            let is_default = default_profile.as_ref().map(|d| d == name).unwrap_or(false)
                || (default_profile.is_none()
                    && profiles.first().map(|f| f == name).unwrap_or(false));

            let p_provider = mgr
                .get_str(&["profiles", name, "provider"])
                .unwrap_or_else(|| "gemini".to_string());
            let p_model = mgr
                .get_str(&["profiles", name, "model"])
                .unwrap_or_else(|| "default".to_string());
            let p_has_key = mgr.get_str(&["profiles", name, "api_key"]).is_some();
            let p_fallback = mgr.get_str(&["profiles", name, "fallback"]);
            let p_web_search = mgr
                .get_str(&["profiles", name, "web_search"])
                .map(|v| v == "true")
                .unwrap_or(false);

            let default_marker = if is_default {
                " (default)".green().bold().to_string()
            } else {
                String::new()
            };
            let key_indicator = if p_has_key {
                "✓".green().to_string()
            } else {
                "✗".red().to_string()
            };
            let web_indicator = if p_web_search {
                " [search]".cyan().to_string()
            } else {
                String::new()
            };
            let fallback_str = p_fallback
                .map(|f| format!(" (fallback: {})", f).bright_black().to_string())
                .unwrap_or_default();

            println!(
                "  {}{} {} {} {}{}{}",
                name.cyan().bold(),
                default_marker,
                p_provider.bright_white(),
                p_model.bright_black(),
                key_indicator,
                fallback_str,
                web_indicator
            );
        }
    } else {
        println!();
        println!("{}", "No profiles configured.".yellow());
        println!("Run 'ask init' to create a profile.");
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
                    let question = Question::confirm("delete_confirm")
                        .message(format!("Delete profile '{}'?", profile_name))
                        .default(false)
                        .build();
                    let confirm = requestty::prompt_one(question)?.as_bool().unwrap_or(false);

                    if confirm {
                        let content = std::fs::read_to_string(&mgr.config_path).unwrap_or_default();
                        let mut doc: toml::Value = toml::from_str(&content)?;
                        if let Some(profiles_table) = doc.get_mut("profiles") {
                            if let Some(table) = profiles_table.as_table_mut() {
                                table.remove(profile_name);
                            }
                        }
                        // If deleted profile was the default, remove default_profile setting
                        if let Some(current_default) = doc.get("default_profile") {
                            if current_default.as_str() == Some(profile_name) {
                                if let Some(table) = doc.as_table_mut() {
                                    table.remove("default_profile");
                                }
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

                let current_default = mgr.get_str(&["default_profile"]);
                let default_idx = current_default
                    .as_ref()
                    .and_then(|d| profiles.iter().position(|p| p == d))
                    .unwrap_or(0);

                let mut items: Vec<String> = profiles.clone();
                items.push("Use first profile (clear setting)".to_string());

                let idx = numbered_select("Select default profile", &items, default_idx)?;

                let content = std::fs::read_to_string(&mgr.config_path).unwrap_or_default();
                let mut doc: toml::Value = toml::from_str(&content)?;

                if idx < profiles.len() {
                    let profile_name = &profiles[idx];
                    if let Some(table) = doc.as_table_mut() {
                        table.insert(
                            "default_profile".to_string(),
                            toml::Value::String(profile_name.clone()),
                        );
                    }
                    std::fs::write(&mgr.config_path, toml::to_string_pretty(&doc)?)?;
                    mgr.reload()?;
                    println!(
                        "{} {}",
                        "Default profile set to:".green(),
                        profile_name.cyan()
                    );
                } else {
                    if let Some(table) = doc.as_table_mut() {
                        table.remove("default_profile");
                    }
                    std::fs::write(&mgr.config_path, toml::to_string_pretty(&doc)?)?;
                    mgr.reload()?;
                    println!(
                        "{}",
                        "Default profile cleared (first profile will be used)".green()
                    );
                }
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

# All configuration lives in profiles
# First profile is used by default (set default_profile to change)
# Switch profiles with: ask -p <profile_name>
[profiles.main]
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
                    println!("Profile '{}' created!", "main".cyan());
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

                    let profiles = mgr.get_profiles();
                    let default_profile_name = mgr
                        .get_str(&["default_profile"])
                        .or_else(|| profiles.first().cloned())
                        .unwrap_or_else(|| "main".to_string());

                    let content = std::fs::read_to_string(&mgr.config_path).unwrap_or_default();
                    let mut doc: toml::Value = toml::from_str(&content)?;

                    let profiles_section = doc
                        .as_table_mut()
                        .unwrap()
                        .entry("profiles".to_string())
                        .or_insert(toml::Value::Table(toml::map::Map::new()));

                    if let Some(profiles_table) = profiles_section.as_table_mut() {
                        let profile_table = profiles_table
                            .entry(default_profile_name.clone())
                            .or_insert(toml::Value::Table(toml::map::Map::new()));

                        if let Some(pt) = profile_table.as_table_mut() {
                            pt.insert("provider".to_string(), toml::Value::String(provider));
                            pt.insert("model".to_string(), toml::Value::String(model));
                            pt.insert("api_key".to_string(), toml::Value::String(api_key));
                            pt.insert("stream".to_string(), toml::Value::Boolean(stream));
                            pt.insert("web_search".to_string(), toml::Value::Boolean(web_search));

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

                    std::fs::write(&mgr.config_path, toml::to_string_pretty(&doc)?)?;
                    mgr.reload()?;
                    println!(
                        "{} {}",
                        "Profile".green(),
                        format!("'{}' updated!", default_profile_name).green()
                    );
                }
                2 => {
                    mgr.backup()?;

                    let profiles = mgr.get_profiles();
                    if profiles.is_empty() {
                        println!(
                            "{}",
                            "No profiles configured. Create a profile first.".yellow()
                        );
                        continue;
                    }

                    let mut items: Vec<String> = profiles.clone();
                    items.push("Cancel".to_string());

                    let idx = numbered_select("Select profile to update API key", &items, 0)?;

                    if idx < profiles.len() {
                        let profile_name = &profiles[idx];
                        let existing_key = mgr
                            .get_str(&["profiles", profile_name, "api_key"])
                            .unwrap_or_default();

                        let new_key: String = if !existing_key.is_empty() {
                            let masked = mask_api_key(&existing_key);
                            let question = Question::input("profile_api_key")
                                .message(format!("API key [{}] (Enter to keep)", masked))
                                .build();
                            requestty::prompt_one(question)?
                                .as_string()
                                .unwrap_or_default()
                                .to_string()
                        } else {
                            let question = Question::input("profile_api_key")
                                .message("API key")
                                .build();
                            requestty::prompt_one(question)?
                                .as_string()
                                .unwrap_or_default()
                                .to_string()
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

                            if let Some(profiles_section) = doc.get_mut("profiles") {
                                if let Some(table) = profiles_section.as_table_mut() {
                                    if let Some(profile_table) = table.get_mut(profile_name) {
                                        if let Some(pt) = profile_table.as_table_mut() {
                                            pt.insert(
                                                "api_key".to_string(),
                                                toml::Value::String(final_key),
                                            );
                                        }
                                    }
                                }
                            }

                            std::fs::write(&mgr.config_path, toml::to_string_pretty(&doc)?)?;
                            mgr.reload()?;
                            println!(
                                "{} {}",
                                "API key updated for profile".green(),
                                profile_name.cyan()
                            );
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
    fn test_cli_overrides_precedence() {
        let mut config = Config::default();
        config.profiles.insert(
            "work".to_string(),
            ProfileConfig {
                provider: Some("openai".to_string()),
                model: Some("gpt-4".to_string()),
                api_key: Some("test-key".to_string()),
                ..Default::default()
            },
        );

        let args = Args {
            profile: Some("work".to_string()),
            ..Default::default()
        };
        let cfg = config.clone().with_cli_overrides(&args);
        assert_eq!(cfg.active_provider(), "openai");
        assert_eq!(cfg.active_model(), "gpt-4");

        let args_model = Args {
            profile: Some("work".to_string()),
            model: Some("claude-3".to_string()),
            ..Default::default()
        };
        let cfg2 = config.clone().with_cli_overrides(&args_model);
        assert_eq!(cfg2.active_provider(), "openai");
        assert_eq!(cfg2.active_model(), "claude-3");
    }

    #[test]
    fn test_thinking_config_logic() {
        let mut config = Config::default();
        config.profiles.insert(
            "thinker".to_string(),
            ProfileConfig {
                provider: Some("gemini".to_string()),
                thinking_level: Some("high".to_string()),
                ..Default::default()
            },
        );

        let args = Args {
            profile: Some("thinker".to_string()),
            ..Default::default()
        };
        let cfg = config.with_cli_overrides(&args);
        let (enabled, value) = cfg.get_thinking_config();
        assert!(enabled);
        assert_eq!(value, Some("high".to_string()));
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

        assert_eq!(config.fallback_profile("p1"), Some("p2".to_string()));
        assert_eq!(config.fallback_profile("p2"), None);

        let fallback_any = config.fallback_profile("p3");
        assert!(fallback_any.is_some());
        assert_ne!(fallback_any.unwrap(), "p3");
    }
}
