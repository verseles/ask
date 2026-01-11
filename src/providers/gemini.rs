//! Google Gemini provider implementation

use super::{Citation, Message, Provider, ProviderOptions, ProviderResponse, StreamCallback};
use crate::http::create_client;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub struct GeminiProvider {
    api_key: String,
    base_url: String,
    model: String,
    client: Client,
}

#[derive(Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
    #[serde(rename = "generationConfig", skip_serializing_if = "Option::is_none")]
    generation_config: Option<GenerationConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<Value>>,
}

#[derive(Serialize)]
struct GeminiContent {
    role: String,
    parts: Vec<GeminiPart>,
}

#[derive(Serialize)]
struct GeminiPart {
    text: String,
}

#[derive(Serialize)]
struct GenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_output_tokens: Option<u32>,
    #[serde(rename = "thinkingConfig", skip_serializing_if = "Option::is_none")]
    thinking_config: Option<ThinkingConfig>,
}

#[derive(Serialize)]
struct ThinkingConfig {
    #[serde(rename = "thinkingLevel", skip_serializing_if = "Option::is_none")]
    thinking_level: Option<String>,
    #[serde(rename = "thinkingBudget", skip_serializing_if = "Option::is_none")]
    thinking_budget: Option<i32>,
}

#[derive(Deserialize)]
struct GeminiResponse {
    candidates: Option<Vec<GeminiCandidate>>,
    error: Option<GeminiError>,
}

#[derive(Deserialize)]
struct GeminiCandidate {
    content: GeminiContentResponse,
    #[serde(rename = "groundingMetadata")]
    grounding_metadata: Option<GroundingMetadata>,
}

#[derive(Deserialize)]
struct GeminiContentResponse {
    parts: Vec<GeminiPartResponse>,
}

#[derive(Deserialize)]
struct GeminiPartResponse {
    text: Option<String>,
}

#[derive(Deserialize)]
struct GeminiError {
    message: String,
}

#[derive(Deserialize)]
struct GroundingMetadata {
    #[serde(rename = "groundingChunks")]
    grounding_chunks: Option<Vec<GroundingChunk>>,
}

#[derive(Deserialize)]
struct GroundingChunk {
    web: Option<WebChunk>,
}

#[derive(Deserialize)]
struct WebChunk {
    uri: Option<String>,
    title: Option<String>,
}

#[derive(Deserialize)]
struct GeminiStreamResponse {
    candidates: Option<Vec<GeminiCandidate>>,
}

impl GeminiProvider {
    pub fn new(api_key: String, base_url: String, model: String) -> Self {
        Self {
            api_key,
            base_url,
            model,
            client: create_client(),
        }
    }

    fn convert_messages(&self, messages: &[Message]) -> Vec<GeminiContent> {
        let mut contents = Vec::new();
        let mut system_text = String::new();

        for msg in messages {
            match msg.role.as_str() {
                "system" => {
                    system_text = msg.content.clone();
                }
                "user" => {
                    let text = if !system_text.is_empty() {
                        let combined = format!("{}\n\n{}", system_text, msg.content);
                        system_text.clear();
                        combined
                    } else {
                        msg.content.clone()
                    };

                    contents.push(GeminiContent {
                        role: "user".to_string(),
                        parts: vec![GeminiPart { text }],
                    });
                }
                "assistant" => {
                    contents.push(GeminiContent {
                        role: "model".to_string(),
                        parts: vec![GeminiPart {
                            text: msg.content.clone(),
                        }],
                    });
                }
                _ => {}
            }
        }

        contents
    }

    fn build_tools(&self, options: &ProviderOptions) -> Option<Vec<Value>> {
        if options.web_search {
            Some(vec![serde_json::json!({ "google_search": {} })])
        } else {
            None
        }
    }

    fn supports_thinking(&self) -> bool {
        let model = self.model.to_lowercase();
        model.contains("gemini-3")
            || model.contains("gemini-2.5")
            || model.contains("2.5-flash")
            || model.contains("2.5-pro")
    }

