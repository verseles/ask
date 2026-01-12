//! Thinking mode configuration helpers
//!
//! Different providers use different parameters:
//! - Gemini 2.5: thinkingBudget (0, 1024-32768, -1 for dynamic)
//! - Gemini 3: thinkingLevel (minimal, low, medium, high)
//! - OpenAI: reasoning_effort (none, minimal, low, medium, high, xhigh)
//! - Anthropic: thinking_budget (0, 1024-128000)

use anyhow::Result;
use requestty::Question;

use super::numbered_select;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ThinkingType {
    GeminiBudget,
    GeminiLevel,
    OpenAIEffort,
    AnthropicBudget,
    NotSupported,
}

pub fn detect_thinking_type(provider: &str, model: &str) -> ThinkingType {
    match provider {
        "gemini" => {
            let model_lower = model.to_lowercase();
            if model_lower.contains("gemini-3")
                || model_lower.contains("gemini-3-flash")
                || model_lower.contains("gemini-3-pro")
            {
                ThinkingType::GeminiLevel
            } else if model_lower.contains("2.5")
                || model_lower.contains("2-5")
                || model_lower.ends_with("-latest")
                || model_lower.contains("flash-lite")
                || model_lower.contains("flash")
                || model_lower.contains("pro")
            {
                ThinkingType::GeminiBudget
            } else {
                ThinkingType::NotSupported
            }
        }
        "openai" => {
            let model_lower = model.to_lowercase();
            if model_lower.starts_with("o1")
                || model_lower.starts_with("o3")
                || model_lower.starts_with("o4")
                || model_lower.contains("gpt-5")
            {
                ThinkingType::OpenAIEffort
            } else {
                ThinkingType::NotSupported
            }
        }
        "anthropic" => ThinkingType::AnthropicBudget,
        _ => ThinkingType::NotSupported,
    }
}

pub struct ThinkingOption {
    pub label: String,
    pub config_value: String,
    pub config_key: &'static str,
}

pub fn get_thinking_options(thinking_type: ThinkingType) -> Vec<ThinkingOption> {
    match thinking_type {
        ThinkingType::GeminiLevel => vec![
            ThinkingOption {
                label: "Disable (minimal) - fastest".to_string(),
                config_value: "minimal".to_string(),
                config_key: "thinking_level",
            },
            ThinkingOption {
                label: "Low - faster responses".to_string(),
                config_value: "low".to_string(),
                config_key: "thinking_level",
            },
            ThinkingOption {
                label: "Medium - balanced".to_string(),
                config_value: "medium".to_string(),
                config_key: "thinking_level",
            },
            ThinkingOption {
                label: "High - deep reasoning (default)".to_string(),
                config_value: "high".to_string(),
                config_key: "thinking_level",
            },
        ],
        ThinkingType::GeminiBudget => vec![
            ThinkingOption {
                label: "Disable (0 tokens)".to_string(),
                config_value: "0".to_string(),
                config_key: "thinking_budget",
            },
            ThinkingOption {
                label: "Low (~1024 tokens)".to_string(),
                config_value: "1024".to_string(),
                config_key: "thinking_budget",
            },
            ThinkingOption {
                label: "Medium (~4096 tokens)".to_string(),
                config_value: "4096".to_string(),
                config_key: "thinking_budget",
            },
            ThinkingOption {
                label: "High (~16384 tokens)".to_string(),
                config_value: "16384".to_string(),
                config_key: "thinking_budget",
            },
            ThinkingOption {
                label: "Dynamic (auto-adjust)".to_string(),
                config_value: "-1".to_string(),
                config_key: "thinking_budget",
            },
            ThinkingOption {
                label: "Custom (enter token count)".to_string(),
                config_value: "custom".to_string(),
                config_key: "thinking_budget",
            },
        ],
        ThinkingType::OpenAIEffort => vec![
            ThinkingOption {
                label: "None - no reasoning".to_string(),
                config_value: "none".to_string(),
                config_key: "reasoning_effort",
            },
            ThinkingOption {
                label: "Minimal - fastest".to_string(),
                config_value: "minimal".to_string(),
                config_key: "reasoning_effort",
            },
            ThinkingOption {
                label: "Low".to_string(),
                config_value: "low".to_string(),
                config_key: "reasoning_effort",
            },
            ThinkingOption {
                label: "Medium (default)".to_string(),
                config_value: "medium".to_string(),
                config_key: "reasoning_effort",
            },
            ThinkingOption {
                label: "High".to_string(),
                config_value: "high".to_string(),
                config_key: "reasoning_effort",
            },
            ThinkingOption {
                label: "XHigh - maximum reasoning".to_string(),
                config_value: "xhigh".to_string(),
                config_key: "reasoning_effort",
            },
        ],
        ThinkingType::AnthropicBudget => vec![
            ThinkingOption {
                label: "Disable (0 tokens)".to_string(),
                config_value: "0".to_string(),
                config_key: "thinking_budget",
            },
            ThinkingOption {
                label: "Low (~8000 tokens)".to_string(),
                config_value: "8000".to_string(),
                config_key: "thinking_budget",
            },
            ThinkingOption {
                label: "Medium (~16000 tokens)".to_string(),
                config_value: "16000".to_string(),
                config_key: "thinking_budget",
            },
            ThinkingOption {
                label: "High (~32000 tokens)".to_string(),
                config_value: "32000".to_string(),
                config_key: "thinking_budget",
            },
            ThinkingOption {
                label: "Custom (enter token count)".to_string(),
                config_value: "custom".to_string(),
                config_key: "thinking_budget",
            },
        ],
        ThinkingType::NotSupported => vec![],
    }
}

