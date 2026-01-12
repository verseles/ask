//! Anthropic Claude provider implementation

use super::{Citation, Message, Provider, ProviderOptions, ProviderResponse, StreamCallback};
use crate::http::create_client;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub struct AnthropicProvider {
    api_key: String,
    base_url: String,
    model: String,
    client: Client,
}

#[derive(Serialize)]
struct AnthropicRequest {
    model: String,
    messages: Vec<AnthropicMessage>,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    thinking: Option<ThinkingConfig>,
}

#[derive(Serialize)]
struct ThinkingConfig {
    #[serde(rename = "type")]
    thinking_type: String,
    budget_tokens: u64,
}

#[derive(Serialize, Deserialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct AnthropicResponse {
    content: Option<Vec<AnthropicContent>>,
    error: Option<AnthropicError>,
}

#[derive(Deserialize)]
struct AnthropicContent {
    #[serde(rename = "type")]
    #[allow(dead_code)]
    content_type: Option<String>,
    text: Option<String>,
    citations: Option<Vec<AnthropicCitation>>,
}

#[derive(Deserialize)]
struct AnthropicCitation {
    url: Option<String>,
    title: Option<String>,
}

#[derive(Deserialize)]
struct AnthropicError {
    message: String,
}

#[derive(Deserialize)]
struct AnthropicStreamEvent {
    #[serde(rename = "type")]
    event_type: String,
    delta: Option<AnthropicDelta>,
}

#[derive(Deserialize)]
struct AnthropicDelta {
    text: Option<String>,
}

impl AnthropicProvider {
    pub fn new(api_key: String, base_url: String, model: String) -> Self {
        Self {
            api_key,
            base_url,
            model,
            client: create_client(),
        }
    }

    fn convert_messages(&self, messages: &[Message]) -> (Option<String>, Vec<AnthropicMessage>) {
        let mut system = None;
        let mut result = Vec::new();

        for msg in messages {
            match msg.role.as_str() {
                "system" => {
                    system = Some(msg.content.clone());
                }
                "user" | "assistant" => {
                    result.push(AnthropicMessage {
                        role: msg.role.clone(),
                        content: msg.content.clone(),
                    });
                }
                _ => {}
            }
        }

        (system, result)
    }

    fn build_tools(&self, options: &ProviderOptions) -> Option<Vec<Value>> {
        if !options.web_search {
            return None;
        }

        let mut tool = serde_json::json!({
            "type": "web_search_20250305",
            "name": "web_search"
        });

        if let Some(ref domains) = options.allowed_domains {
            if !domains.is_empty() {
                tool["allowed_domains"] = serde_json::json!(domains);
            }
        }

        if let Some(ref domains) = options.blocked_domains {
            if !domains.is_empty() {
                tool["blocked_domains"] = serde_json::json!(domains);
            }
        }

        Some(vec![tool])
    }

    fn extract_citations(&self, content: &[AnthropicContent]) -> Vec<Citation> {
        let mut citations = Vec::new();
        for item in content {
            if let Some(ref cites) = item.citations {
                for cite in cites {
                    citations.push(Citation {
                        url: cite.url.clone().unwrap_or_default(),
                        title: cite.title.clone().unwrap_or_default(),
                        snippet: None,
                    });
                }
            }
        }
        citations
    }

    fn build_thinking(&self, options: &ProviderOptions) -> Option<ThinkingConfig> {
        if !options.thinking_enabled {
            return None;
        }

        let value = options
            .thinking_value
            .as_ref()
            .map(|v| v.to_lowercase())
            .unwrap_or_else(|| "medium".to_string());

        let budget = match value.as_str() {
            "min" | "minimal" => 2048,
            "low" => 4096,
            "medium" | "med" => 8192,
            "high" => 16384,
            "xhigh" | "max" => 32768,
            s => s.parse::<u64>().unwrap_or(8192),
        };

        // Ensure budget is at least 1024 (API requirement)
        let budget = budget.max(1024);

        Some(ThinkingConfig {
            thinking_type: "enabled".to_string(),
            budget_tokens: budget,
        })
    }
}

#[async_trait]
impl Provider for AnthropicProvider {
    async fn complete_with_options(
        &self,
        messages: &[Message],
        options: &ProviderOptions,
    ) -> Result<ProviderResponse> {
        let url = format!("{}/v1/messages", self.base_url);
        let (system, msgs) = self.convert_messages(messages);

        let request = AnthropicRequest {
            model: self.model.clone(),
            messages: msgs,
            max_tokens: 4096,
            system,
            stream: false,
            tools: self.build_tools(options),
            thinking: self.build_thinking(options),
        };

        let response = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        let status = response.status();
        let body = response.text().await?;

        if !status.is_success() {
            return Err(anyhow!("Anthropic API error ({}): {}", status, body));
        }

        let response: AnthropicResponse = serde_json::from_str(&body)?;

        if let Some(error) = response.error {
            return Err(anyhow!("Anthropic error: {}", error.message));
        }

        let content = response.content.unwrap_or_default();

        let text = content
            .iter()
            .filter_map(|c| c.text.clone())
            .collect::<Vec<_>>()
            .join("");

        let citations = self.extract_citations(&content);

        Ok(ProviderResponse { text, citations })
    }

    async fn stream_with_options(
        &self,
        messages: &[Message],
        mut callback: StreamCallback,
        options: &ProviderOptions,
    ) -> Result<()> {
        let url = format!("{}/v1/messages", self.base_url);
        let (system, msgs) = self.convert_messages(messages);

        let request = AnthropicRequest {
            model: self.model.clone(),
            messages: msgs,
            max_tokens: 4096,
            system,
            stream: true,
            tools: self.build_tools(options),
            thinking: self.build_thinking(options),
        };

        let response = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let body = response.text().await?;
            return Err(anyhow!("Anthropic API error: {}", body));
        }

        let mut stream = response.bytes_stream();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            let text = String::from_utf8_lossy(&chunk);

            for line in text.lines() {
                if let Some(data) = line.strip_prefix("data: ") {
                    if let Ok(event) = serde_json::from_str::<AnthropicStreamEvent>(data) {
                        if event.event_type == "content_block_delta" {
                            if let Some(delta) = event.delta {
                                if let Some(text) = delta.text {
                                    callback(&text);
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn name(&self) -> &str {
        "anthropic"
    }

    fn model(&self) -> &str {
        &self.model
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_thinking_levels() {
        let provider = AnthropicProvider::new("key".into(), "url".into(), "claude-3-7-sonnet".into());
        
        let cases = vec![
            ("minimal", 2048),
            ("low", 4096),
            ("medium", 8192),
            ("high", 16384),
            ("12345", 12345),
            ("invalid", 8192), // default
        ];

        for (input, expected) in cases {
            let options = ProviderOptions {
                thinking_enabled: true,
                thinking_value: Some(input.to_string()),
                web_search: false,
                allowed_domains: None,
                blocked_domains: None,
            };
            
            let config = provider.build_thinking(&options).unwrap();
            assert_eq!(config.budget_tokens, expected, "Failed for input: {}", input);
        }
    }
    
    #[test]
    fn test_build_thinking_disabled() {
        let provider = AnthropicProvider::new("key".into(), "url".into(), "claude-3-5-sonnet".into());
        let options = ProviderOptions {
            thinking_enabled: false,
            thinking_value: Some("high".to_string()),
            web_search: false,
            allowed_domains: None,
            blocked_domains: None,
        };
        assert!(provider.build_thinking(&options).is_none());
    }
}