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
}
