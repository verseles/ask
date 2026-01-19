//! Provider integrations for various AI APIs

mod anthropic;
mod gemini;
mod openai;
mod traits;

pub use anthropic::AnthropicProvider;
pub use gemini::GeminiProvider;
pub use openai::OpenAIProvider;
pub use traits::*;

use crate::config::Config;
use anyhow::{anyhow, Result};

/// List of common command prefixes used to detect if a line is a shell command.
const COMMAND_STARTERS: &[&str] = &[
    "ls",
    "cd",
    "rm",
    "cp",
    "mv",
    "mkdir",
    "touch",
    "cat",
    "echo",
    "grep",
    "find",
    "chmod",
    "chown",
    "sudo",
    "apt",
    "yum",
    "dnf",
    "brew",
    "npm",
    "yarn",
    "pnpm",
    "cargo",
    "git",
    "docker",
    "podman",
    "kubectl",
    "systemctl",
    "service",
    "curl",
    "wget",
    "tar",
    "zip",
    "unzip",
    "ssh",
    "scp",
    "rsync",
    "ps",
    "kill",
    "top",
    "htop",
    "df",
    "du",
    "free",
    "ping",
    "traceroute",
    "netstat",
    "ss",
    "iptables",
    "ufw",
    "python",
    "python3",
    "node",
    "ruby",
    "perl",
    "php",
    "java",
    "go",
    "rustc",
    "gcc",
    "g++",
    "make",
    "cmake",
    "sed",
    "awk",
    "sort",
    "head",
    "tail",
    "wc",
    "xargs",
    "tee",
    "diff",
    "patch",
    "ln",
    "readlink",
    "basename",
    "dirname",
    "env",
    "export",
    "source",
    ".",
    "exec",
    "nohup",
    "timeout",
    "watch",
    "sleep",
    "date",
    "cal",
    "whoami",
    "id",
    "groups",
    "passwd",
    "useradd",
    "usermod",
    "groupadd",
    "crontab",
    "at",
    "journalctl",
    "dmesg",
    "lsof",
    "strace",
    "ltrace",
    "gdb",
    "valgrind",
    "perf",
    "time",
    "./",
    "/",
    "~",
];

/// Checks if a line starts with a known command.
fn line_starts_with_command(line: &str) -> bool {
    let first_word = line.split_whitespace().next().unwrap_or("");
    COMMAND_STARTERS
        .iter()
        .any(|cmd| first_word.starts_with(cmd))
}

/// Attempts to flatten a multi-line command response into a single line.
///
/// Returns `Some(flattened)` only when it's safe to join lines with `&&`.
/// Returns `None` if the text contains patterns that would break if flattened:
/// - Line continuations (ending with `\`)
/// - Heredocs (`<<`)
/// - Lines that don't look like commands
/// - Lines that are too long (likely a single wrapped command)
///
/// Join with `&&` is compatible with sh, bash, zsh, and fish 3.0+.
pub fn flatten_command_if_safe(text: &str) -> Option<String> {
    let trimmed = text.trim();

    // Already a single line - return as-is
    if !trimmed.contains('\n') {
        return Some(trimmed.to_string());
    }

    // Split by lines and filter empty ones
    let lines: Vec<&str> = trimmed
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();

    // Single effective line after filtering
    if lines.len() <= 1 {
        return Some(trimmed.replace('\n', " ").trim().to_string());
    }

    // Safety checks for each line
    for line in &lines {
        // Line continuation - don't flatten
        if line.ends_with('\\') {
            return None;
        }
        // Heredoc - don't flatten
        if line.contains("<<") {
            return None;
        }
        // Line too long - likely a wrapped single command
        if line.len() > 120 {
            return None;
        }
        // Must look like a command
        if !line_starts_with_command(line) {
            return None;
        }
    }

    // Safe to flatten
    Some(lines.join(" && "))
}

/// Create a provider based on configuration
pub fn create_provider(config: &Config) -> Result<Box<dyn Provider>> {
    let provider_name = config.active_provider();
    let model = config.active_model().to_string();

    let api_key = config.api_key().ok_or_else(|| {
        anyhow!(
            "No API key found for provider '{}'. Run 'ask init' to configure.",
            provider_name
        )
    })?;

    match provider_name {
        "gemini" => {
            let base_url = config
                .base_url()
                .unwrap_or_else(|| crate::config::DEFAULT_GEMINI_BASE_URL.to_string());
            Ok(Box::new(GeminiProvider::new(api_key, base_url, model)))
        }
        "openai" | "openai_compatible" => {
            let base_url = config
                .base_url()
                .unwrap_or_else(|| crate::config::DEFAULT_OPENAI_BASE_URL.to_string());
            Ok(Box::new(OpenAIProvider::new(api_key, base_url, model)))
        }
        "anthropic" | "claude" => {
            let base_url = config
                .base_url()
                .unwrap_or_else(|| crate::config::DEFAULT_ANTHROPIC_BASE_URL.to_string());
            Ok(Box::new(AnthropicProvider::new(api_key, base_url, model)))
        }
        _ => Err(anyhow!("Unknown provider: {}", provider_name)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flatten_command_if_safe_single_line() {
        // Single line remains unchanged
        assert_eq!(
            flatten_command_if_safe("ls -la"),
            Some("ls -la".to_string())
        );
    }

    #[test]
    fn test_flatten_command_if_safe_valid_multiline() {
        // Multi-line commands joined with &&
        assert_eq!(
            flatten_command_if_safe("mkdir test\ncd test\ntouch hello.txt"),
            Some("mkdir test && cd test && touch hello.txt".to_string())
        );

        // Extra whitespace and empty lines handled
        assert_eq!(
            flatten_command_if_safe("  apt update  \n\n  apt upgrade  "),
            Some("apt update && apt upgrade".to_string())
        );
    }

    #[test]
    fn test_flatten_command_if_safe_line_continuation() {
        // Line continuation should NOT be flattened
        assert_eq!(
            flatten_command_if_safe("docker run \\\n  --name test \\\n  nginx"),
            None
        );
    }

    #[test]
    fn test_flatten_command_if_safe_heredoc() {
        // Heredoc should NOT be flattened
        assert_eq!(flatten_command_if_safe("cat <<EOF\nhello world\nEOF"), None);
    }

    #[test]
    fn test_flatten_command_if_safe_non_command_line() {
        // Text that doesn't look like commands should NOT be flattened
        assert_eq!(
            flatten_command_if_safe("ls -la\nThis is not a command"),
            None
        );
    }

    #[test]
    fn test_flatten_command_if_safe_long_line() {
        // Very long lines should NOT be flattened (likely wrapped single command)
        let long_line = format!("echo {}", "x".repeat(130));
        assert_eq!(
            flatten_command_if_safe(&format!("ls -la\n{}", long_line)),
            None
        );
    }
}
