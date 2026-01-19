//! Auto-update module - checks GitHub releases and updates the binary

use crate::http::create_client_builder;
use anyhow::{anyhow, Result};
use colored::Colorize;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

const RELEASES_URL: &str = "https://api.github.com/repos/verseles/ask/releases/latest";

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    html_url: String,
    body: Option<String>,
    assets: Vec<GitHubAsset>,
}

#[derive(Debug, Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Debug, Deserialize)]
struct GitHubError {
    message: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct UpdateNotification {
    pub old_version: String,
    pub new_version: String,
    pub changelog: String,
    pub url: String,
    pub timestamp: i64,
}

#[allow(dead_code)]
pub fn should_check_update(aggressive: bool, interval_hours: u64, last_check: Option<i64>) -> bool {
    match last_check {
        None => true,
        Some(timestamp) => {
            let now = chrono::Utc::now().timestamp();
            let elapsed = now - timestamp;
            if aggressive {
                // Limit aggressive checks to once per hour (3600s)
                elapsed >= 3600
            } else {
                // Normal checks respect interval (default 24h)
                elapsed >= (interval_hours * 3600) as i64
            }
        }
    }
}

pub fn format_changelog(changelog: &str, max_lines: usize) -> String {
    changelog
        .lines()
        .take(max_lines)
        .collect::<Vec<_>>()
        .join("\n")
}

/// Get the update notification file path
fn notification_path() -> Result<PathBuf> {
    let data_dir = dirs::data_local_dir()
        .ok_or_else(|| anyhow!("Could not find data directory"))?
        .join("ask");
    fs::create_dir_all(&data_dir)?;
    Ok(data_dir.join("update_notification.json"))
}

/// Get pending update notification if exists
pub fn get_pending_notification() -> Option<UpdateNotification> {
    let path = notification_path().ok()?;
    if !path.exists() {
        return None;
    }

    let content = fs::read_to_string(&path).ok()?;
    let notification: UpdateNotification = serde_json::from_str(&content).ok()?;

    // Check if notification is less than 24 hours old
    let now = chrono::Utc::now().timestamp();
    if now - notification.timestamp > 86400 {
        let _ = fs::remove_file(&path);
        return None;
    }

    // Remove notification after reading
    let _ = fs::remove_file(&path);

    Some(notification)
}

#[allow(dead_code)]
/// Check if an update notification exists and display it
pub fn check_and_show_notification() -> Result<bool> {
    let path = notification_path()?;
    if !path.exists() {
        return Ok(false);
    }

    let content = fs::read_to_string(&path)?;
    let notification: UpdateNotification = serde_json::from_str(&content)?;

    // Check if notification is less than 24 hours old
    let now = chrono::Utc::now().timestamp();
    if now - notification.timestamp > 86400 {
        // Remove old notification
        fs::remove_file(&path)?;
        return Ok(false);
    }

    // Show notification
    println!(
        "{} {} {} {}",
        "Updated:".green().bold(),
        notification.old_version.bright_black(),
        "→".bright_black(),
        notification.new_version.green()
    );

    if !notification.changelog.is_empty() {
        let changelog_preview: String = notification
            .changelog
            .lines()
            .take(3)
            .collect::<Vec<_>>()
            .join("\n");
        if !changelog_preview.is_empty() {
            println!("{}", changelog_preview.bright_black());
        }
    }

    println!();

    // Remove notification after showing
    fs::remove_file(&path)?;

    Ok(true)
}

/// Save update notification for next run
fn save_notification(
    old_version: &str,
    new_version: &str,
    changelog: &str,
    url: &str,
) -> Result<()> {
    let notification = UpdateNotification {
        old_version: old_version.to_string(),
        new_version: new_version.to_string(),
        changelog: changelog.to_string(),
        url: url.to_string(),
        timestamp: chrono::Utc::now().timestamp(),
    };

    let path = notification_path()?;
    let content = serde_json::to_string_pretty(&notification)?;
    fs::write(&path, content)?;
    Ok(())
}

/// Get platform-specific asset name
fn get_asset_name() -> String {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    let extension = if os == "windows" { ".exe" } else { "" };

    format!("ask-{}-{}{}", os, arch, extension)
}

/// Parse version string (removes 'v' prefix if present)
fn parse_version(version: &str) -> &str {
    version.strip_prefix('v').unwrap_or(version)
}

/// Compare versions, returns true if remote is newer
fn is_newer_version(current: &str, remote: &str) -> bool {
    let current = parse_version(current);
    let remote = parse_version(remote);

    // Simple semver comparison
    let current_parts: Vec<u32> = current.split('.').filter_map(|s| s.parse().ok()).collect();
    let remote_parts: Vec<u32> = remote.split('.').filter_map(|s| s.parse().ok()).collect();

    for i in 0..3 {
        let c = current_parts.get(i).unwrap_or(&0);
        let r = remote_parts.get(i).unwrap_or(&0);
        if r > c {
            return true;
        }
        if r < c {
            return false;
        }
    }
    false
}

/// Check for updates in background (non-blocking)
pub fn check_updates_background(aggressive: bool, interval_hours: u64) {
    if std::env::var("ASK_NO_UPDATE").is_ok() {
        return;
    }

    let data_dir = match dirs::data_local_dir() {
        Some(d) => d.join("ask"),
        None => return,
    };

    let last_check_file = data_dir.join("last_update_check");
    let last_check = if last_check_file.exists() {
        fs::read_to_string(&last_check_file)
            .ok()
            .and_then(|content| content.trim().parse::<i64>().ok())
    } else {
        None
    };

    if !should_check_update(aggressive, interval_hours, last_check) {
        return;
    }

    // Spawn background process
    let exe = match std::env::current_exe() {
        Ok(e) => e,
        Err(_) => return,
    };

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        use std::process::Command;

        let _ = Command::new(&exe)
            .arg("--update-check-background")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .process_group(0)
            .spawn();
    }

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        use std::process::Command;

