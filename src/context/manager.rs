//! Context manager - handles conversation context per directory

use super::storage::{ContextEntry, ContextStorage, StoredMessage};
use crate::config::Config;
use crate::providers::Message;
use anyhow::Result;
use chrono::Utc;
use colored::Colorize;
use sha2::{Digest, Sha256};

/// Manages conversation context for the current directory
pub struct ContextManager {
    storage: ContextStorage,
    context_id: String,
    max_messages: usize,
    max_age_minutes: u64,
}

impl ContextManager {
    pub fn new(config: &Config) -> Result<Self> {
        let storage_path = config.context_storage_path();
        let storage = ContextStorage::new(storage_path)?;

        // Create context ID from current directory
        let pwd = std::env::current_dir()?
            .to_string_lossy()
            .to_string();
        let context_id = Self::hash_pwd(&pwd);

        // Run cleanup
        let _ = storage.cleanup(config.context.max_age_minutes);

        Ok(Self {
            storage,
            context_id,
            max_messages: config.context.max_messages,
            max_age_minutes: config.context.max_age_minutes,
        })
    }

    /// Create a hash of the directory path
    fn hash_pwd(pwd: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(pwd.as_bytes());
        let result = hasher.finalize();
        format!("{:x}", result)[..16].to_string()
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
        let pwd = std::env::current_dir()?
            .to_string_lossy()
            .to_string();

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

    /// Show context history
    pub fn show_history(&self) -> Result<()> {
        let entry = self.storage.load(&self.context_id)?;

        match entry {
            Some(ctx) => {
                println!(
                    "{} {}",
                    "Context for:".cyan(),
                    ctx.pwd.bright_white()
                );
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
                println!(
                    "{} {}",
                    "Messages:".cyan(),
                    ctx.messages.len()
                );
                println!();

                for msg in &ctx.messages {
                    let role_color = match msg.role.as_str() {
                        "user" => msg.role.green(),
                        "assistant" => msg.role.blue(),
                        _ => msg.role.normal(),
                    };

                    println!(
                        "[{}] {}",
                        role_color,
                        msg.timestamp.format("%H:%M:%S")
                    );

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
}
