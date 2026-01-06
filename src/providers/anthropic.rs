//! Anthropic Claude provider implementation

use super::{Message, Provider, StreamCallback};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};

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
    text: Option<String>,
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
            client: Client::new(),
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
}

#[async_trait]
impl Provider for AnthropicProvider {
    async fn complete(&self, messages: &[Message]) -> Result<String> {
        let url = format!("{}/v1/messages", self.base_url);
        let (system, msgs) = self.convert_messages(messages);

        let request = AnthropicRequest {
            model: self.model.clone(),
            messages: msgs,
            max_tokens: 4096,
            system,
            stream: false,
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

        let text = response
            .content
            .and_then(|c| c.into_iter().next())
            .and_then(|c| c.text)
            .unwrap_or_default();

        Ok(text)
    }

    async fn stream(&self, messages: &[Message], mut callback: StreamCallback) -> Result<()> {
        let url = format!("{}/v1/messages", self.base_url);
        let (system, msgs) = self.convert_messages(messages);

        let request = AnthropicRequest {
            model: self.model.clone(),
            messages: msgs,
            max_tokens: 4096,
            system,
            stream: true,
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

            // Parse SSE data
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
