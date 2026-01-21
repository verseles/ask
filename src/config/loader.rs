//! Configuration loader - handles TOML config hierarchy

use super::Config;
use anyhow::Result;
use std::path::PathBuf;

impl Config {
    /// Load only aliases from config (fast, for early argument expansion)
    pub fn load_aliases_only() -> std::collections::HashMap<String, String> {
        if let Some(path) = Self::find_local_config()
            .or_else(Self::find_home_config)
            .or_else(Self::find_xdg_config)
        {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(config) = toml::from_str::<Config>(&content) {
                    return config.aliases;
                }
            }
        }
        std::collections::HashMap::new()
    }

    /// Load configuration with precedence:
    /// 1. CLI arguments (handled separately via with_cli_overrides)
    /// 2. Environment variables (handled separately)
    /// 3. ./ask.toml or ./.ask.toml (project local)
    /// 4. ~/ask.toml (home directory)
    /// 5. ~/.config/ask/ask.toml (XDG config)
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
    /// On Linux/macOS: ~/.config/ask/ask.toml
    /// On Windows: C:\Users\<user>\AppData\Roaming\ask\ask.toml
    fn find_xdg_config() -> Option<PathBuf> {
        #[cfg(windows)]
        {
            // Windows: use AppData\Roaming
            if let Some(config_dir) = dirs::config_dir() {
                let path = config_dir.join("ask").join("ask.toml");
                if path.exists() {
                    return Some(path);
                }
            }
        }

        #[cfg(not(windows))]
        {
            // Linux/macOS: use ~/.config for Unix consistency
            if let Some(home) = dirs::home_dir() {
                let path = home.join(".config").join("ask").join("ask.toml");
                if path.exists() {
                    return Some(path);
                }
            }
        }

        None
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

    /// Find project local config by searching upwards from current directory
    fn find_local_config() -> Option<PathBuf> {
        find_recursive_file(&["ask.toml", ".ask.toml"])
    }

    /// Load config from a specific file
    fn load_from_file(path: &PathBuf) -> Result<Config> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    fn merge(base: Config, overlay: Config) -> Config {
        Config {
            profiles: {
                let mut profiles = base.profiles;
                for (k, v) in overlay.profiles {
                    profiles.insert(k, v);
                }
                profiles
            },
            default_profile: overlay.default_profile.or(base.default_profile),
            behavior: overlay.behavior,
            context: overlay.context,
            update: overlay.update,
            commands: {
                let mut commands = base.commands;
                for (k, v) in overlay.commands {
                    commands.insert(k, v);
                }
                commands
            },
            aliases: {
                let mut aliases = base.aliases;
                for (k, v) in overlay.aliases {
                    aliases.insert(k, v);
                }
                aliases
            },
            active: Default::default(),
        }
    }

    fn apply_env_overrides(mut config: Config) -> Config {
        if let Ok(val) = std::env::var("ASK_AUTO_EXECUTE") {
            config.behavior.auto_execute = parse_bool(&val);
        }
        if let Ok(val) = std::env::var("ASK_CONFIRM_DESTRUCTIVE") {
            config.behavior.confirm_destructive = parse_bool(&val);
        }
        if let Ok(val) = std::env::var("ASK_TIMEOUT") {
            if let Ok(timeout) = val.parse() {
                config.behavior.timeout = timeout;
            }
        }

        if let Ok(val) = std::env::var("ASK_CONTEXT_MAX_AGE") {
            if let Ok(age) = val.parse() {
                config.context.max_age_minutes = age;
            }
        }
        if let Ok(val) = std::env::var("ASK_CONTEXT_MAX_MESSAGES") {
            if let Ok(max) = val.parse() {
                config.context.max_messages = max;
            }
        }
        if let Ok(path) = std::env::var("ASK_CONTEXT_PATH") {
            config.context.storage_path = Some(path);
        }

        if let Ok(val) = std::env::var("ASK_UPDATE_AUTO_CHECK") {
            config.update.auto_check = parse_bool(&val);
        }
        if let Ok(val) = std::env::var("ASK_UPDATE_INTERVAL") {
            if let Ok(hours) = val.parse() {
                config.update.check_interval_hours = hours;
            }
        }
        if let Ok(channel) = std::env::var("ASK_UPDATE_CHANNEL") {
            config.update.channel = channel;
        }
        if let Ok(val) = std::env::var("ASK_UPDATE_AGGRESSIVE") {
            config.update.aggressive = parse_bool(&val);
        }

        config
    }
}

