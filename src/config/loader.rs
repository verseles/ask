//! Configuration loader - handles TOML config hierarchy

use super::Config;
use anyhow::Result;
use std::path::PathBuf;

impl Config {
    /// Load configuration with precedence:
    /// 1. CLI arguments (handled separately via with_cli_overrides)
    /// 2. Environment variables (handled separately)
    /// 3. ./ask.toml or ./.ask.toml (project local)
    /// 4. ~/ask.toml (home directory)
    /// 5. ~/.config/ask/config.toml (XDG config)
    /// 6. Defaults (hardcoded)
    pub fn load() -> Result<Self> {
        let mut config = Config::default();

        // Load in reverse precedence order (lowest first, higher overwrites)

        // XDG config
        if let Some(xdg_config) = Self::find_xdg_config() {
            if let Ok(loaded) = Self::load_from_file(&xdg_config) {
                config = Self::merge(config, loaded);
            }
        }

        // Home directory config
        if let Some(home_config) = Self::find_home_config() {
            if let Ok(loaded) = Self::load_from_file(&home_config) {
                config = Self::merge(config, loaded);
            }
        }

        // Project local config
        if let Some(local_config) = Self::find_local_config() {
            if let Ok(loaded) = Self::load_from_file(&local_config) {
                config = Self::merge(config, loaded);
            }
        }

        // Apply environment variable overrides
        config = Self::apply_env_overrides(config);

        Ok(config)
    }

    /// Find XDG config file
    fn find_xdg_config() -> Option<PathBuf> {
        let config_dir = dirs::config_dir()?;
        let path = config_dir.join("ask").join("config.toml");
        if path.exists() {
            Some(path)
        } else {
            None
        }
    }

    /// Find home directory config
    fn find_home_config() -> Option<PathBuf> {
        let home = dirs::home_dir()?;
        let path = home.join("ask.toml");
        if path.exists() {
            Some(path)
        } else {
            None
        }
    }

    /// Find project local config
    fn find_local_config() -> Option<PathBuf> {
        let cwd = std::env::current_dir().ok()?;

        // Try ask.toml first
        let path = cwd.join("ask.toml");
        if path.exists() {
            return Some(path);
        }

        // Try .ask.toml
        let path = cwd.join(".ask.toml");
        if path.exists() {
            return Some(path);
        }

        None
    }

    /// Load config from a specific file
    fn load_from_file(path: &PathBuf) -> Result<Config> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    /// Merge two configs (second takes precedence)
    fn merge(base: Config, overlay: Config) -> Config {
        Config {
            default: super::DefaultConfig {
                provider: if overlay.default.provider != "gemini" {
                    overlay.default.provider
                } else {
                    base.default.provider
                },
                model: if overlay.default.model != "gemini-2.0-flash" {
                    overlay.default.model
                } else {
                    base.default.model
                },
                stream: overlay.default.stream,
            },
            providers: {
                let mut providers = base.providers;
                for (k, v) in overlay.providers {
                    providers.insert(k, v);
                }
                providers
            },
            behavior: super::BehaviorConfig {
                auto_execute: overlay.behavior.auto_execute || base.behavior.auto_execute,
                confirm_destructive: overlay.behavior.confirm_destructive,
                timeout: if overlay.behavior.timeout != 30 {
                    overlay.behavior.timeout
                } else {
                    base.behavior.timeout
                },
            },
            context: overlay.context,
            update: overlay.update,
            commands: {
                let mut commands = base.commands;
                for (k, v) in overlay.commands {
                    commands.insert(k, v);
                }
                commands
            },
        }
    }

    /// Apply environment variable overrides
    fn apply_env_overrides(mut config: Config) -> Config {
        // Provider override
        if let Ok(provider) = std::env::var("ASK_PROVIDER") {
            config.default.provider = provider;
        }

        // Model override
        if let Ok(model) = std::env::var("ASK_MODEL") {
            config.default.model = model;
        }

        // Stream override
        if let Ok(stream) = std::env::var("ASK_STREAM") {
            config.default.stream = stream.to_lowercase() == "true" || stream == "1";
        }

        config
    }

