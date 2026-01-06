//! ask - AI-powered CLI that accepts plain text questions without quotes.
//!
//! Ask anything in plain text, get commands or answers instantly. No quotes needed.

mod cli;
mod config;
mod context;
mod executor;
mod output;
mod providers;

use anyhow::Result;
use cli::run;

#[tokio::main]
async fn main() -> Result<()> {
    run().await
}
