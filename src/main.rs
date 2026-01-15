//! ask - AI-powered CLI that accepts plain text questions without quotes.
//!
//! Ask anything in plain text, get commands or answers instantly. No quotes needed.

mod cli;
mod completions;
mod config;
mod context;
mod executor;
pub mod http;
mod output;
mod providers;
mod update;

use anyhow::Result;
use cli::run;

#[tokio::main]
async fn main() -> Result<()> {
    // Handle background update check (spawned by main process)
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--update-check-background") {
        return update::background_update_check().await;
    }

    // Get pending notification (don't print yet - will be handled by run())
    let update_notification = update::get_pending_notification();

    // Load config to check aggressive mode
    let config = config::Config::load().unwrap_or_default();

    // Spawn background update check
    update::check_updates_background(config.update.aggressive, config.update.check_interval_hours);

    run(update_notification).await
}
