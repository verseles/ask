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

/// Create a provider based on configuration
pub fn create_provider(config: &Config) -> Result<Box<dyn Provider>> {
    let provider_name = config.active_provider();
    let model = config.active_model().to_string();

    let api_key = config
        .api_key()
        .ok_or_else(|| anyhow!("No API key found for provider '{}'. Run 'ask init' to configure.", provider_name))?;

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
