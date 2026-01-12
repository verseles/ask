//! OpenAI provider implementation (also works with OpenAI-compatible APIs)

use super::{Citation, Message, Provider, ProviderOptions, ProviderResponse, StreamCallback};
use crate::http::create_client;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub struct OpenAIProvider {
    api_key: String,
    base_url: String,
    model: String,
    client: Client,
}

#[derive(Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_completion_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning_effort: Option<String>,
}

#[derive(Serialize)]
struct ResponsesAPIRequest {
    model: String,
    input: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<Value>>,
}

#[derive(Serialize, Deserialize)]
struct OpenAIMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct OpenAIResponse {
    choices: Option<Vec<OpenAIChoice>>,
    error: Option<OpenAIError>,
}

#[derive(Deserialize)]
struct ResponsesAPIResponse {
    output: Option<Vec<ResponseOutput>>,
    error: Option<OpenAIError>,
}

#[derive(Deserialize)]
struct ResponseOutput {
    #[serde(rename = "type")]
    #[allow(dead_code)]
    output_type: Option<String>,
    content: Option<Vec<ResponseContent>>,
}

#[derive(Deserialize)]
struct ResponseContent {
    #[serde(rename = "type")]
    #[allow(dead_code)]
    content_type: Option<String>,
    #[allow(dead_code)]
    text: Option<String>,
    annotations: Option<Vec<ResponseAnnotation>>,
}

#[derive(Deserialize)]
struct ResponseAnnotation {
    #[serde(rename = "type")]
    annotation_type: Option<String>,
    url: Option<String>,
    title: Option<String>,
}

#[derive(Deserialize)]
struct OpenAIChoice {
    message: Option<OpenAIMessage>,
    delta: Option<OpenAIDelta>,
}

#[derive(Deserialize)]
struct OpenAIDelta {
    content: Option<String>,
}

#[derive(Deserialize)]
struct OpenAIError {
    message: String,
}

impl OpenAIProvider {
    pub fn new(api_key: String, base_url: String, model: String) -> Self {
        Self {
            api_key,
            base_url,
            model,
            client: create_client(),
        }
    }

