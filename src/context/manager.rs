//! Context manager - handles conversation context per directory

use super::storage::{ContextEntry, ContextStorage, StoredMessage};
use crate::config::Config;
use crate::providers::Message;
use anyhow::{bail, Result};
use chrono::Utc;
use colored::Colorize;
use requestty::Question;
use sha2::{Digest, Sha256};
use std::path::Path;

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

fn sort_contexts_by_recent(contexts: &mut [ContextEntry]) {
    contexts.sort_by(|a, b| b.last_used.cmp(&a.last_used).then_with(|| a.id.cmp(&b.id)));
}

fn truncate_chars(text: &str, max_chars: usize) -> String {
    if text.chars().count() > max_chars {
        let truncated: String = text.chars().take(max_chars).collect();
        format!("{}...", truncated)
    } else {
        text.to_string()
    }
}

fn excerpt_around(text: &str, start: usize, match_len: usize, max_chars: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    let char_start = text[..start].chars().count();
    let match_chars = text[start..start + match_len].chars().count();
    let window_start = char_start.saturating_sub(max_chars / 3);
    let window_end = (char_start + match_chars + (max_chars / 2)).min(chars.len());
    let mut snippet: String = chars[window_start..window_end].iter().collect();

    if window_start > 0 {
        snippet = format!("...{}", snippet);
    }

    if window_end < chars.len() {
        snippet.push_str("...");
    }

    snippet
}

fn first_matching_snippet(text: &str, query: &str) -> Option<String> {
    if query.is_empty() {
        return None;
    }

    if let Some((index, matched)) = text.match_indices(query).next() {
        return Some(excerpt_around(text, index, matched.len(), 140));
    }

    let lowercase_query = query.to_lowercase();
    if text.to_lowercase().contains(&lowercase_query) {
        return Some(truncate_chars(text, 140));
    }

    None
}

