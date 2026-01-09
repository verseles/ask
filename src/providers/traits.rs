//! Provider trait definitions

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// A message in a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

/// Citation from web search results
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct Citation {
    pub title: String,
    pub url: String,
    pub snippet: Option<String>,
}

/// Response with optional citations
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct ProviderResponse {
    pub text: String,
    pub citations: Vec<Citation>,
}

/// Options for provider requests
#[derive(Debug, Clone, Default)]
pub struct ProviderOptions {
    pub web_search: bool,
    pub allowed_domains: Option<Vec<String>>,
    pub blocked_domains: Option<Vec<String>>,
}

/// Callback type for streaming responses
pub type StreamCallback = Box<dyn FnMut(&str) + Send>;

#[async_trait]
pub trait Provider: Send + Sync {
    #[allow(dead_code)]
    async fn complete(&self, messages: &[Message]) -> Result<String> {
        let response = self
            .complete_with_options(messages, &ProviderOptions::default())
            .await?;
        Ok(response.text)
    }

    async fn complete_with_options(
        &self,
        messages: &[Message],
        options: &ProviderOptions,
    ) -> Result<ProviderResponse>;

    #[allow(dead_code)]
    async fn stream(&self, messages: &[Message], callback: StreamCallback) -> Result<()> {
        self.stream_with_options(messages, callback, &ProviderOptions::default())
            .await
    }

    async fn stream_with_options(
        &self,
        messages: &[Message],
        callback: StreamCallback,
        options: &ProviderOptions,
    ) -> Result<()>;

    #[allow(dead_code)]
    fn name(&self) -> &str;
    #[allow(dead_code)]
    fn model(&self) -> &str;
}

/// Intent classification types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntentType {
    /// User wants to execute shell commands
    Command,
    /// User has a question/informational request
    Question,
    /// User wants to generate code
    Code,
}

/// Intent classifier using a lightweight model
pub struct IntentClassifier<'a> {
    provider: &'a dyn Provider,
}

impl<'a> IntentClassifier<'a> {
    pub fn new(provider: &'a dyn Provider) -> Self {
        Self { provider }
    }

    /// Classify the user's intent
    pub async fn classify(&self, query: &str) -> Result<IntentType> {
        let system_prompt = r#"Classify the user's intent into exactly one category:

COMMAND - User wants to execute shell/terminal commands
QUESTION - User has a question or wants information
CODE - User wants to generate/write code

Respond with ONLY the category name, nothing else.

Examples:
"list all docker containers" -> COMMAND
"how does kubernetes work" -> QUESTION
"write a python function to sort" -> CODE
"delete old log files" -> COMMAND
"what is the capital of France" -> QUESTION
"create a rust struct for user" -> CODE
"show disk usage" -> COMMAND
"explain async/await" -> QUESTION
"#;

        let messages = vec![
            Message {
                role: "system".to_string(),
                content: system_prompt.to_string(),
            },
            Message {
                role: "user".to_string(),
                content: query.to_string(),
            },
        ];

        let response = self.provider.complete(&messages).await?;
        let response = response.trim().to_uppercase();

        Ok(match response.as_str() {
            "COMMAND" => IntentType::Command,
            "CODE" => IntentType::Code,
            _ => IntentType::Question,
        })
    }
}
