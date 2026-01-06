//! Context storage using JSON files (simpler than Native DB for initial implementation)

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A stored context entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextEntry {
    pub id: String,
    pub pwd: String,
    pub messages: Vec<StoredMessage>,
    pub created_at: DateTime<Utc>,
    pub last_used: DateTime<Utc>,
}

/// A stored message in context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredMessage {
    pub role: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

/// Context storage backend
pub struct ContextStorage {
    storage_path: PathBuf,
}

impl ContextStorage {
    pub fn new(storage_path: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&storage_path)?;
        Ok(Self { storage_path })
    }

    /// Get the file path for a context ID
    fn context_file(&self, id: &str) -> PathBuf {
        self.storage_path.join(format!("{}.json", id))
    }

    /// Load a context by ID
    pub fn load(&self, id: &str) -> Result<Option<ContextEntry>> {
        let path = self.context_file(id);
        if !path.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&path)?;
        let entry: ContextEntry = serde_json::from_str(&content)?;
        Ok(Some(entry))
    }

    /// Save a context
    pub fn save(&self, entry: &ContextEntry) -> Result<()> {
        let path = self.context_file(&entry.id);
        let content = serde_json::to_string_pretty(entry)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Delete a context
    pub fn delete(&self, id: &str) -> Result<()> {
        let path = self.context_file(id);
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        Ok(())
    }

    /// List all contexts
    pub fn list(&self) -> Result<Vec<ContextEntry>> {
        let mut entries = Vec::new();

        for entry in std::fs::read_dir(&self.storage_path)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().map(|e| e == "json").unwrap_or(false) {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(ctx) = serde_json::from_str::<ContextEntry>(&content) {
                        entries.push(ctx);
                    }
                }
            }
        }

        Ok(entries)
    }

    /// Clean up expired contexts
    pub fn cleanup(&self, max_age_minutes: u64) -> Result<usize> {
        let now = Utc::now();
        let mut cleaned = 0;

        for entry in self.list()? {
            let age = now.signed_duration_since(entry.last_used);
            if age.num_minutes() as u64 > max_age_minutes {
                self.delete(&entry.id)?;
                cleaned += 1;
            }
        }

        Ok(cleaned)
    }
}
