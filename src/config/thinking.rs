//! Thinking mode configuration helpers
//!
//! Different providers use different parameters:
//! - Gemini 2.5: thinkingBudget (0, 1024-32768, -1 for dynamic)
//! - Gemini 3: thinkingLevel (minimal, low, medium, high)
//! - OpenAI: reasoning_effort (none, minimal, low, medium, high, xhigh)
//! - Anthropic: thinking_budget (0, 1024-128000)

use anyhow::{anyhow, Result};
use requestty::Question;

use super::numbered_select;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ThinkingType {
    GeminiBudget,
    GeminiLevel,
    OpenAIEffort,
    AnthropicBudget,
    OllamaThink,
    NotSupported,
}

fn gemini_major_version(model: &str) -> Option<u32> {
    model
        .strip_prefix("gemini-")?
        .split(['.', '-'])
        .next()?
        .parse()
        .ok()
}

pub(crate) fn legacy_budget_to_level(budget: i64) -> Result<Option<&'static str>> {
    match budget {
        -1 => Ok(None),
        value if value < -1 => Err(anyhow!("Invalid legacy Gemini thinking budget: {value}")),
        0 => Ok(Some("minimal")),
        1..=2048 => Ok(Some("low")),
        2049..=8192 => Ok(Some("medium")),
        _ => Ok(Some("high")),
    }
}

fn normalize_existing_thinking_value(
    thinking_type: ThinkingType,
    existing_value: String,
) -> String {
    if thinking_type != ThinkingType::GeminiLevel {
        return existing_value;
    }

    let Ok(budget) = existing_value.parse::<i64>() else {
        return existing_value;
    };
    if budget == -1 {
        return existing_value;
    }

    legacy_budget_to_level(budget)
        .ok()
        .flatten()
        .unwrap_or(existing_value.as_str())
        .to_string()
}

pub fn detect_thinking_type(provider: &str, model: &str) -> ThinkingType {
    match provider {
        "gemini" => {
            let model_lower = model.to_lowercase();
            if model_lower == "gemini-2.5"
                || model_lower.starts_with("gemini-2.5-")
                || model_lower == "gemini-2-5"
                || model_lower.starts_with("gemini-2-5-")
            {
                ThinkingType::GeminiBudget
            } else if matches!(
                model_lower.as_str(),
                "gemini-flash-latest" | "gemini-flash-lite-latest"
            ) || gemini_major_version(&model_lower).is_some_and(|major| major >= 3)
            {
                ThinkingType::GeminiLevel
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
        "ollama" => ThinkingType::OllamaThink,
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
                label: "Minimal - fastest".to_string(),
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
                label: "High - deep reasoning".to_string(),
                config_value: "high".to_string(),
                config_key: "thinking_level",
            },
            ThinkingOption {
                label: "Default (dynamic)".to_string(),
                config_value: "-1".to_string(),
                config_key: "thinking_budget",
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
                label: "Disable".to_string(),
                config_value: "0".to_string(),
                config_key: "thinking_budget",
            },
            ThinkingOption {
                label: "Low (4k tokens)".to_string(),
                config_value: "low".to_string(),
                config_key: "thinking_level",
            },
            ThinkingOption {
                label: "Medium (8k tokens)".to_string(),
                config_value: "medium".to_string(),
                config_key: "thinking_level",
            },
            ThinkingOption {
                label: "High (16k tokens)".to_string(),
                config_value: "high".to_string(),
                config_key: "thinking_level",
            },
            ThinkingOption {
                label: "XHigh (32k tokens)".to_string(),
                config_value: "xhigh".to_string(),
                config_key: "thinking_level",
            },
            ThinkingOption {
                label: "Custom (enter token count)".to_string(),
                config_value: "custom".to_string(),
                config_key: "thinking_budget",
            },
        ],
        ThinkingType::OllamaThink => vec![
            ThinkingOption {
                label: "Disable (default)".to_string(),
                config_value: "0".to_string(),
                config_key: "thinking_budget",
            },
            ThinkingOption {
                label: "Enable thinking (model must support it)".to_string(),
                config_value: "1".to_string(),
                config_key: "thinking_budget",
            },
        ],
        ThinkingType::NotSupported => vec![],
    }
}