fn load_all_contexts(config: &Config) -> Result<Vec<ContextEntry>> {
    let storage_path = config.context_storage_path();
    let storage = ContextStorage::new(storage_path)?;
    storage.list()
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
                    let content = truncate_chars(&msg.content, 200);

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

    /// Show specific history by ID or path.
    pub fn show_specific_history(config: &Config, target: &str) -> Result<()> {
        let contexts = load_all_contexts(config)?;

        if contexts.is_empty() {
            println!("{}", "No global context history found.".yellow());
            return Ok(());
        }

        let current_dir = std::env::current_dir()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let search_target = if target == "." {
            current_dir.clone()
        } else if let Ok(abs_path) = std::fs::canonicalize(target) {
            abs_path.to_string_lossy().to_string()
        } else {
            target.to_string()
        };

        let matching_ctx = contexts
            .into_iter()
            .find(|ctx| ctx.id.starts_with(&search_target) || ctx.pwd == search_target);

        match matching_ctx {
            Some(ctx) => {
                println!("{} {}", "Context for:".cyan(), ctx.pwd.bright_white());
                println!("{} {}", "ID:".cyan(), ctx.id.bright_black());
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
                println!();

                for msg in &ctx.messages {
                    let role_color = match msg.role.as_str() {
                        "user" => msg.role.green(),
                        "assistant" => msg.role.blue(),
                        _ => msg.role.normal(),
                    };

                    println!("[{}] {}", role_color, msg.timestamp.format("%H:%M:%S"));

                    let content = truncate_chars(&msg.content, 200);

                    println!("{}", content.bright_black());
                    println!();
                }
            }
            None => {
                println!(
                    "{} '{}'",
                    "No context found matching:".yellow(),
                    search_target.bright_white()
                );
            }
        }

        Ok(())
    }

    /// List all global context history
    pub fn list_global(config: &Config) -> Result<()> {
        let mut contexts = load_all_contexts(config)?;

        if contexts.is_empty() {
            println!("{}", "No global context history found.".yellow());
            return Ok(());
        }

        sort_contexts_by_recent(&mut contexts);

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

    /// Search all saved contexts by path or message content.
    pub fn search_global(config: &Config, query: &str) -> Result<()> {
        let query = query.trim();
        if query.is_empty() {
            bail!("History search requires a query.");
        }

        let mut contexts = load_all_contexts(config)?;

        if contexts.is_empty() {
            println!("{}", "No global context history found.".yellow());
            return Ok(());
        }

        sort_contexts_by_recent(&mut contexts);

        let current_dir = std::env::current_dir()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let lowercase_query = query.to_lowercase();

        let matches: Vec<_> = contexts
            .into_iter()
            .filter_map(|ctx| {
                let path_match = ctx.pwd.to_lowercase().contains(&lowercase_query);
                let snippet = ctx
                    .messages
                    .iter()
                    .find_map(|msg| first_matching_snippet(&msg.content, query));

                if path_match || snippet.is_some() {
                    Some((ctx, path_match, snippet))
                } else {
                    None
                }
            })
            .collect();

        if matches.is_empty() {
            println!(
                "{} '{}'",
                "No history matches found for:".yellow(),
                query.bright_white()
            );
            return Ok(());
        }

        println!(
            "{}",
            format!("History Search ({})", matches.len()).cyan().bold()
        );
        println!("{} {}", "Query:".cyan(), query.bright_white());
        println!();

        for (ctx, path_match, snippet) in matches {
            let marker = if ctx.pwd == current_dir { "* " } else { "  " };
            let pwd_display = if ctx.pwd == current_dir {
                ctx.pwd.green().bold()
            } else {
                ctx.pwd.white()
            };

            println!(
                "{}{} {} {} {}",
                marker.green(),
                ctx.id[..8].bright_black(),
                ctx.last_used.format("%Y-%m-%d %H:%M:%S").to_string().blue(),
                pwd_display,
                format!("({} msgs)", ctx.messages.len()).bright_black(),
            );

            if path_match {
                println!("   {}", "Path matched query.".bright_black());
            }

            if let Some(snippet) = snippet {
                println!("   {}", snippet.bright_black());
            }
        }

        Ok(())
    }

    /// Delete saved contexts whose directories no longer exist.
    pub fn prune_deleted(config: &Config, auto_yes: bool) -> Result<()> {
        let storage_path = config.context_storage_path();
        let storage = ContextStorage::new(storage_path)?;
        let mut contexts = storage.list()?;

        if contexts.is_empty() {
            println!("{}", "No global context history found.".yellow());
            return Ok(());
        }

        sort_contexts_by_recent(&mut contexts);

        let orphaned: Vec<_> = contexts
            .into_iter()
            .filter(|ctx| !Path::new(&ctx.pwd).exists())
            .collect();

        if orphaned.is_empty() {
            println!("{}", "No orphaned contexts found.".green());
            return Ok(());
        }

        println!(
            "{}",
            format!("Orphaned History ({})", orphaned.len())
                .yellow()
                .bold()
        );
        println!();

        for ctx in &orphaned {
            println!(
                "  {} {} {}",
                ctx.id[..8].bright_black(),
                ctx.last_used.format("%Y-%m-%d %H:%M:%S").to_string().blue(),
                ctx.pwd.white(),
            );
        }

        let should_delete = if auto_yes {
            true
        } else {
            println!();
            let question = Question::confirm("prune_history")
                .message(format!("Delete {} orphaned context(s)?", orphaned.len()))
                .default(false)
                .build();
            requestty::prompt_one(question)
                .map(|answer| answer.as_bool().unwrap_or(false))
                .unwrap_or(false)
        };

        if !should_delete {
            println!("{}", "Cancelled.".yellow());
            return Ok(());
        }

        for ctx in &orphaned {
            storage.delete(&ctx.id)?;
        }

        println!();
        println!("{} {}", "Pruned orphaned contexts:".green(), orphaned.len());

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
