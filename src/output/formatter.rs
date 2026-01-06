use super::markdown::print_markdown;
use crate::cli::Args;
use std::io::IsTerminal;

pub struct OutputFormatter {
    json: bool,
    markdown: bool,
    raw: bool,
    #[allow(dead_code)]
    no_color: bool,
}

impl OutputFormatter {
    pub fn new(args: &Args) -> Self {
        let is_piped = !std::io::stdout().is_terminal();

        Self {
            json: args.json,
            markdown: args.markdown || (!args.raw && !args.json && !is_piped),
            raw: args.raw || is_piped,
            no_color: args.no_color || is_piped,
        }
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
        let output = serde_json::json!({
            "response": text,
            "success": true
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&output).unwrap_or_default()
        );
    }

    fn format_markdown(&self, text: &str) {
        print_markdown(text);
    }

    fn format_raw(&self, text: &str) {
        println!("{}", text);
    }
}