/// Find a file by searching upwards from current directory
pub fn find_recursive_file(names: &[&str]) -> Option<PathBuf> {
    let cwd = std::env::current_dir().ok()?;
    let mut current = cwd.as_path();

    loop {
        for name in names {
            let path = current.join(name);
            if path.exists() {
                return Some(path);
            }
        }

        // Move to parent directory
        match current.parent() {
            Some(parent) => current = parent,
            None => break,
        }
    }

    None
}

fn parse_bool(s: &str) -> bool {
    matches!(s.to_lowercase().as_str(), "true" | "1" | "yes" | "on")
}

impl Config {
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
        assert!(config.profiles.is_empty());
        assert!(config.default_profile.is_none());
        assert!(!config.behavior.auto_execute);
        assert!(config.behavior.confirm_destructive);
        assert_eq!(config.behavior.timeout, 30);
        assert_eq!(config.context.max_age_minutes, 30);
        assert_eq!(config.context.max_messages, 20);
        assert!(config.update.auto_check);
        assert_eq!(config.update.check_interval_hours, 24);
    }

    #[test]
    fn test_parse_profile_config() {
        let toml = r#"
[profiles.main]
provider = "openai"
model = "gpt-5"
api_key = "sk-test"
"#;
        let config = Config::from_toml(toml).unwrap();
        assert_eq!(config.profiles.len(), 1);
        let main = config.profiles.get("main").unwrap();
        assert_eq!(main.provider.as_deref(), Some("openai"));
        assert_eq!(main.model.as_deref(), Some("gpt-5"));
        assert_eq!(main.api_key.as_deref(), Some("sk-test"));
    }

    #[test]
    fn test_parse_full_config() {
        let toml = r#"
[profiles.work]
provider = "anthropic"
model = "claude-3-opus"
api_key = "test-key"
stream = false

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
        let work = config.profiles.get("work").unwrap();
        assert_eq!(work.provider.as_deref(), Some("anthropic"));
        assert_eq!(work.model.as_deref(), Some("claude-3-opus"));
        assert_eq!(work.stream, Some(false));
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
    fn test_parse_profiles() {
        let toml = r#"
default_profile = "work"

[profiles.work]
provider = "openai"
model = "gpt-5"
api_key = "sk-work-key"

[profiles.personal]
provider = "anthropic"
model = "claude-haiku-4-5"
base_url = "https://custom.anthropic.com"
fallback = "work"

[profiles.local]
provider = "openai"
base_url = "http://localhost:11434/v1"
model = "llama3"
api_key = "ollama"
fallback = "none"
"#;
        let config = Config::from_toml(toml).unwrap();

        assert_eq!(config.default_profile.as_deref(), Some("work"));
        assert_eq!(config.profiles.len(), 3);

        let work = config.profiles.get("work").unwrap();
        assert_eq!(work.provider.as_deref(), Some("openai"));
        assert_eq!(work.model.as_deref(), Some("gpt-5"));
        assert_eq!(work.api_key.as_deref(), Some("sk-work-key"));

        let personal = config.profiles.get("personal").unwrap();
        assert_eq!(personal.provider.as_deref(), Some("anthropic"));
        assert_eq!(personal.fallback.as_deref(), Some("work"));

        let local = config.profiles.get("local").unwrap();
        assert_eq!(local.base_url.as_deref(), Some("http://localhost:11434/v1"));
        assert_eq!(local.fallback.as_deref(), Some("none"));
    }

    #[test]
    fn test_merge_profiles() {
        let base_toml = r#"
[profiles.work]
provider = "gemini"
model = "gemini-flash"
"#;
        let overlay_toml = r#"
[profiles.work]
model = "gemini-pro"

[profiles.personal]
provider = "anthropic"
"#;
        let base = Config::from_toml(base_toml).unwrap();
        let overlay = Config::from_toml(overlay_toml).unwrap();
        let merged = Config::merge(base, overlay);

        assert_eq!(merged.profiles.len(), 2);

        let work = merged.profiles.get("work").unwrap();
        assert_eq!(work.model.as_deref(), Some("gemini-pro"));

        let personal = merged.profiles.get("personal").unwrap();
        assert_eq!(personal.provider.as_deref(), Some("anthropic"));
    }

    #[test]
    fn test_profile_with_cli_overrides() {
        use crate::cli::Args;

        let toml = r#"
[profiles.work]
provider = "openai"
model = "gpt-5"
api_key = "profile-key"
"#;
        let config = Config::from_toml(toml).unwrap();
        let args = Args {
            profile: Some("work".to_string()),
            ..Default::default()
        };
        let applied = config.with_cli_overrides(&args);

        assert_eq!(applied.active_provider(), "openai");
        assert_eq!(applied.active_model(), "gpt-5");
        assert_eq!(applied.api_key(), Some("profile-key".to_string()));
    }

    #[test]
    fn test_default_profile_selection() {
        use crate::cli::Args;

        let toml = r#"
default_profile = "work"

[profiles.work]
provider = "openai"
model = "gpt-5"

[profiles.personal]
provider = "anthropic"
"#;
        let config = Config::from_toml(toml).unwrap();
        let args = Args::default();
        let applied = config.with_cli_overrides(&args);

        assert_eq!(applied.active_provider(), "openai");
        assert_eq!(applied.active_model(), "gpt-5");
    }

    #[test]
    fn test_cli_model_override() {
        use crate::cli::Args;

        let toml = r#"
[profiles.work]
provider = "openai"
model = "gpt-5"
"#;
        let config = Config::from_toml(toml).unwrap();
        let args = Args {
            profile: Some("work".to_string()),
            model: Some("gpt-4".to_string()),
            ..Default::default()
        };
        let applied = config.with_cli_overrides(&args);

        assert_eq!(applied.active_provider(), "openai");
        assert_eq!(applied.active_model(), "gpt-4");
    }

    #[test]
    fn test_fallback_profile_none() {
        let toml = r#"
[profiles.work]
provider = "openai"
fallback = "none"

[profiles.personal]
provider = "anthropic"
"#;
        let config = Config::from_toml(toml).unwrap();
        assert!(config.fallback_profile("work").is_none());
    }

    #[test]
    fn test_fallback_profile_specific() {
        let toml = r#"
[profiles.work]
provider = "openai"
fallback = "personal"

[profiles.personal]
provider = "anthropic"
"#;
        let config = Config::from_toml(toml).unwrap();
        assert_eq!(
            config.fallback_profile("work"),
            Some("personal".to_string())
        );
    }

    #[test]
    fn test_fallback_profile_any() {
        let toml = r#"
[profiles.work]
provider = "openai"
fallback = "any"

[profiles.personal]
provider = "anthropic"
"#;
        let config = Config::from_toml(toml).unwrap();
        let fallback = config.fallback_profile("work");
        assert!(fallback.is_some());
        assert_ne!(fallback.as_deref(), Some("work"));
    }

    #[test]
    fn test_first_profile_as_default() {
        use crate::cli::Args;

        let toml = r#"
[profiles.only_one]
provider = "gemini"
model = "gemini-flash"
api_key = "test-key"
"#;
        let config = Config::from_toml(toml).unwrap();
        let args = Args::default();
        let applied = config.with_cli_overrides(&args);

        assert_eq!(applied.active_provider(), "gemini");
        assert_eq!(applied.active_model(), "gemini-flash");
    }

    #[test]
    fn test_ad_hoc_mode() {
        use crate::cli::Args;

        let toml = r#"
[profiles.work]
provider = "openai"
model = "gpt-5"
"#;
        let config = Config::from_toml(toml).unwrap();
        let args = Args {
            provider: Some("anthropic".to_string()),
            api_key: Some("ad-hoc-key".to_string()),
            model: Some("claude-3".to_string()),
            ..Default::default()
        };
        let applied = config.with_cli_overrides(&args);

        assert_eq!(applied.active_provider(), "anthropic");
        assert_eq!(applied.active_model(), "claude-3");
        assert_eq!(applied.api_key(), Some("ad-hoc-key".to_string()));
        assert!(applied.active.profile_name.is_none());
    }
}