        const DETACHED_PROCESS: u32 = 0x00000008;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        let _ = Command::new(&exe)
            .arg("--update-check-background")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .creation_flags(DETACHED_PROCESS | CREATE_NO_WINDOW)
            .spawn();
    }
}

/// Background update check (called from spawned process)
pub async fn background_update_check() -> Result<()> {
    let current_version = env!("CARGO_PKG_VERSION");

    // Update last check time
    let data_dir = dirs::data_local_dir()
        .ok_or_else(|| anyhow!("No data dir"))?
        .join("ask");
    fs::create_dir_all(&data_dir)?;
    let last_check_file = data_dir.join("last_update_check");
    fs::write(&last_check_file, chrono::Utc::now().timestamp().to_string())?;

    // Fetch latest release
    let client = create_client_builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent(format!("ask/{}", current_version))
        .build()?;

    let response = client.get(RELEASES_URL).send().await?;
    let status = response.status();
    let body = response.text().await?;

    // Check for API errors (rate limit, not found, etc.)
    if !status.is_success() {
        if let Ok(error) = serde_json::from_str::<GitHubError>(&body) {
            return Err(anyhow!("GitHub API error: {}", error.message));
        }
        return Err(anyhow!("GitHub API error: HTTP {}", status));
    }

    let release: GitHubRelease =
        serde_json::from_str(&body).map_err(|e| anyhow!("Failed to parse release info: {}", e))?;

    let remote_version = parse_version(&release.tag_name);

    if !is_newer_version(current_version, remote_version) {
        return Ok(());
    }

    // Find matching asset
    let asset_name = get_asset_name();
    let asset = release
        .assets
        .iter()
        .find(|a| a.name == asset_name)
        .ok_or_else(|| anyhow!("No matching asset found: {}", asset_name))?;

    // Download update
    let response = client.get(&asset.browser_download_url).send().await?;
    let bytes = response.bytes().await?;

    // Get current executable path
    let current_exe = std::env::current_exe()?;

    // Create temp file
    let temp_path = current_exe.with_extension("new");

    // Write new binary
    fs::write(&temp_path, &bytes)?;

    // Set executable permission on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&temp_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&temp_path, perms)?;
    }

    // Replace binary
    #[cfg(unix)]
    {
        fs::rename(&temp_path, &current_exe)?;
    }

    #[cfg(windows)]
    {
        let backup_path = current_exe.with_extension("old");
        let _ = fs::remove_file(&backup_path);
        fs::rename(&current_exe, &backup_path)?;
        fs::rename(&temp_path, &current_exe)?;
        let _ = fs::remove_file(&backup_path);
    }

    // Save notification
    let changelog = release.body.unwrap_or_default();
    save_notification(
        current_version,
        remote_version,
        &changelog,
        &release.html_url,
    )?;

    Ok(())
}