    /// Load config from a TOML string (for testing)
    #[cfg(test)]
    pub fn from_toml(content: &str) -> Result<Config> {
        let config: Config = toml::from_str(content)?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.default.provider, "gemini");
        assert_eq!(config.default.model, "gemini-3-flash-preview");
        assert!(config.default.stream);
        assert!(!config.behavior.auto_execute);
        assert!(config.behavior.confirm_destructive);
        assert_eq!(config.behavior.timeout, 30);
        assert_eq!(config.context.max_age_minutes, 30);
        assert_eq!(config.context.max_messages, 20);
        assert!(config.update.auto_check);
        assert_eq!(config.update.check_interval_hours, 24);
    }

    #[test]
    fn test_parse_minimal_config() {
        let toml = r#"
[default]
provider = "openai"
"#;
        let config = Config::from_toml(toml).unwrap();
        assert_eq!(config.default.provider, "openai");
        // Model should use default since not specified
        assert_eq!(config.default.model, "gemini-3-flash-preview");
    }

    #[test]
    fn test_parse_full_config() {
        let toml = r#"
[default]
provider = "anthropic"
model = "claude-3-opus"
stream = false

[providers.anthropic]
api_key = "test-key"

[behavior]
auto_execute = true
confirm_destructive = false
timeout = 60

[context]
max_age_minutes = 60
max_messages = 50

[update]
auto_check = false
check_interval_hours = 48
channel = "beta"

[commands.cm]
system = "Generate commit message"
type = "command"
auto_execute = false
"#;
        let config = Config::from_toml(toml).unwrap();
        assert_eq!(config.default.provider, "anthropic");
        assert_eq!(config.default.model, "claude-3-opus");
        assert!(!config.default.stream);
        assert!(config.behavior.auto_execute);
        assert!(!config.behavior.confirm_destructive);
        assert_eq!(config.behavior.timeout, 60);
        assert_eq!(config.context.max_age_minutes, 60);
        assert_eq!(config.context.max_messages, 50);
        assert!(!config.update.auto_check);
        assert_eq!(config.update.check_interval_hours, 48);
        assert_eq!(config.update.channel, "beta");

        let cmd = config.commands.get("cm").unwrap();
        assert_eq!(cmd.system, "Generate commit message");
        assert_eq!(cmd.r#type.as_deref(), Some("command"));
        assert_eq!(cmd.auto_execute, Some(false));
    }

    #[test]
    fn test_parse_provider_config() {
        let toml = r#"
[providers.gemini]
api_key = "gemini-key"

[providers.openai]
api_key = "openai-key"
base_url = "https://custom.openai.com"
model = "gpt-4"

[providers.anthropic]
api_key = "anthropic-key"
"#;
        let config = Config::from_toml(toml).unwrap();

        let gemini = config.providers.get("gemini").unwrap();
        assert_eq!(gemini.api_key.as_deref(), Some("gemini-key"));

        let openai = config.providers.get("openai").unwrap();
        assert_eq!(openai.api_key.as_deref(), Some("openai-key"));
        assert_eq!(
            openai.base_url.as_deref(),
            Some("https://custom.openai.com")
        );
        assert_eq!(openai.model.as_deref(), Some("gpt-4"));

        let anthropic = config.providers.get("anthropic").unwrap();
        assert_eq!(anthropic.api_key.as_deref(), Some("anthropic-key"));
    }

    #[test]
    fn test_merge_configs() {
        let base = Config::default();
        let overlay_toml = r#"
[default]
provider = "openai"
model = "gpt-5"

[behavior]
timeout = 120
"#;
        let overlay = Config::from_toml(overlay_toml).unwrap();
        let merged = Config::merge(base, overlay);

        assert_eq!(merged.default.provider, "openai");
        assert_eq!(merged.default.model, "gpt-5");
        assert_eq!(merged.behavior.timeout, 120);
        // Base defaults should be preserved
        assert!(!merged.behavior.auto_execute);
    }

    #[test]
    fn test_custom_commands() {
        let toml = r#"
[commands.explain]
system = "Explain this code in detail"
inherit_flags = true

[commands.fix]
system = "Fix any issues in this code"
type = "code"
provider = "anthropic"
model = "claude-3-opus"
"#;
        let config = Config::from_toml(toml).unwrap();

        let explain = config.commands.get("explain").unwrap();
        assert_eq!(explain.system, "Explain this code in detail");
        assert!(explain.inherit_flags);

        let fix = config.commands.get("fix").unwrap();
        assert_eq!(fix.system, "Fix any issues in this code");
        assert_eq!(fix.r#type.as_deref(), Some("code"));
        assert_eq!(fix.provider.as_deref(), Some("anthropic"));
        assert_eq!(fix.model.as_deref(), Some("claude-3-opus"));
    }

    #[test]
    fn test_api_key_retrieval() {
        let toml = r#"
[default]
provider = "gemini"

[providers.gemini]
api_key = "my-gemini-key"
"#;
        let config = Config::from_toml(toml).unwrap();
        assert_eq!(config.api_key(), Some("my-gemini-key".to_string()));
    }

    #[test]
    fn test_active_provider_and_model() {
        let toml = r#"
[default]
provider = "openai"
model = "gpt-4-turbo"
"#;
        let config = Config::from_toml(toml).unwrap();
        assert_eq!(config.active_provider(), "openai");
        assert_eq!(config.active_model(), "gpt-4-turbo");
    }
}
