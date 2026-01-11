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
    /// On Linux: ~/.config/ask/config.toml
    /// On macOS: ~/Library/Application Support/ask/config.toml OR ~/.config/ask/config.toml
    /// On Windows: C:\Users\<user>\AppData\Roaming\ask\config.toml
    fn find_xdg_config() -> Option<PathBuf> {
        // First try the platform-specific config dir
        if let Some(config_dir) = dirs::config_dir() {
            let path = config_dir.join("ask").join("config.toml");
            if path.exists() {
                return Some(path);
            }
        }

        // On macOS, also check ~/.config/ for Unix compatibility
        #[cfg(target_os = "macos")]
        {
            if let Some(home) = dirs::home_dir() {
                let path = home.join(".config").join("ask").join("config.toml");
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

    /// Merge two configs (overlay takes precedence)
    /// For scalar values: overlay wins
    /// For collections (providers, commands): merge with overlay winning conflicts
    fn merge(base: Config, overlay: Config) -> Config {
        Config {
            // Overlay always wins for default settings
            default: overlay.default,
            // Merge providers: base + overlay, overlay wins conflicts
            providers: {
                let mut providers = base.providers;
                for (k, v) in overlay.providers {
                    providers.insert(k, v);
                }
                providers
            },
            // Overlay wins for behavior
            behavior: overlay.behavior,
            // Overlay wins for context
            context: overlay.context,
            // Overlay wins for update
            update: overlay.update,
            // Merge commands: base + overlay, overlay wins conflicts
            commands: {
                let mut commands = base.commands;
                for (k, v) in overlay.commands {
                    commands.insert(k, v);
                }
                commands
            },
            // Merge profiles: base + overlay, overlay wins conflicts
            profiles: {
                let mut profiles = base.profiles;
                for (k, v) in overlay.profiles {
                    profiles.insert(k, v);
                }
                profiles
            },
            // Overlay wins for default_profile
            default_profile: overlay.default_profile.or(base.default_profile),
            // Merge aliases: base + overlay, overlay wins conflicts
            aliases: {
                let mut aliases = base.aliases;
                for (k, v) in overlay.aliases {
                    aliases.insert(k, v);
                }
                aliases
            },
        }
    }

    /// Apply environment variable overrides
    fn apply_env_overrides(mut config: Config) -> Config {
        // === Default settings ===
        if let Ok(provider) = std::env::var("ASK_PROVIDER") {
            config.default.provider = provider;
        }
        if let Ok(model) = std::env::var("ASK_MODEL") {
            config.default.model = model;
        }
        if let Ok(stream) = std::env::var("ASK_STREAM") {
            config.default.stream = parse_bool(&stream);
        }

        // === Provider settings ===
        // Base URLs
        if let Ok(url) = std::env::var("ASK_GEMINI_BASE_URL") {
            config
                .providers
                .entry("gemini".to_string())
                .or_default()
                .base_url = Some(url);
        }
        if let Ok(url) = std::env::var("ASK_OPENAI_BASE_URL") {
            config
                .providers
                .entry("openai".to_string())
                .or_default()
                .base_url = Some(url);
        }
        if let Ok(url) = std::env::var("ASK_ANTHROPIC_BASE_URL") {
            config
                .providers
                .entry("anthropic".to_string())
                .or_default()
                .base_url = Some(url);
        }

        // === Behavior settings ===
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

        // === Context settings ===
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

        // === Update settings ===
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

        config
    }
}

/// Parse boolean from string (true/false/1/0/yes/no)
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
    fn test_profile_inheritance() {
        use crate::cli::Args;

        let toml = r#"
[default]
provider = "gemini"
model = "gemini-flash"

[providers.openai]
api_key = "base-key"

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
        assert_eq!(
            applied.providers.get("openai").unwrap().api_key.as_deref(),
            Some("profile-key")
        );
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
    fn test_cli_overrides_profile() {
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
    fn test_fallback_profile_default() {
        let toml = r#"
[profiles.work]
provider = "openai"

[profiles.personal]
provider = "anthropic"
"#;
        let config = Config::from_toml(toml).unwrap();
        let fallback = config.fallback_profile("work");
        assert!(fallback.is_some());
        assert_ne!(fallback.as_deref(), Some("work"));
    }
}
