use anyhow::Result;

/// Save current clipboard content
fn save_clipboard() -> Option<String> {
    arboard::Clipboard::new()
        .ok()
        .and_then(|mut cb| cb.get_text().ok())
}

/// Restore clipboard content after a delay (spawns a thread)
fn restore_clipboard_delayed(previous: Option<String>, delay_ms: u64) {
    if let Some(text) = previous {
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(delay_ms));
            if let Ok(mut cb) = arboard::Clipboard::new() {
                let _ = cb.set_text(&text);
            }
        });
    }
}

#[cfg(target_os = "linux")]
fn can_use_clipboard_paste() -> bool {
    // Check if we have display access for key simulation
    std::env::var("DISPLAY").is_ok() || std::env::var("WAYLAND_DISPLAY").is_ok()
}

#[cfg(target_os = "macos")]
fn can_use_accessibility() -> bool {
    use std::process::Command;
    Command::new("osascript")
        .args(["-e", "tell application \"System Events\" to return 1"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(target_os = "linux")]
fn try_clipboard_paste(command: &str) -> Result<()> {
    use mouse_keyboard_input::key_codes::*;
    use mouse_keyboard_input::VirtualDevice;
    use std::thread;
    use std::time::Duration;

    // Save current clipboard
    let previous_clipboard = save_clipboard();

    // Set command to clipboard
    let mut clipboard = arboard::Clipboard::new().map_err(|e| anyhow::anyhow!("{}", e))?;
    clipboard
        .set_text(command)
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    // Small delay for clipboard to update
    thread::sleep(Duration::from_millis(50));

    // Create virtual device for key simulation
    let mut device = VirtualDevice::default().map_err(|e| anyhow::anyhow!("{}", e))?;

    // Wait a bit before sending keys
    thread::sleep(Duration::from_millis(100));

    // Simulate Ctrl+Shift+V (standard paste in Linux terminals)
    device
        .press(KEY_LEFTCTRL)
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    device
        .press(KEY_LEFTSHIFT)
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    device
        .click(KEY_V)
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    device
        .release(KEY_LEFTSHIFT)
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    device
        .release(KEY_LEFTCTRL)
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    // Restore clipboard after delay
    restore_clipboard_delayed(previous_clipboard, 500);

    Ok(())
}

#[cfg(target_os = "macos")]
fn try_clipboard_paste(command: &str) -> Result<()> {
    use enigo::{Direction, Enigo, Key, Keyboard, Settings};
    use std::thread;
    use std::time::Duration;

    // Save current clipboard
    let previous_clipboard = save_clipboard();

    // Set command to clipboard
    let mut clipboard = arboard::Clipboard::new().map_err(|e| anyhow::anyhow!("{}", e))?;
    clipboard
        .set_text(command)
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    // Small delay for clipboard to update
    thread::sleep(Duration::from_millis(50));

    // Create enigo for key simulation
    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| anyhow::anyhow!("{}", e))?;

    // Wait a bit before sending keys
    thread::sleep(Duration::from_millis(100));

    // Simulate Cmd+V (paste on macOS)
    enigo
        .key(Key::Meta, Direction::Press)
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    enigo
        .key(Key::Unicode('v'), Direction::Click)
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    enigo
        .key(Key::Meta, Direction::Release)
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    // Restore clipboard after delay
    restore_clipboard_delayed(previous_clipboard, 500);

    Ok(())
}

#[cfg(target_os = "windows")]
fn try_clipboard_paste(command: &str) -> Result<()> {
    use enigo::{Direction, Enigo, Key, Keyboard, Settings};
    use std::thread;
    use std::time::Duration;

    // Save current clipboard
    let previous_clipboard = save_clipboard();

    // Set command to clipboard
    let mut clipboard = arboard::Clipboard::new().map_err(|e| anyhow::anyhow!("{}", e))?;
    clipboard
        .set_text(command)
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    // Small delay for clipboard to update
    thread::sleep(Duration::from_millis(50));

    // Create enigo for key simulation
    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| anyhow::anyhow!("{}", e))?;

    // Wait a bit before sending keys
    thread::sleep(Duration::from_millis(100));

    // Simulate Ctrl+V (paste on Windows)
    enigo
        .key(Key::Control, Direction::Press)
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    enigo
        .key(Key::Unicode('v'), Direction::Click)
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    enigo
        .key(Key::Control, Direction::Release)
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    // Restore clipboard after delay
    restore_clipboard_delayed(previous_clipboard, 500);

    Ok(())
}

fn interactive_prompt(command: &str) -> Result<Option<String>> {
    use requestty::Question;

    let question = Question::input("command")
        .message("Command")
        .default(command)
        .build();

    match requestty::prompt_one(question) {
        Ok(answer) => {
            let cmd = answer.as_string().unwrap_or_default();
            if cmd.is_empty() {
                Ok(None)
            } else {
                Ok(Some(cmd.to_string()))
            }
        }
        Err(_) => Ok(None),
    }
}

pub fn inject_raw_only(command: &str) -> Result<()> {
    let clean_command = command.replace('\n', " && ").replace('\r', "");

    #[cfg(target_os = "linux")]
    {
        try_clipboard_paste(&clean_command)
    }

    #[cfg(target_os = "macos")]
    {
        try_clipboard_paste(&clean_command)
    }

    #[cfg(target_os = "windows")]
    {
        try_clipboard_paste(&clean_command)
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    {
        anyhow::bail!("Unsupported platform for raw injection")
    }
}

pub fn inject_command(command: &str) -> Result<Option<String>> {
    let clean_command = command.replace('\n', " && ").replace('\r', "");

    #[cfg(target_os = "linux")]
    {
        if can_use_clipboard_paste() {
            if let Ok(exe) = std::env::current_exe() {
                use std::process::{Command, Stdio};
                let child = Command::new(exe)
                    .arg("--inject-raw")
                    .arg(&clean_command)
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn();

                if child.is_ok() {
                    return Ok(None);
                }
            }
        }
        interactive_prompt(&clean_command)
    }

    #[cfg(target_os = "macos")]
    {
        if can_use_accessibility() {
            if let Ok(exe) = std::env::current_exe() {
                use std::process::{Command, Stdio};
                let child = Command::new(exe)
                    .arg("--inject-raw")
                    .arg(&clean_command)
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn();

                if child.is_ok() {
                    return Ok(None);
                }
            }
        }
        interactive_prompt(&clean_command)
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(exe) = std::env::current_exe() {
            use std::process::{Command, Stdio};
            let child = Command::new(exe)
                .arg("--inject-raw")
                .arg(&clean_command)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();

            if child.is_ok() {
                return Ok(None);
            }
        }
        interactive_prompt(&clean_command)
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    interactive_prompt(&clean_command)
}

pub fn can_inject() -> bool {
    #[cfg(target_os = "linux")]
    {
        std::env::var("DISPLAY").is_ok() || std::env::var("WAYLAND_DISPLAY").is_ok()
    }
    #[cfg(not(target_os = "linux"))]
    {
        true
    }
}
