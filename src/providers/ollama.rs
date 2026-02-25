//! Ollama provider implementation using the native /api/chat endpoint

use super::{Message, Provider, ProviderOptions, ProviderResponse, StreamCallback};
use crate::http::create_client;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};

pub struct OllamaProvider {
    base_url: String,
    model: String,
    client: Client,
}

#[derive(Serialize)]
struct OllamaRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "is_false")]
    think: bool,
}

fn is_false(v: &bool) -> bool {
    !v
}

#[derive(Serialize, Deserialize)]
struct OllamaMessage {
    role: String,
    content: String,
}

// Non-streaming response
#[derive(Deserialize)]
struct OllamaResponse {
    message: Option<OllamaMessage>,
    error: Option<String>,
}

// Streaming NDJSON chunk
#[derive(Deserialize)]
struct OllamaStreamChunk {
    message: Option<OllamaDelta>,
    done: bool,
    error: Option<String>,
}

#[derive(Deserialize)]
struct OllamaDelta {
    content: Option<String>,
}

impl OllamaProvider {
    pub fn new(_api_key: String, base_url: String, model: String) -> Self {
        Self {
            base_url,
            model,
            client: create_client(),
        }
    }

    fn convert_messages(&self, messages: &[Message]) -> Vec<OllamaMessage> {
        messages
            .iter()
            .map(|m| OllamaMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect()
    }
}

#[async_trait]
impl Provider for OllamaProvider {
    async fn complete_with_options(
        &self,
        messages: &[Message],
        options: &ProviderOptions,
    ) -> Result<ProviderResponse> {
        let url = format!("{}/api/chat", self.base_url);

        let request = OllamaRequest {
            model: self.model.clone(),
            messages: self.convert_messages(messages),
            stream: false,
            think: options.thinking_enabled,
        };

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        let status = response.status();
        let body = response.text().await?;

        if !status.is_success() {
            return Err(anyhow!("Ollama API error ({}): {}", status, body));
        }

        let parsed: OllamaResponse = serde_json::from_str(&body)
            .map_err(|e| anyhow!("Failed to parse Ollama response: {} — body: {}", e, body))?;

        if let Some(err) = parsed.error {
            return Err(anyhow!("Ollama error: {}", err));
        }

        let text = parsed.message.map(|m| m.content).unwrap_or_default();

        Ok(ProviderResponse {
            text,
            citations: vec![],
        })
    }

    async fn stream_with_options(
        &self,
        messages: &[Message],
        mut callback: StreamCallback,
        options: &ProviderOptions,
    ) -> Result<()> {
        let url = format!("{}/api/chat", self.base_url);

        let request = OllamaRequest {
            model: self.model.clone(),
            messages: self.convert_messages(messages),
            stream: true,
            think: options.thinking_enabled,
        };

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let body = response.text().await?;
            return Err(anyhow!("Ollama API error: {}", body));
        }

        let mut stream = response.bytes_stream();
        // Raw byte buffer to avoid splitting multibyte UTF-8 sequences at chunk boundaries
        let mut raw_buf: Vec<u8> = Vec::new();

        'outer: while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            raw_buf.extend_from_slice(&chunk);

            // Process complete NDJSON lines (newline delimited)
            while let Some(newline_pos) = raw_buf.iter().position(|&b| b == b'\n') {
                let line_bytes = raw_buf.drain(..=newline_pos).collect::<Vec<u8>>();
                let line = match std::str::from_utf8(&line_bytes) {
                    Ok(s) => s.trim().to_string(),
                    Err(_) => continue, // Skip malformed UTF-8 lines
                };

                if line.is_empty() {
                    continue;
                }

                if let Ok(parsed) = serde_json::from_str::<OllamaStreamChunk>(&line) {
                    if let Some(err) = parsed.error {
                        return Err(anyhow!("Ollama stream error: {}", err));
                    }

                    if let Some(delta) = parsed.message {
                        if let Some(content) = delta.content {
                            if !content.is_empty() {
                                callback(&content);
                            }
                        }
                    }

                    if parsed.done {
                        break 'outer;
                    }
                }
            }
        }

        Ok(())
    }

    fn name(&self) -> &str {
        "ollama"
    }

    fn model(&self) -> &str {
        &self.model
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ollama_request_with_thinking() {
        let req = OllamaRequest {
            model: "qwen3".to_string(),
            messages: vec![],
            stream: false,
            think: true,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains(r#""think":true"#));
    }

    #[test]
    fn test_ollama_request_without_thinking() {
        let req = OllamaRequest {
            model: "llama3.2".to_string(),
            messages: vec![],
            stream: false,
            think: false,
        };
        let json = serde_json::to_string(&req).unwrap();
        // think:false must not be serialized (skip_serializing_if = is_false)
        assert!(!json.contains(r#""think""#));
    }
}