    fn build_generation_config(&self, options: &ProviderOptions) -> GenerationConfig {
        let thinking_config = if options.thinking_enabled && self.supports_thinking() {
            let value = options
                .thinking_value
                .as_ref()
                .map(|v| v.to_uppercase())
                .unwrap_or_else(|| "LOW".to_string());

            // Gemini 3 models use thinkingLevel (minimal, low, medium, high)
            // Gemini 2.5 models use thinkingBudget (number of tokens)
            let is_gemini_3 = self.model.contains("gemini-3");

            if is_gemini_3 {
                Some(ThinkingConfig {
                    thinking_level: Some(value),
                    thinking_budget: None,
                })
            } else {
                // For Gemini 2.5, convert level to budget or parse as number
                let budget = match value.as_str() {
                    "MINIMAL" => 1024,
                    "LOW" => 4096,
                    "MEDIUM" => 8192,
                    "HIGH" => 16384,
                    _ => value.parse::<i32>().unwrap_or(4096),
                };
                Some(ThinkingConfig {
                    thinking_level: None,
                    thinking_budget: Some(budget),
                })
            }
        } else {
            None
        };

        GenerationConfig {
            temperature: if options.thinking_enabled && self.supports_thinking() {
                None
            } else {
                Some(0.7)
            },
            max_output_tokens: Some(8192),
            thinking_config,
        }
    }

    fn extract_citations(&self, candidate: &GeminiCandidate) -> Vec<Citation> {
        let mut citations = Vec::new();
        if let Some(ref metadata) = candidate.grounding_metadata {
            if let Some(ref chunks) = metadata.grounding_chunks {
                for chunk in chunks {
                    if let Some(ref web) = chunk.web {
                        citations.push(Citation {
                            url: web.uri.clone().unwrap_or_default(),
                            title: web.title.clone().unwrap_or_default(),
                            snippet: None,
                        });
                    }
                }
            }
        }
        citations
    }
}

#[async_trait]
impl Provider for GeminiProvider {
    async fn complete_with_options(
        &self,
        messages: &[Message],
        options: &ProviderOptions,
    ) -> Result<ProviderResponse> {
        let url = format!(
            "{}/v1beta/models/{}:generateContent?key={}",
            self.base_url, self.model, self.api_key
        );

        let request = GeminiRequest {
            contents: self.convert_messages(messages),
            generation_config: Some(self.build_generation_config(options)),
            tools: self.build_tools(options),
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
            return Err(anyhow!("Gemini API error ({}): {}", status, body));
        }

        let response: GeminiResponse = serde_json::from_str(&body)?;

        if let Some(error) = response.error {
            return Err(anyhow!("Gemini error: {}", error.message));
        }

        let candidate = response.candidates.and_then(|c| c.into_iter().next());

        let text = candidate
            .as_ref()
            .and_then(|c| c.content.parts.first())
            .and_then(|p| p.text.clone())
            .unwrap_or_default();

        let citations = candidate
            .as_ref()
            .map(|c| self.extract_citations(c))
            .unwrap_or_default();

        Ok(ProviderResponse { text, citations })
    }

    async fn stream_with_options(
        &self,
        messages: &[Message],
        mut callback: StreamCallback,
        options: &ProviderOptions,
    ) -> Result<()> {
        let url = format!(
            "{}/v1beta/models/{}:streamGenerateContent?key={}&alt=sse",
            self.base_url, self.model, self.api_key
        );

        let request = GeminiRequest {
            contents: self.convert_messages(messages),
            generation_config: Some(self.build_generation_config(options)),
            tools: self.build_tools(options),
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
            return Err(anyhow!("Gemini API error: {}", body));
        }

        let mut stream = response.bytes_stream();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            let text = String::from_utf8_lossy(&chunk);

            for line in text.lines() {
                if let Some(data) = line.strip_prefix("data: ") {
                    if let Ok(response) = serde_json::from_str::<GeminiStreamResponse>(data) {
                        if let Some(candidates) = response.candidates {
                            for candidate in candidates {
                                for part in candidate.content.parts {
                                    if let Some(text) = part.text {
                                        callback(&text);
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
        "gemini"
    }

    fn model(&self) -> &str {
        &self.model
    }
}
