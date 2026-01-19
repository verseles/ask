//! Provider integrations for various AI APIs

mod anthropic;
mod gemini;
mod openai;
mod traits;

pub use anthropic::AnthropicProvider;
pub use gemini::GeminiProvider;
pub use openai::OpenAIProvider;
pub use traits::*;

use crate::config::Config;
use anyhow::{anyhow, Result};

/// Flattens a command that might contain accidental newlines.
///
/// If the text looks like a command but has newlines, it attempts to
/// join them with ' && ' if they are sequential commands, or just
/// remove the newline if it was a wrapping issue.
pub fn flatten_command(text: &str) -> String {
    let trimmed = text.trim();
    if !trimmed.contains('\n') {
        return trimmed.to_string();
    }

    // Split by lines and filter empty ones
    let lines: Vec<&str> = trimmed
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();

    if lines.len() <= 1 {
        return trimmed.replace('\n', " ").trim().to_string();
    }

    // Join with && to ensure sequential execution of multi-line outputs.
    // This is compatible with sh, bash, zsh, fish, and cmd.exe.
    lines.join(" && ")
}

/// Create a provider based on configuration
pub fn create_provider(config: &Config) -> Result<Box<dyn Provider>> {
    let provider_name = config.active_provider();
    let model = config.active_model().to_string();

    let api_key = config.api_key().ok_or_else(|| {
        anyhow!(
            "No API key found for provider '{}'. Run 'ask init' to configure.",
            provider_name
        )
    })?;

    match provider_name {
        "gemini" => {
            let base_url = config
                .base_url()
                .unwrap_or_else(|| crate::config::DEFAULT_GEMINI_BASE_URL.to_string());
            Ok(Box::new(GeminiProvider::new(api_key, base_url, model)))
        }
        "openai" | "openai_compatible" => {
            let base_url = config
                .base_url()
                .unwrap_or_else(|| crate::config::DEFAULT_OPENAI_BASE_URL.to_string());
            Ok(Box::new(OpenAIProvider::new(api_key, base_url, model)))
        }
        "anthropic" | "claude" => {
            let base_url = config
                .base_url()
                .unwrap_or_else(|| crate::config::DEFAULT_ANTHROPIC_BASE_URL.to_string());
            Ok(Box::new(AnthropicProvider::new(api_key, base_url, model)))
        }
        _ => Err(anyhow!("Unknown provider: {}", provider_name)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flatten_command() {
        // Single line remains unchanged
        assert_eq!(flatten_command("ls -la"), "ls -la");

        // Multi-line command joined with &&
        assert_eq!(
            flatten_command("mkdir test\ncd test\ntouch hello.txt"),
            "mkdir test && cd test && touch hello.txt"
        );

        // Extra whitespace and empty lines handled
        assert_eq!(
            flatten_command("  apt update  \n\n  apt upgrade  "),
            "apt update && apt upgrade"
        );
    }
}