pub fn select_thinking_config(provider: &str, model: &str) -> Result<Option<(String, String)>> {
    let thinking_type = detect_thinking_type(provider, model);

    if thinking_type == ThinkingType::NotSupported {
        return Ok(None);
    }

    let options = get_thinking_options(thinking_type);
    if options.is_empty() {
        return Ok(None);
    }

    let labels: Vec<&str> = options.iter().map(|o| o.label.as_str()).collect();

    let default_idx = match thinking_type {
        ThinkingType::GeminiLevel => 1,
        ThinkingType::GeminiBudget => 1,
        ThinkingType::OpenAIEffort => 3,
        ThinkingType::AnthropicBudget => 1,
        ThinkingType::NotSupported => 0,
    };

    let idx = numbered_select("Select thinking mode", &labels, default_idx)?;
    let selected = &options[idx];

    let value = if selected.config_value == "custom" {
        let question = Question::input("token_count")
            .message("Enter token count (1024-128000)")
            .default("8000")
            .build();
        requestty::prompt_one(question)?
            .as_string()
            .unwrap_or("8000")
            .to_string()
    } else {
        selected.config_value.clone()
    };

    Ok(Some((selected.config_key.to_string(), value)))
}

pub fn format_thinking_config(key: &str, value: &str) -> String {
    if value == "0" || value.is_empty() {
        return String::new();
    }

    if key == "thinking_budget" {
        format!("\n{} = {}", key, value)
    } else {
        format!("\n{} = \"{}\"", key, value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_gemini_3() {
        assert_eq!(
            detect_thinking_type("gemini", "gemini-3-flash-preview"),
            ThinkingType::GeminiLevel
        );
        assert_eq!(
            detect_thinking_type("gemini", "gemini-3-pro-preview"),
            ThinkingType::GeminiLevel
        );
    }

    #[test]
    fn test_detect_gemini_25() {
        assert_eq!(
            detect_thinking_type("gemini", "gemini-2.5-flash"),
            ThinkingType::GeminiBudget
        );
        assert_eq!(
            detect_thinking_type("gemini", "gemini-flash-latest"),
            ThinkingType::GeminiBudget
        );
        assert_eq!(
            detect_thinking_type("gemini", "gemini-pro-latest"),
            ThinkingType::GeminiBudget
        );
    }

    #[test]
    fn test_detect_openai() {
        assert_eq!(
            detect_thinking_type("openai", "o1-preview"),
            ThinkingType::OpenAIEffort
        );
        assert_eq!(
            detect_thinking_type("openai", "gpt-5"),
            ThinkingType::OpenAIEffort
        );
        assert_eq!(
            detect_thinking_type("openai", "gpt-4o"),
            ThinkingType::NotSupported
        );
    }

    #[test]
    fn test_detect_anthropic() {
        assert_eq!(
            detect_thinking_type("anthropic", "claude-3-sonnet"),
            ThinkingType::AnthropicBudget
        );
    }
}
