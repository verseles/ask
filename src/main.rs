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

    // Check for update notification from previous run
    let _ = update::check_and_show_notification();

    // Spawn background update check
    update::check_updates_background();

    run().await
}
