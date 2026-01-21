use anyhow::Result;
use std::process::Command;

/// Injection method detection result
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InjectionMethod {
    /// GUI paste via clipboard + key simulation (Wayland/X11/macOS/Windows)
    GuiPaste,
    /// Inside tmux session - use `tmux send-keys`
    TmuxSendKeys,
    /// Inside screen session - use `screen -X stuff`
    ScreenStuff,
    /// Headless terminal without multiplexer - enhanced fallback
    Fallback,
}

/// Detect the best injection method for the current environment
/// Priority: GUI paste > tmux/screen (only in headless) > fallback
pub fn detect_injection_method() -> InjectionMethod {
    // Check for GUI environment first - works everywhere including inside tmux/screen
    #[cfg(target_os = "linux")]
    {
        if std::env::var("DISPLAY").is_ok() || std::env::var("WAYLAND_DISPLAY").is_ok() {
            return InjectionMethod::GuiPaste;
        }
    }

    #[cfg(target_os = "macos")]
    {
        if can_use_accessibility() {
            return InjectionMethod::GuiPaste;
        }
    }

    #[cfg(target_os = "windows")]
    {
        return InjectionMethod::GuiPaste;
    }

    // No GUI available - check for terminal multiplexers (headless SSH scenario)
    if std::env::var("TMUX").is_ok() {
        return InjectionMethod::TmuxSendKeys;
    }

    if std::env::var("STY").is_ok() {
        return InjectionMethod::ScreenStuff;
    }

    // No GUI, no multiplexer - use enhanced fallback
    InjectionMethod::Fallback
}

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
    device.click(KEY_V).map_err(|e| anyhow::anyhow!("{}", e))?;
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

/// Try to inject command using tmux send-keys
fn try_tmux_inject(command: &str) -> Result<Option<String>> {
    // Use send-keys -l for literal text (no key name interpretation)
    let status = Command::new("tmux")
        .args(["send-keys", "-l", "--", command])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();

    match status {
        Ok(s) if s.success() => Ok(None),
        Ok(_) => {
            // tmux failed, fall back to enhanced fallback
            enhanced_fallback(command)
        }
        Err(_) => {
            // tmux not available, fall back
            enhanced_fallback(command)
        }
    }
}

/// Try to inject command using GNU screen
fn try_screen_inject(command: &str) -> Result<Option<String>> {
    // screen -X stuff sends literal characters to the current window
    let status = Command::new("screen")
        .args(["-X", "stuff", command])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();

    match status {
        Ok(s) if s.success() => Ok(None),
        Ok(_) => {
            // screen failed, fall back to enhanced fallback
            enhanced_fallback(command)
        }
        Err(_) => {
            // screen not available, fall back
            enhanced_fallback(command)
        }
    }
}

/// Enhanced fallback for headless terminals: print command with visual highlight, then prompt
fn enhanced_fallback(command: &str) -> Result<Option<String>> {
    use std::io::{self, Write};

    // Check if terminal supports colors (not "dumb")
    let use_colors = std::env::var("TERM").map(|t| t != "dumb").unwrap_or(true);

    let stdout = io::stdout();
    let mut handle = stdout.lock();

    // Print visual separator and command
    if use_colors {
        // Cyan bold for the command, dim for hints
        writeln!(handle)?;
        writeln!(
            handle,
            "\x1b[2m══════════════════════════════════════════════════════════════\x1b[0m"
        )?;
        writeln!(handle, "\x1b[1;36m  {}\x1b[0m", command)?;
        writeln!(
            handle,
            "\x1b[2m══════════════════════════════════════════════════════════════\x1b[0m"
        )?;
        writeln!(
            handle,
            "\x1b[2m[Edit below or press Enter to run, Ctrl+C to cancel]\x1b[0m"
        )?;
    } else {
        writeln!(handle)?;
        writeln!(
            handle,
            "================================================================"
        )?;
        writeln!(handle, "  {}", command)?;
        writeln!(
            handle,
            "================================================================"
        )?;
        writeln!(
            handle,
            "[Edit below or press Enter to run, Ctrl+C to cancel]"
        )?;
    }
    handle.flush()?;

    // Now show the interactive prompt
    interactive_prompt(command)
}

/// Try GUI paste injection (spawns background process)
fn try_gui_paste_inject(command: &str) -> Result<Option<String>> {
    if let Ok(exe) = std::env::current_exe() {
        use std::process::Stdio;
        let child = Command::new(exe)
            .arg("--inject-raw")
            .arg(command)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();

        if child.is_ok() {
            return Ok(None);
        }
    }
    // If spawn failed, use enhanced fallback
    enhanced_fallback(command)
}

pub fn inject_raw_only(command: &str) -> Result<()> {
    let clean_command = command.replace('\n', " && ").replace('\r', "");

    // For raw injection, we only support GUI paste (used by background process)
    #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
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

    match detect_injection_method() {
        InjectionMethod::TmuxSendKeys => try_tmux_inject(&clean_command),
        InjectionMethod::ScreenStuff => try_screen_inject(&clean_command),
        InjectionMethod::GuiPaste => try_gui_paste_inject(&clean_command),
        InjectionMethod::Fallback => enhanced_fallback(&clean_command),
    }
}

pub fn can_inject() -> bool {
    // We can always "inject" now - either via GUI, tmux/screen, or enhanced fallback
    // This function now indicates if automatic injection (without user interaction) is possible
    matches!(
        detect_injection_method(),
        InjectionMethod::GuiPaste | InjectionMethod::TmuxSendKeys | InjectionMethod::ScreenStuff
    )
}
