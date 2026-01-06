//! Command execution with output capture

use super::SafetyAnalyzer;
use crate::config::Config;
use anyhow::Result;
use colored::Colorize;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

/// Command executor with safety checks
pub struct CommandExecutor {
    analyzer: SafetyAnalyzer,
    #[allow(dead_code)]
    confirm_destructive: bool,
}

impl CommandExecutor {
    pub fn new(config: &Config) -> Self {
        Self {
            analyzer: SafetyAnalyzer::new(),
            confirm_destructive: config.behavior.confirm_destructive,
        }
    }

    /// Check if command is safe for auto-execution
    pub fn is_safe(&self, command: &str) -> bool {
        self.analyzer.is_safe(command)
    }

    /// Check if command is destructive
    pub fn is_destructive(&self, command: &str) -> bool {
        self.analyzer.is_destructive(command)
    }

    /// Execute a command with optional output following
    pub async fn execute(&self, command: &str, follow: bool) -> Result<i32> {
        println!("{}", "Executing...".cyan());

        // Determine shell
        let shell = if cfg!(windows) { "cmd" } else { "sh" };
        let shell_arg = if cfg!(windows) { "/C" } else { "-c" };

        let mut child = Command::new(shell)
            .arg(shell_arg)
            .arg(command)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let exit_code = if follow {
            // Stream output in real-time
            let stdout = child.stdout.take().unwrap();
            let stderr = child.stderr.take().unwrap();

            let stdout_reader = BufReader::new(stdout);
            let stderr_reader = BufReader::new(stderr);

            let mut stdout_lines = stdout_reader.lines();
            let mut stderr_lines = stderr_reader.lines();

            // Process output
            loop {
                tokio::select! {
                    line = stdout_lines.next_line() => {
                        match line {
                            Ok(Some(line)) => println!("{}", line),
                            Ok(None) => break,
                            Err(e) => eprintln!("{}: {}", "Error".red(), e),
                        }
                    }
                    line = stderr_lines.next_line() => {
                        match line {
                            Ok(Some(line)) => eprintln!("{}", line.red()),
                            Ok(None) => {}
                            Err(e) => eprintln!("{}: {}", "Error".red(), e),
                        }
                    }
                }
            }

            // Wait for process to complete
            let status = child.wait().await?;
            status.code().unwrap_or(1)
        } else {
            // Just wait for completion
            let output = child.wait_with_output().await?;
            output.status.code().unwrap_or(1)
        };

        // Show result
        if exit_code == 0 {
            println!("{}", "Done".green());
        } else {
            println!("{} (exit code: {})", "Failed".red(), exit_code);
        }

        Ok(exit_code)
    }

    #[allow(dead_code)]
    pub async fn execute_with_confirm(
        &self,
        command: &str,
        auto_yes: bool,
        follow: bool,
    ) -> Result<i32> {
        if !auto_yes && self.is_destructive(command) && self.confirm_destructive {
            println!(
                "{} {}",
                "Warning:".yellow().bold(),
                "This command may be destructive!".yellow()
            );
            println!("{}", command.bright_white());

            // Ask for confirmation
            let confirm = dialoguer::Confirm::new()
                .with_prompt("Execute anyway?")
                .default(false)
                .interact()?;

            if !confirm {
                println!("{}", "Cancelled.".yellow());
                return Ok(1);
            }
        }

        self.execute(command, follow).await
    }
}
