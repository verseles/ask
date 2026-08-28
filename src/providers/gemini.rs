//! Google Gemini provider implementation

use super::{Citation, Message, Provider, ProviderOptions, ProviderResponse, StreamCallback};
use crate::config::{detect_thinking_type, legacy_budget_to_level, ThinkingType};
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
    #[serde(rename = "maxOutputTokens", skip_serializing_if = "Option::is_none")]
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
    pub fn new(api_key: String, base_url: String, model: String) -> Result<Self> {
        Ok(Self {
            api_key,
            base_url,
            model,
            client: create_client()?,
        })
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

    fn build_generation_config(&self, options: &ProviderOptions) -> Result<GenerationConfig> {
        let thinking_type = detect_thinking_type("gemini", &self.model);
        let thinking_config = if options.thinking_enabled || options.thinking_value.is_some() {
            let value = options
                .thinking_value
                .as_deref()
                .unwrap_or("low")
                .trim()
                .to_lowercase();

            match thinking_type {
                ThinkingType::GeminiLevel => {
                    let level = if let Ok(budget) = value.parse::<i64>() {
                        legacy_budget_to_level(budget)
                            .map_err(|_| {
                                anyhow!(
                                    "Invalid Gemini thinking budget for {}: {}",
                                    self.model,
                                    budget
                                )
                            })?
                            .map(str::to_uppercase)
                    } else {
                        match value.as_str() {
                            "none" | "minimal" => Some("MINIMAL".to_string()),
                            "low" => Some("LOW".to_string()),
                            "medium" => Some("MEDIUM".to_string()),
                            "high" => Some("HIGH".to_string()),
                            _ => {
                                return Err(anyhow!(
                                    "Invalid Gemini thinking level for {}: {}",
                                    self.model,
                                    value
                                ));
                            }
                        }
                    };

                    level.map(|level| ThinkingConfig {
                        thinking_level: Some(level),
                        thinking_budget: None,
                    })
                }
                ThinkingType::GeminiBudget => {
                    let budget = match value.as_str() {
                        "none" => 0,
                        "minimal" => 1024,
                        "low" => 4096,
                        "medium" => 8192,
                        "high" => 16384,
                        _ => value.parse::<i32>().map_err(|_| {
                            anyhow!(
                                "Invalid Gemini thinking budget for {}: {}",
                                self.model,
                                value
                            )
                        })?,
                    };

                    if budget < -1 {
                        return Err(anyhow!(
                            "Invalid Gemini thinking budget for {}: {}",
                            self.model,
                            budget
                        ));
                    }

                    Some(ThinkingConfig {
                        thinking_level: None,
                        thinking_budget: Some(budget),
                    })
                }
                _ => None,
            }
        } else {
            None
        };

        Ok(GenerationConfig {
            temperature: match thinking_type {
                ThinkingType::GeminiLevel => None,
                ThinkingType::GeminiBudget if options.thinking_enabled => None,
                _ => Some(0.7),
            },
            max_output_tokens: Some(65536),
            thinking_config,
        })
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
            generation_config: Some(self.build_generation_config(options)?),
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
            generation_config: Some(self.build_generation_config(options)?),
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
        let mut raw_buf: Vec<u8> = Vec::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            raw_buf.extend_from_slice(&chunk);

            // Process complete lines from buffer
            while let Some(newline_pos) = raw_buf.iter().position(|&b| b == b'\n') {
                let line_bytes = raw_buf.drain(..=newline_pos).collect::<Vec<u8>>();
                let line = match std::str::from_utf8(&line_bytes) {
                    Ok(s) => s.trim().to_string(),
                    Err(_) => continue,
                };

                if line.is_empty() {
                    continue;
                }

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

        // Process any remaining data in buffer after stream ends
        if !raw_buf.is_empty() {
            if let Ok(line) = std::str::from_utf8(&raw_buf) {
                let line = line.trim();
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

#[cfg(test)]
mod tests {
    use super::*;

    fn provider(model: &str) -> GeminiProvider {
        GeminiProvider::new(
            "test-key".to_string(),
            "https://example.com".to_string(),
            model.to_string(),
        )
    }

    fn thinking_options(value: &str) -> ProviderOptions {
        ProviderOptions {
            thinking_enabled: true,
            thinking_value: Some(value.to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn latest_models_use_level_payload_without_sampling() {
        for model in [
            "gemini-3.6-flash",
            "gemini-3.5-flash-lite",
            "gemini-flash-latest",
            "gemini-flash-lite-latest",
        ] {
            let config = provider(model)
                .build_generation_config(&thinking_options("medium"))
                .unwrap();
            let payload = serde_json::to_value(config).unwrap();

            assert_eq!(payload["thinkingConfig"]["thinkingLevel"], "MEDIUM");
            assert!(payload["thinkingConfig"].get("thinkingBudget").is_none());
            assert!(payload.get("temperature").is_none());
            assert_eq!(payload["maxOutputTokens"], 65536);
            assert!(payload.get("max_output_tokens").is_none());
        }
    }

    #[test]
    fn latest_models_omit_sampling_without_explicit_thinking() {
        for model in ["gemini-3.6-flash", "gemini-flash-lite-latest"] {
            let config = provider(model)
                .build_generation_config(&ProviderOptions::default())
                .unwrap();
            let payload = serde_json::to_value(config).unwrap();

            assert!(payload.get("temperature").is_none());
            assert!(payload.get("thinkingConfig").is_none());
        }
    }

    #[test]
    fn explicit_disabled_legacy_config_is_preserved() {
        let options = ProviderOptions {
            thinking_enabled: false,
            thinking_value: Some("0".to_string()),
            ..Default::default()
        };

        let config = provider("gemini-flash-latest")
            .build_generation_config(&options)
            .unwrap();
        let payload = serde_json::to_value(config).unwrap();
        assert_eq!(payload["thinkingConfig"]["thinkingLevel"], "MINIMAL");

        let config = provider("gemini-2.5-flash")
            .build_generation_config(&options)
            .unwrap();
        let payload = serde_json::to_value(config).unwrap();
        assert_eq!(payload["thinkingConfig"]["thinkingBudget"], 0);
    }

    #[test]
    fn legacy_budgets_are_converted_to_levels() {
        for (budget, expected) in [
            ("0", Some("MINIMAL")),
            ("1024", Some("LOW")),
            ("4096", Some("MEDIUM")),
            ("8192", Some("MEDIUM")),
            ("16384", Some("HIGH")),
            ("-1", None),
        ] {
            let config = provider("gemini-flash-latest")
                .build_generation_config(&thinking_options(budget))
                .unwrap();
            let payload = serde_json::to_value(config).unwrap();

            match expected {
                Some(level) => assert_eq!(payload["thinkingConfig"]["thinkingLevel"], level),
                None => assert!(payload.get("thinkingConfig").is_none()),
            }
        }
    }

    #[test]
    fn gemini_25_keeps_budget_contract_and_sampling_behavior() {
        let provider = provider("gemini-2.5-flash");
        let config = provider
            .build_generation_config(&thinking_options("4096"))
            .unwrap();
        let payload = serde_json::to_value(config).unwrap();

        assert_eq!(payload["thinkingConfig"]["thinkingBudget"], 4096);
        assert!(payload["thinkingConfig"].get("thinkingLevel").is_none());
        assert!(payload.get("temperature").is_none());

        let config = provider
            .build_generation_config(&ProviderOptions::default())
            .unwrap();
        let payload = serde_json::to_value(config).unwrap();
        assert!((payload["temperature"].as_f64().unwrap() - 0.7).abs() < 1e-6);
    }

    #[test]
    fn unknown_models_do_not_receive_inferred_thinking_config() {
        let config = provider("gemini-2.0-flash")
            .build_generation_config(&thinking_options("medium"))
            .unwrap();
        let payload = serde_json::to_value(config).unwrap();

        assert!(payload.get("thinkingConfig").is_none());
        assert!((payload["temperature"].as_f64().unwrap() - 0.7).abs() < 1e-6);
    }

    #[test]
    fn invalid_thinking_values_fail_locally() {
        let provider = provider("gemini-3.6-flash");
        assert!(provider
            .build_generation_config(&thinking_options("extreme"))
            .is_err());
        assert!(provider
            .build_generation_config(&thinking_options("-2"))
            .is_err());
    }
}