    fn convert_messages(&self, messages: &[Message]) -> Vec<OpenAIMessage> {
        messages
            .iter()
            .map(|m| OpenAIMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect()
    }

    fn is_reasoning_model(&self) -> bool {
        let model = self.model.to_lowercase();
        model.starts_with("o1")
            || model.starts_with("o3")
            || model.starts_with("o4")
            || model.starts_with("gpt-5")
    }

    fn supports_none_reasoning(&self) -> bool {
        let model = self.model.to_lowercase();
        model.contains("gpt-5.1") || model.contains("gpt-5.2") || model.contains("gpt-5.3")
    }

    fn normalize_reasoning_effort(&self, level: &str) -> String {
        if level == "none" && !self.supports_none_reasoning() {
            "minimal".to_string()
        } else {
            level.to_string()
        }
    }

    fn build_reasoning_effort(&self, options: &ProviderOptions) -> Option<String> {
        if !self.is_reasoning_model() {
            return None;
        }

        if options.thinking_enabled {
            let level = options
                .thinking_value
                .clone()
                .unwrap_or_else(|| "medium".to_string());
            Some(self.normalize_reasoning_effort(&level))
        } else {
            Some("minimal".to_string())
        }
    }

    fn is_official_openai(&self) -> bool {
        self.base_url.contains("api.openai.com")
    }

    fn messages_to_input(&self, messages: &[Message]) -> String {
        let mut parts = Vec::new();
        for msg in messages {
            match msg.role.as_str() {
                "system" => parts.push(format!("[System]: {}", msg.content)),
                "user" => parts.push(msg.content.clone()),
                "assistant" => parts.push(format!("[Assistant]: {}", msg.content)),
                _ => {}
            }
        }
        parts.join("\n\n")
    }

    async fn complete_with_responses_api(&self, messages: &[Message]) -> Result<ProviderResponse> {
        let url = format!("{}/responses", self.base_url);

        let request = ResponsesAPIRequest {
            model: self.model.clone(),
            input: self.messages_to_input(messages),
            tools: Some(vec![serde_json::json!({ "type": "web_search" })]),
        };

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        let status = response.status();
        let body = response.text().await?;

        if !status.is_success() {
            return Err(anyhow!("OpenAI Responses API error ({}): {}", status, body));
        }

        let response: ResponsesAPIResponse = serde_json::from_str(&body)?;

        if let Some(error) = response.error {
            return Err(anyhow!("OpenAI error: {}", error.message));
        }

        let mut text = String::new();
        let mut citations = Vec::new();

        if let Some(outputs) = response.output {
            for output in outputs {
                if output.output_type.as_deref() != Some("message") {
                    continue;
                }
                if let Some(contents) = output.content {
                    for content in contents {
                        if content.content_type.as_deref() == Some("output_text") {
                            if let Some(t) = &content.text {
                                text.push_str(t);
                            }
                        }
                        if let Some(annotations) = &content.annotations {
                            for annotation in annotations {
                                if annotation.annotation_type.as_deref() == Some("url_citation") {
                                    citations.push(Citation {
                                        url: annotation.url.clone().unwrap_or_default(),
                                        title: annotation.title.clone().unwrap_or_default(),
                                        snippet: None,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(ProviderResponse { text, citations })
    }
}

#[async_trait]
impl Provider for OpenAIProvider {
    async fn complete_with_options(
        &self,
        messages: &[Message],
        options: &ProviderOptions,
    ) -> Result<ProviderResponse> {
        if options.web_search && self.is_official_openai() {
            return self.complete_with_responses_api(messages).await;
        }

        let url = format!("{}/chat/completions", self.base_url);

        let is_reasoning = self.is_reasoning_model();
        let reasoning_effort = self.build_reasoning_effort(options);
        let request = OpenAIRequest {
            model: self.model.clone(),
            messages: self.convert_messages(messages),
            stream: false,
            temperature: if is_reasoning { None } else { Some(0.7) },
            max_tokens: if is_reasoning { None } else { Some(4096) },
            max_completion_tokens: if is_reasoning { Some(4096) } else { None },
            reasoning_effort,
        };

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        let status = response.status();
        let body = response.text().await?;

        if !status.is_success() {
            return Err(anyhow!("OpenAI API error ({}): {}", status, body));
        }

        let response: OpenAIResponse = serde_json::from_str(&body)?;

        if let Some(error) = response.error {
            return Err(anyhow!("OpenAI error: {}", error.message));
        }

        let text = response
            .choices
            .and_then(|c| c.into_iter().next())
            .and_then(|c| c.message)
            .map(|m| m.content)
            .unwrap_or_default();

        Ok(ProviderResponse {
            text,
            citations: Vec::new(),
        })
    }

    async fn stream_with_options(
        &self,
        messages: &[Message],
        mut callback: StreamCallback,
        options: &ProviderOptions,
    ) -> Result<()> {
        let url = format!("{}/chat/completions", self.base_url);

        let is_reasoning = self.is_reasoning_model();
        let reasoning_effort = self.build_reasoning_effort(options);
        let request = OpenAIRequest {
            model: self.model.clone(),
            messages: self.convert_messages(messages),
            stream: true,
            temperature: if is_reasoning { None } else { Some(0.7) },
            max_tokens: if is_reasoning { None } else { Some(4096) },
            max_completion_tokens: if is_reasoning { Some(4096) } else { None },
            reasoning_effort,
        };

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let body = response.text().await?;
            return Err(anyhow!("OpenAI API error: {}", body));
        }

        let mut stream = response.bytes_stream();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            let text = String::from_utf8_lossy(&chunk);

            for line in text.lines() {
                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" {
                        break;
                    }

                    if let Ok(response) = serde_json::from_str::<OpenAIResponse>(data) {
                        if let Some(choices) = response.choices {
                            for choice in choices {
                                if let Some(delta) = choice.delta {
                                    if let Some(content) = delta.content {
                                        callback(&content);
                                    }
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
        "openai"
    }

    fn model(&self) -> &str {
        &self.model
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_reasoning_model() {
        let provider = OpenAIProvider::new("key".into(), "url".into(), "gpt-5-nano".into());
        assert!(provider.is_reasoning_model());

        let provider = OpenAIProvider::new("key".into(), "url".into(), "o1-preview".into());
        assert!(provider.is_reasoning_model());

        let provider = OpenAIProvider::new("key".into(), "url".into(), "gpt-4o".into());
        assert!(!provider.is_reasoning_model());
    }

    #[test]
    fn test_supports_none_reasoning() {
        let provider = OpenAIProvider::new("key".into(), "url".into(), "gpt-5.1".into());
        assert!(provider.supports_none_reasoning());

        let provider = OpenAIProvider::new("key".into(), "url".into(), "gpt-5.2-turbo".into());
        assert!(provider.supports_none_reasoning());

        let provider = OpenAIProvider::new("key".into(), "url".into(), "gpt-5-nano".into());
        assert!(!provider.supports_none_reasoning());

        let provider = OpenAIProvider::new("key".into(), "url".into(), "gpt-5-mini".into());
        assert!(!provider.supports_none_reasoning());
    }

    #[test]
    fn test_normalize_reasoning_effort() {
        let provider = OpenAIProvider::new("key".into(), "url".into(), "gpt-5-nano".into());
        assert_eq!(provider.normalize_reasoning_effort("none"), "minimal");
        assert_eq!(provider.normalize_reasoning_effort("minimal"), "minimal");
        assert_eq!(provider.normalize_reasoning_effort("medium"), "medium");

        let provider = OpenAIProvider::new("key".into(), "url".into(), "gpt-5.1".into());
        assert_eq!(provider.normalize_reasoning_effort("none"), "none");
    }

    #[test]
    fn test_build_reasoning_effort_disabled() {
        let provider = OpenAIProvider::new("key".into(), "url".into(), "gpt-5-nano".into());
        let options = ProviderOptions {
            thinking_enabled: false,
            thinking_value: None,
            web_search: false,
            allowed_domains: None,
            blocked_domains: None,
        };
        assert_eq!(
            provider.build_reasoning_effort(&options),
            Some("minimal".to_string())
        );
    }

    #[test]
    fn test_build_reasoning_effort_enabled() {
        let provider = OpenAIProvider::new("key".into(), "url".into(), "gpt-5-nano".into());
        let options = ProviderOptions {
            thinking_enabled: true,
            thinking_value: Some("high".to_string()),
            web_search: false,
            allowed_domains: None,
            blocked_domains: None,
        };
        assert_eq!(
            provider.build_reasoning_effort(&options),
            Some("high".to_string())
        );
    }

    #[test]
    fn test_build_reasoning_effort_non_reasoning_model() {
        let provider = OpenAIProvider::new("key".into(), "url".into(), "gpt-4o".into());
        let options = ProviderOptions {
            thinking_enabled: true,
            thinking_value: Some("high".to_string()),
            web_search: false,
            allowed_domains: None,
            blocked_domains: None,
        };
        assert_eq!(provider.build_reasoning_effort(&options), None);
    }
}
