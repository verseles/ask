//! Context manager - handles conversation context per directory

use super::storage::{ContextEntry, ContextStorage, StoredMessage};
use crate::config::Config;
use crate::providers::Message;
use anyhow::Result;
use chrono::Utc;
use colored::Colorize;
use sha2::{Digest, Sha256};

/// Context statistics for echo display
#[derive(Debug, Clone)]
pub struct ContextStats {
    pub message_count: usize,
    pub total_chars: usize,
}

impl ContextStats {
    /// Check if context should show echo (> 500 chars OR > 3 messages)
    pub fn should_show_echo(&self) -> bool {
        self.total_chars > 500 || self.message_count > 3
    }
}

/// Manages conversation context for the current directory
pub struct ContextManager {
    storage: ContextStorage,
    context_id: String,
    max_messages: usize,
    max_age_minutes: u64,
}

#[allow(dead_code)]
impl ContextManager {
    pub fn new(config: &Config) -> Result<Self> {
        Self::with_ttl(config, config.context.max_age_minutes)
    }

    /// Create with custom TTL (0 = permanent, no cleanup)
    pub fn with_ttl(config: &Config, ttl_minutes: u64) -> Result<Self> {
        let storage_path = config.context_storage_path();
        let storage = ContextStorage::new(storage_path)?;

        // Create context ID from current directory
        let pwd = std::env::current_dir()?.to_string_lossy().to_string();
        let context_id = Self::hash_pwd(&pwd);

        // Run cleanup only if TTL > 0 (not permanent)
        if ttl_minutes > 0 {
            let _ = storage.cleanup(ttl_minutes);
        }

        Ok(Self {
            storage,
            context_id,
            max_messages: config.context.max_messages,
            max_age_minutes: ttl_minutes,
        })
    }

    /// Create a hash of the directory path
    fn hash_pwd(pwd: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(pwd.as_bytes());
        let result = hasher.finalize();
        format!("{:x}", result)[..16].to_string()
    }

    /// Get context statistics (message count and total chars)
    pub fn get_stats(&self) -> Result<ContextStats> {
        let entry = self.storage.load(&self.context_id)?;

        Ok(entry
            .map(|e| {
                let message_count = e.messages.len();
                let total_chars: usize = e.messages.iter().map(|m| m.content.len()).sum();
                ContextStats {
                    message_count,
                    total_chars,
                }
            })
            .unwrap_or(ContextStats {
                message_count: 0,
                total_chars: 0,
            }))
    }

    /// Get messages from the current context
    pub fn get_messages(&self) -> Result<Vec<Message>> {
        let entry = self.storage.load(&self.context_id)?;

        Ok(entry
            .map(|e| {
                e.messages
                    .into_iter()
                    .map(|m| Message {
                        role: m.role,
                        content: m.content,
                    })
                    .collect()
            })
            .unwrap_or_default())
    }

    /// Add a message to the current context
    pub fn add_message(&self, role: &str, content: &str) -> Result<()> {
        let pwd = std::env::current_dir()?.to_string_lossy().to_string();

        let mut entry = self
            .storage
            .load(&self.context_id)?
            .unwrap_or_else(|| ContextEntry {
                id: self.context_id.clone(),
                pwd: pwd.clone(),
                messages: Vec::new(),
                created_at: Utc::now(),
                last_used: Utc::now(),
            });

        entry.messages.push(StoredMessage {
            role: role.to_string(),
            content: content.to_string(),
            timestamp: Utc::now(),
        });

        // Trim to max messages
        while entry.messages.len() > self.max_messages {
            entry.messages.remove(0);
        }

        entry.last_used = Utc::now();
        self.storage.save(&entry)?;

        Ok(())
    }

    /// Clear the current context
    pub fn clear_current(&self) -> Result<()> {
        self.storage.delete(&self.context_id)
    }

