//! Command execution with output capture

use super::SafetyAnalyzer;
use crate::config::Config;
use anyhow::Result;
use colored::Colorize;
use std::io::Write;
use std::process::Stdio;
use tokio::io::AsyncReadExt;
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
            let mut stdout = child.stdout.take().unwrap();
            let mut stderr = child.stderr.take().unwrap();

            let mut stdout_buf = [0u8; 1024];
            let mut stderr_buf = [0u8; 1024];
            let mut stdout_done = false;
            let mut stderr_done = false;

            let mut stdout_leftover = Vec::new();
            let mut stderr_leftover = Vec::new();

            // Process output
            while !stdout_done || !stderr_done {
                tokio::select! {
                    res = stdout.read(&mut stdout_buf), if !stdout_done => {
                        match res {
                            Ok(0) => {
                                stdout_done = true;
                                if !stdout_leftover.is_empty() {
                                    print!("{}", String::from_utf8_lossy(&stdout_leftover));
                                    std::io::stdout().flush().unwrap_or(());
                                }
                            }
                            Ok(n) => {
                                let chunk = &stdout_buf[..n];
                                stdout_leftover.extend_from_slice(chunk);

                                match std::str::from_utf8(&stdout_leftover) {
                                    Ok(s) => {
                                        print!("{}", s);
                                        stdout_leftover.clear();
                                    }
                                    Err(e) => {
                                        let valid_len = e.valid_up_to();
                                        if valid_len > 0 {
                                            // SAFETY: We just checked that this portion is valid UTF-8
                                            let valid_str = unsafe { std::str::from_utf8_unchecked(&stdout_leftover[..valid_len]) };
                                            print!("{}", valid_str);
                                        }

                                        if e.error_len().is_none() {
                                            // Incomplete character at the end, keep it for next time
                                            let new_leftover = stdout_leftover[valid_len..].to_vec();
                                            stdout_leftover = new_leftover;
                                        } else {
                                            // Invalid character, fallback to lossy
                                            print!("{}", String::from_utf8_lossy(&stdout_leftover[valid_len..]));
                                            stdout_leftover.clear();
                                        }
                                    }
                                }
                                std::io::stdout().flush().unwrap_or(());
                            }
                            Err(e) => {
                                eprintln!("{}: {}", "Error".red(), e);
                                stdout_done = true;
                            }
                        }
                    }
                    res = stderr.read(&mut stderr_buf), if !stderr_done => {
                        match res {
                            Ok(0) => {
                                stderr_done = true;
                                if !stderr_leftover.is_empty() {
                                    eprint!("{}", String::from_utf8_lossy(&stderr_leftover).red());
                                    std::io::stderr().flush().unwrap_or(());
                                }
                            }
                            Ok(n) => {
                                let chunk = &stderr_buf[..n];
                                stderr_leftover.extend_from_slice(chunk);

                                match std::str::from_utf8(&stderr_leftover) {
                                    Ok(s) => {
                                        eprint!("{}", s.red());
                                        stderr_leftover.clear();
                                    }
                                    Err(e) => {
                                        let valid_len = e.valid_up_to();
                                        if valid_len > 0 {
                                            // SAFETY: We just checked that this portion is valid UTF-8
                                            let valid_str = unsafe { std::str::from_utf8_unchecked(&stderr_leftover[..valid_len]) };
                                            eprint!("{}", valid_str.red());
                                        }

                                        if e.error_len().is_none() {
                                            // Incomplete character at the end, keep it for next time
                                            let new_leftover = stderr_leftover[valid_len..].to_vec();
                                            stderr_leftover = new_leftover;
                                        } else {
                                            // Invalid character, fallback to lossy
                                            let lossy = String::from_utf8_lossy(&stderr_leftover[valid_len..]);
                                            eprint!("{}", lossy.red());
                                            stderr_leftover.clear();
                                        }
                                    }
                                }
                                std::io::stderr().flush().unwrap_or(());
                            }
                            Err(e) => {
                                eprintln!("{}: {}", "Error".red(), e);
                                stderr_done = true;
                            }
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

    /// Execute a command and suggest sudo retry on permission denied
    pub async fn execute_with_sudo_retry(&self, command: &str, follow: bool) -> Result<i32> {
        let exit_code = self.execute(command, follow).await?;

        // Check if it looks like a permission error (common exit codes)
        if exit_code != 0 && !command.starts_with("sudo ") && !cfg!(windows) {
            // Check if we should suggest sudo
            let should_suggest = self.might_need_sudo(command);

            if should_suggest {
                println!();
                println!(
                    "{} {}",
                    "Tip:".yellow().bold(),
                    "Command may require elevated permissions.".yellow()
                );

                let retry = {
                    let question = requestty::Question::confirm("sudo_retry")
                        .message("Retry with sudo?")
                        .default(false)
                        .build();
                    requestty::prompt_one(question)
                        .map(|a| a.as_bool().unwrap_or(false))
                        .unwrap_or(false)
                };

                if retry {
                    let sudo_cmd = format!("sudo {}", command);
                    return self.execute(&sudo_cmd, follow).await;
                }
            }
        }

        Ok(exit_code)
    }

    /// Check if a command might need sudo based on common patterns
    fn might_need_sudo(&self, command: &str) -> bool {
        let sudo_patterns = [
            // Package managers
            "apt ",
            "apt-get ",
            "dnf ",
            "yum ",
            "pacman ",
            "zypper ",
            "apk ",
            // System paths
            "/etc/",
            "/usr/",
            "/var/",
            "/opt/",
            // System commands
            "systemctl ",
            "service ",
            "mount ",
            "umount ",
            "chown ",
            "chmod ",
            "useradd ",
            "userdel ",
            "groupadd ",
            "groupdel ",
            "usermod ",
            // Network
            "iptables ",
            "ip6tables ",
            "nft ",
            "ifconfig ",
            "ip addr",
            "ip link",
            // Other
            "modprobe ",
            "insmod ",
            "rmmod ",
            "fdisk ",
            "parted ",
            "mkfs",
        ];

        for pattern in sudo_patterns {
            if command.contains(pattern) {
                return true;
            }
        }

        false
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
            let confirm = {
                let question = requestty::Question::confirm("execute_destructive")
                    .message("Execute anyway?")
                    .default(false)
                    .build();
                requestty::prompt_one(question)
                    .map(|a| a.as_bool().unwrap_or(false))
                    .unwrap_or(false)
            };

            if !confirm {
                println!("{}", "Cancelled.".yellow());
                return Ok(1);
            }
        }

        self.execute(command, follow).await
    }
}