pub fn select_thinking_config(
    provider: &str,
    model: &str,
    existing_value: Option<String>,
) -> Result<Option<(String, String)>> {
    let thinking_type = detect_thinking_type(provider, model);

    if thinking_type == ThinkingType::NotSupported {
        return Ok(None);
    }

    let options = get_thinking_options(thinking_type);
    if options.is_empty() {
        return Ok(None);
    }

    let labels: Vec<&str> = options.iter().map(|o| o.label.as_str()).collect();

    let default_idx = if let Some(existing) =
        existing_value.map(|value| normalize_existing_thinking_value(thinking_type, value))
    {
        options
            .iter()
            .position(|o| o.config_value == existing)
            .unwrap_or(match thinking_type {
                ThinkingType::GeminiLevel => 1,
                ThinkingType::GeminiBudget => 1,
                ThinkingType::OpenAIEffort => 3,
                ThinkingType::AnthropicBudget => 1,
                ThinkingType::OllamaThink => 0,
                ThinkingType::NotSupported => 0,
            })
    } else {
        match thinking_type {
            ThinkingType::GeminiLevel => 1,
            ThinkingType::GeminiBudget => 1,
            ThinkingType::OpenAIEffort => 3,
            ThinkingType::AnthropicBudget => 1,
            ThinkingType::OllamaThink => 0,
            ThinkingType::NotSupported => 0,
        }
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
        assert_eq!(
            detect_thinking_type("gemini", "gemini-3.5-flash-lite"),
            ThinkingType::GeminiLevel
        );
        assert_eq!(
            detect_thinking_type("gemini", "gemini-3.6-flash"),
            ThinkingType::GeminiLevel
        );
        assert_eq!(
            detect_thinking_type("gemini", "gemini-4-flash"),
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
            detect_thinking_type("gemini", "gemini-2.5-flash-preview-05-20"),
            ThinkingType::GeminiBudget
        );
    }

    #[test]
    fn test_detect_gemini_latest_aliases() {
        assert_eq!(
            detect_thinking_type("gemini", "gemini-flash-latest"),
            ThinkingType::GeminiLevel
        );
        assert_eq!(
            detect_thinking_type("gemini", "gemini-flash-lite-latest"),
            ThinkingType::GeminiLevel
        );
    }

    #[test]
    fn test_normalize_legacy_budgets_for_level_models() {
        for (budget, expected) in [
            ("0", "minimal"),
            ("1024", "low"),
            ("4096", "medium"),
            ("8192", "medium"),
            ("16384", "high"),
            ("-1", "-1"),
        ] {
            assert_eq!(
                normalize_existing_thinking_value(ThinkingType::GeminiLevel, budget.to_string()),
                expected
            );
        }

        let options = get_thinking_options(ThinkingType::GeminiLevel);
        let dynamic = options.iter().find(|option| option.config_value == "-1");
        assert_eq!(
            dynamic.map(|option| option.config_key),
            Some("thinking_budget")
        );
    }

    #[test]
    fn test_does_not_guess_unknown_gemini_contracts() {
        assert_eq!(
            detect_thinking_type("gemini", "gemini-2.0-flash"),
            ThinkingType::NotSupported
        );
        assert_eq!(
            detect_thinking_type("gemini", "gemini-pro-latest"),
            ThinkingType::NotSupported
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

    #[test]
    fn test_detect_ollama() {
        assert_eq!(
            detect_thinking_type("ollama", "llama3.2"),
            ThinkingType::OllamaThink
        );
        assert_eq!(
            detect_thinking_type("ollama", "qwen3"),
            ThinkingType::OllamaThink
        );
        assert_eq!(
            detect_thinking_type("ollama", "deepseek-r1"),
            ThinkingType::OllamaThink
        );
    }

    #[test]
    fn test_ollama_thinking_options() {
        let opts = get_thinking_options(ThinkingType::OllamaThink);
        assert_eq!(opts.len(), 2);
        assert_eq!(opts[0].config_value, "0"); // disable
        assert_eq!(opts[1].config_value, "1"); // enable
        assert!(opts.iter().all(|o| o.config_key == "thinking_budget"));
    }
}