    /// Get max age in minutes (0 = permanent)
    pub fn max_age_minutes(&self) -> u64 {
        self.max_age_minutes
    }

    /// Show context history
    pub fn show_history(&self) -> Result<()> {
        let entry = self.storage.load(&self.context_id)?;

        match entry {
            Some(ctx) => {
                println!("{} {}", "Context for:".cyan(), ctx.pwd.bright_white());
                println!(
                    "{} {}",
                    "Created:".cyan(),
                    ctx.created_at.format("%Y-%m-%d %H:%M:%S")
                );
                println!(
                    "{} {}",
                    "Last used:".cyan(),
                    ctx.last_used.format("%Y-%m-%d %H:%M:%S")
                );
                println!("{} {}", "Messages:".cyan(), ctx.messages.len());

                // Show TTL info
                if self.max_age_minutes == 0 {
                    println!("{} {}", "TTL:".cyan(), "permanent".green());
                } else {
                    println!("{} {} minutes", "TTL:".cyan(), self.max_age_minutes);
                }
                println!();

                for msg in &ctx.messages {
                    let role_color = match msg.role.as_str() {
                        "user" => msg.role.green(),
                        "assistant" => msg.role.blue(),
                        _ => msg.role.normal(),
                    };

                    println!("[{}] {}", role_color, msg.timestamp.format("%H:%M:%S"));

                    // Truncate long messages
                    let content = if msg.content.len() > 200 {
                        format!("{}...", &msg.content[..200])
                    } else {
                        msg.content.clone()
                    };

                    println!("{}", content.bright_black());
                    println!();
                }
            }
            None => {
                println!("{}", "No context found for current directory.".yellow());
                println!(
                    "{}",
                    "Use 'ask -c <question>' to start a conversation with context.".bright_black()
                );
            }
        }

        Ok(())
    }

    /// List all global context history
    pub fn list_global(config: &Config) -> Result<()> {
        let storage_path = config.context_storage_path();
        let storage = ContextStorage::new(storage_path)?;
        let mut contexts = storage.list()?;

        if contexts.is_empty() {
            println!("{}", "No global context history found.".yellow());
            return Ok(());
        }

        // Sort by last used (descending)
        contexts.sort_by(|a, b| b.last_used.cmp(&a.last_used));

        println!(
            "{}",
            format!("Global History ({} contexts)", contexts.len())
                .cyan()
                .bold()
        );
        println!();

        let current_dir = std::env::current_dir()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        for ctx in contexts {
            let msg_count = ctx.messages.len();
            let total_chars: usize = ctx.messages.iter().map(|m| m.content.len()).sum();

            // Format time
            let time_str = ctx.last_used.format("%Y-%m-%d %H:%M:%S").to_string();

            let is_current = ctx.pwd == current_dir;
            let pwd_display = if is_current {
                ctx.pwd.green().bold()
            } else {
                ctx.pwd.white()
            };

            let marker = if is_current { "* " } else { "  " };

            println!(
                "{}{} {} {} {}",
                marker.green(),
                ctx.id[..8].bright_black(),
                time_str.blue(),
                pwd_display,
                format!("({} msgs, {} chars)", msg_count, total_chars).bright_black(),
            );
        }

        println!();
        println!(
            "{}",
            "Use 'ask -c <question>' to use context in the current directory.".bright_black()
        );

        Ok(())
    }

    /// Print context echo if stats exceed threshold
    pub fn print_echo_if_needed(&self) -> Result<()> {
        let stats = self.get_stats()?;
        if stats.should_show_echo() {
            let ttl_info = if self.max_age_minutes == 0 {
                "permanent".to_string()
            } else {
                format!("{} min TTL", self.max_age_minutes)
            };
            eprintln!(
                "{}",
                format!(
                    "(context: {} msgs, {} chars, {} - use -c --clear to reset)",
                    stats.message_count, stats.total_chars, ttl_info
                )
                .bright_black()
            );
        }
        Ok(())
    }
}