/// Interactive update check and install
pub async fn check_and_update() -> Result<()> {
    let current_version = env!("CARGO_PKG_VERSION");

    println!("{}", "Checking for updates...".cyan());

    let client = create_client_builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent(format!("ask/{}", current_version))
        .build()?;

    let response = client.get(RELEASES_URL).send().await?;
    let status = response.status();
    let body = response.text().await?;

    // Check for API errors (rate limit, not found, etc.)
    if !status.is_success() {
        if let Ok(error) = serde_json::from_str::<GitHubError>(&body) {
            return Err(anyhow!("GitHub API error: {}", error.message));
        }
        return Err(anyhow!("GitHub API error: HTTP {}", status));
    }

    let release: GitHubRelease =
        serde_json::from_str(&body).map_err(|e| anyhow!("Failed to parse release info: {}", e))?;

    let remote_version = parse_version(&release.tag_name);

    if !is_newer_version(current_version, remote_version) {
        println!(
            "{} {} {}",
            "Already up to date:".green(),
            current_version,
            "(latest)".bright_black()
        );
        return Ok(());
    }

    println!(
        "{} {} {} {}",
        "Update available:".yellow(),
        current_version,
        "→".bright_black(),
        remote_version.green()
    );

    // Show changelog preview
    if let Some(ref body) = release.body {
        let preview: String = body.lines().take(5).collect::<Vec<_>>().join("\n");
        if !preview.is_empty() {
            println!();
            println!("{}", preview.bright_black());
            println!();
        }
    }

    // Ask for confirmation
    let confirm = {
        let question = requestty::Question::confirm("install_update")
            .message("Install update?")
            .default(true)
            .build();
        requestty::prompt_one(question)?.as_bool().unwrap_or(false)
    };

    if !confirm {
        println!("{}", "Update cancelled.".yellow());
        return Ok(());
    }

    // Find matching asset
    let asset_name = get_asset_name();
    let asset = release
        .assets
        .iter()
        .find(|a| a.name == asset_name)
        .ok_or_else(|| anyhow!("No matching asset for your platform: {}", asset_name))?;

    println!("{} {}", "Downloading:".cyan(), asset.name.bright_white());

    // Download with longer timeout for large binary
    let download_client = create_client_builder()
        .timeout(std::time::Duration::from_secs(300))
        .user_agent(format!("ask/{}", current_version))
        .build()?;

    let response = download_client
        .get(&asset.browser_download_url)
        .send()
        .await?;
    let total_size = response.content_length().unwrap_or(0);

    let pb = indicatif::ProgressBar::new(total_size);
    pb.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
            .unwrap()
            .progress_chars("#>-"),
    );

    let bytes = response.bytes().await?;
    pb.finish_and_clear();

    // Get current executable path
    let current_exe = std::env::current_exe()?;

    // Create temp file
    let temp_path = current_exe.with_extension("new");

    // Write new binary
    fs::write(&temp_path, &bytes)?;

    // Set executable permission on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&temp_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&temp_path, perms)?;
    }

    // Replace binary
    #[cfg(unix)]
    {
        fs::rename(&temp_path, &current_exe)?;
    }

    #[cfg(windows)]
    {
        let backup_path = current_exe.with_extension("old");
        let _ = fs::remove_file(&backup_path);
        fs::rename(&current_exe, &backup_path)?;
        fs::rename(&temp_path, &current_exe)?;
        let _ = fs::remove_file(&backup_path);
    }

    println!(
        "{} {} → {}",
        "Updated!".green().bold(),
        current_version,
        remote_version.green()
    );

    println!();
    println!(
        "{}",
        format!("Release notes: {}", release.html_url).bright_black()
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_newer_version() {
        assert!(is_newer_version("0.14.4", "0.15.0"));
        assert!(is_newer_version("0.14.4", "0.14.5"));
        assert!(is_newer_version("1.0.0", "2.0.0"));
        assert!(!is_newer_version("0.15.0", "0.14.4"));
        assert!(!is_newer_version("0.14.4", "0.14.4"));
        assert!(!is_newer_version("2.0.0", "1.0.0"));
    }

    #[test]
    fn test_parse_version() {
        assert_eq!(parse_version("v0.14.4"), "0.14.4");
        assert_eq!(parse_version("0.14.4"), "0.14.4");
        assert_eq!(parse_version("v1.0.0"), "1.0.0");
    }

    #[test]
    fn test_should_check_update_aggressive() {
        let now = chrono::Utc::now().timestamp();
        assert!(should_check_update(true, 24, None));
        assert!(should_check_update(true, 24, Some(0)));
        assert!(should_check_update(true, 24, Some(now - 3601)));
        assert!(!should_check_update(true, 24, Some(now - 3599)));
    }

    #[test]
    fn test_should_check_update_normal() {
        let now = chrono::Utc::now().timestamp();
        assert!(!should_check_update(false, 24, Some(now)));
        assert!(!should_check_update(false, 24, Some(now - 3600)));
        assert!(should_check_update(false, 24, Some(now - 86401)));
        assert!(should_check_update(false, 24, None));

        // Custom interval
        assert!(should_check_update(false, 1, Some(now - 3601)));
        assert!(!should_check_update(false, 1, Some(now - 3599)));
    }

    #[test]
    fn test_format_changelog() {
        let changelog = "Line 1\nLine 2\nLine 3\nLine 4\nLine 5";
        assert_eq!(format_changelog(changelog, 3), "Line 1\nLine 2\nLine 3");
        assert_eq!(format_changelog(changelog, 10), changelog);
        assert_eq!(format_changelog("", 5), "");
    }

    #[test]
    fn test_get_asset_name() {
        let name = get_asset_name();
        assert!(name.starts_with("ask-"));
    }
}
