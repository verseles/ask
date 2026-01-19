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
    pub thinking_enabled: bool,
    pub thinking_value: Option<String>,
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

#[derive(Debug, Clone, Default)]
pub struct PromptContext {
    pub os: String,
    pub shell: String,
    pub cwd: String,
    pub locale: String,
    pub now: String,
    pub command_mode: bool,
    pub use_markdown: bool,
    pub use_colors: bool,
}

impl PromptContext {
    pub fn from_env(command_mode: bool, use_markdown: bool, use_colors: bool) -> Self {
        Self {
            os: std::env::consts::OS.to_string(),
            shell: std::env::var("SHELL").unwrap_or_else(|_| "sh".to_string()),
            cwd: std::env::current_dir()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| ".".to_string()),
            locale: std::env::var("LANG").unwrap_or_else(|_| "en_US.UTF-8".to_string()),
            now: chrono::Local::now().format("%Y-%m-%d %H:%M").to_string(),
            command_mode,
            use_markdown,
            use_colors,
        }
    }

    fn format_instructions(&self) -> &'static str {
        if self.use_markdown {
            "Use markdown for formatting."
        } else if self.use_colors {
            "Use terminal colors and ANSI formatting when helpful."
        } else {
            "Plain text only, no formatting codes or markdown."
        }
    }
}

/// Generates the system prompt based on context
///
/// This is the central location for prompt engineering. It instructs the model
/// to prefer single-line commands and handle intent detection.
pub fn build_unified_prompt(ctx: &PromptContext) -> String {
    let command_emphasis = if ctx.command_mode {
        "IMPORTANT: User explicitly requested command mode. Return ONLY the shell command, nothing else.\n\n"
    } else {
        ""
    };

    let format_instructions = ctx.format_instructions();

    format!(
        r#"{command_emphasis}You are a helpful CLI assistant. Respond in the user's language based on locale ({locale}).

INTENT DETECTION:
- If the user asks for a shell command (e.g., "list files", "delete logs", "show disk usage"), return ONLY the command
  - No explanations, no markdown, no code blocks, no backticks
  - Use && for multiple commands, \ for line continuation
  - NEVER use newlines in commands
- If it's a question or informational request, be brief (1-3 sentences max)
- If user wants code, provide concise code with minimal explanation

Context: OS={os}, shell={shell}, cwd={cwd}, locale={locale}, now={now}
{format_instructions}"#,
        command_emphasis = command_emphasis,
        locale = ctx.locale,
        os = ctx.os,
        shell = ctx.shell,
        cwd = ctx.cwd,
        now = ctx.now,
        format_instructions = format_instructions
    )
}

pub const DEFAULT_PROMPT_TEMPLATE: &str = r#"You are a helpful CLI assistant. Respond in the user's language based on locale ({locale}).

INTENT DETECTION:
- If the user asks for a shell command (e.g., "list files", "delete logs", "show disk usage"), return ONLY the command
  - No explanations, no markdown, no code blocks, no backticks
  - Use && for multiple commands, \ for line continuation
  - NEVER use newlines in commands
- If it's a question or informational request, be brief (1-3 sentences max)
- If user wants code, provide concise code with minimal explanation

Context: OS={os}, shell={shell}, cwd={cwd}, locale={locale}, now={now}
{format}
"#;

pub fn load_custom_prompt(command_name: Option<&str>) -> Option<String> {
    use crate::config::loader::find_recursive_file;
    use std::path::PathBuf;

    let home = dirs::home_dir();
    let config_dir = dirs::config_dir().map(|p| p.join("ask"));

    let local_file = if let Some(cmd) = command_name {
        let filename = format!("ask.{}.md", cmd);
        let dot_filename = format!(".ask.{}.md", cmd);
        find_recursive_file(&[&filename, &dot_filename])
    } else {
        find_recursive_file(&["ask.md", ".ask.md"])
    };

    if let Some(path) = local_file {
        if let Ok(content) = std::fs::read_to_string(&path) {
            return Some(content);
        }
    }

    // Fallback to home and config dir
    let search_paths: Vec<PathBuf> = if let Some(cmd) = command_name {
        let filename = format!("ask.{}.md", cmd);
        vec![
            home.clone().map(|h| h.join(&filename)).unwrap_or_default(),
            config_dir
                .clone()
                .map(|c| c.join(&filename))
                .unwrap_or_default(),
        ]
    } else {
        vec![
            home.clone().map(|h| h.join("ask.md")).unwrap_or_default(),
            config_dir
                .clone()
                .map(|c| c.join("ask.md"))
                .unwrap_or_default(),
        ]
    };

    for path in search_paths {
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                return Some(content);
            }
        }
    }

    None
}

pub fn expand_prompt_variables(template: &str, ctx: &PromptContext) -> String {
    template
        .replace("{os}", &ctx.os)
        .replace("{shell}", &ctx.shell)
        .replace("{cwd}", &ctx.cwd)
        .replace("{locale}", &ctx.locale)
        .replace("{now}", &ctx.now)
        .replace("{format}", ctx.format_instructions())
}
