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
    output_text: Option<String>,
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

        let text = response.output_text.unwrap_or_default();

        let mut citations = Vec::new();
        if let Some(outputs) = response.output {
            for output in outputs {
                if let Some(contents) = output.content {
                    for content in contents {
                        if let Some(annotations) = content.annotations {
                            for annotation in annotations {
                                if annotation.annotation_type.as_deref() == Some("url_citation") {
                                    citations.push(Citation {
                                        url: annotation.url.unwrap_or_default(),
                                        title: annotation.title.unwrap_or_default(),
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

        Ok(ProviderResponse {
            text,
            citations: Vec::new(),
        })
    }

    async fn stream_with_options(
        &self,
        messages: &[Message],
        mut callback: StreamCallback,
        _options: &ProviderOptions,
    ) -> Result<()> {
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
