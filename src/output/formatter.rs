use super::markdown::print_markdown;
use crate::cli::Args;
use crate::update::UpdateNotification;
use std::io::IsTerminal;

#[derive(serde::Serialize)]
struct UpdateInfo {
    from: String,
    to: String,
    changelog: String,
}

pub struct OutputFormatter {
    json: bool,
    markdown: bool,
    raw: bool,
    #[allow(dead_code)]
    no_color: bool,
    update_notification: Option<UpdateNotification>,
}

impl OutputFormatter {
    pub fn new(args: &Args) -> Self {
        let is_piped = !std::io::stdout().is_terminal();

        Self {
            json: args.json,
            markdown: args.markdown || (!args.raw && !args.json && !is_piped),
            raw: args.raw || is_piped,
            no_color: args.no_color || is_piped,
            update_notification: None,
        }
    }

    #[allow(dead_code)]
    pub fn with_update(mut self, notification: Option<UpdateNotification>) -> Self {
        self.update_notification = notification;
        self
    }

    /// Format and print the response
    pub fn format(&self, text: &str) {
        if self.json {
            self.format_json(text);
        } else if self.raw || self.no_color {
            self.format_raw(text);
        } else if self.markdown {
            self.format_markdown(text);
        } else {
            self.format_raw(text);
        }
    }

    fn format_json(&self, text: &str) {
        let update_info = self.update_notification.as_ref().map(|n| UpdateInfo {
            from: n.old_version.clone(),
            to: n.new_version.clone(),
            changelog: n.changelog.clone(),
        });

        let output = if update_info.is_some() {
            serde_json::json!({
                "response": text,
                "success": true,
                "update": update_info
            })
        } else {
            serde_json::json!({
                "response": text,
                "success": true
            })
        };
        println!(
            "{}",
            serde_json::to_string_pretty(&output).unwrap_or_default()
        );
    }

    fn format_markdown(&self, text: &str) {
        print_markdown(text);
    }

    fn format_raw(&self, text: &str) {
        if !self.no_color {
            println!("{}", unescape_ansi(text));
        } else {
            println!("{}", text);
        }
    }
}

fn unescape_ansi(text: &str) -> String {
    text.replace("\\033", "\x1b")
        .replace("\\x1b", "\x1b")
        .replace("\\e", "\x1b")
}
