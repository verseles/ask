//! OpenAI provider implementation (also works with OpenAI-compatible APIs)

use super::{Message, Provider, StreamCallback};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use futures::StreamExt;
use crate::http::create_client;
use reqwest::Client;
use serde::{Deserialize, Serialize};

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

    /// Check if model is a reasoning model (o1, o3, o4, gpt-5) that requires max_completion_tokens
    fn is_reasoning_model(&self) -> bool {
        let model = self.model.to_lowercase();
        model.starts_with("o1")
            || model.starts_with("o3")
            || model.starts_with("o4")
            || model.starts_with("gpt-5")
    }
}

#[async_trait]
impl Provider for OpenAIProvider {
    async fn complete(&self, messages: &[Message]) -> Result<String> {
        let url = format!("{}/chat/completions", self.base_url);

        let is_reasoning = self.is_reasoning_model();
        let request = OpenAIRequest {
            model: self.model.clone(),
            messages: self.convert_messages(messages),
            stream: false,
            temperature: if is_reasoning { None } else { Some(0.7) },
            max_tokens: if is_reasoning { None } else { Some(4096) },
            max_completion_tokens: if is_reasoning { Some(4096) } else { None },
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

        Ok(text)
    }

    async fn stream(&self, messages: &[Message], mut callback: StreamCallback) -> Result<()> {
        let url = format!("{}/chat/completions", self.base_url);

        let is_reasoning = self.is_reasoning_model();
        let request = OpenAIRequest {
            model: self.model.clone(),
            messages: self.convert_messages(messages),
            stream: true,
            temperature: if is_reasoning { None } else { Some(0.7) },
            max_tokens: if is_reasoning { None } else { Some(4096) },
            max_completion_tokens: if is_reasoning { Some(4096) } else { None },
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

            // Parse SSE data
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
