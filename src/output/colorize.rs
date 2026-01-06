#![allow(dead_code)]

use colored::{ColoredString, Colorize};

pub struct ColorScheme;

impl ColorScheme {
    /// Success message (green)
    pub fn success(text: &str) -> ColoredString {
        text.green()
    }

    /// Error message (red)
    pub fn error(text: &str) -> ColoredString {
        text.red()
    }

    /// Warning message (yellow)
    pub fn warning(text: &str) -> ColoredString {
        text.yellow()
    }

    /// Prompt/question (cyan)
    pub fn prompt(text: &str) -> ColoredString {
        text.cyan()
    }

    /// Info message (blue)
    pub fn info(text: &str) -> ColoredString {
        text.blue()
    }

    /// Command text (bright white)
    pub fn command(text: &str) -> ColoredString {
        text.bright_white()
    }

    /// Muted text (bright black/gray)
    pub fn muted(text: &str) -> ColoredString {
        text.bright_black()
    }

    /// Bold text
    pub fn bold(text: &str) -> ColoredString {
        text.bold()
    }

    /// Print a success indicator
    pub fn print_success(message: &str) {
        println!("{} {}", "✓".green(), message);
    }

    /// Print an error indicator
    pub fn print_error(message: &str) {
        eprintln!("{} {}", "✗".red(), message);
    }

    /// Print a warning indicator
    pub fn print_warning(message: &str) {
        println!("{} {}", "⚠".yellow(), message);
    }

    /// Print an info indicator
    pub fn print_info(message: &str) {
        println!("{} {}", "ℹ".blue(), message);
    }
}
