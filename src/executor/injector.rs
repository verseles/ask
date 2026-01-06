use anyhow::Result;

#[cfg(any(
    all(target_os = "linux", target_arch = "x86_64"),
    target_os = "windows",
    target_os = "macos"
))]
pub fn inject_command(command: &str) -> Result<()> {
    let clean_command = command.replace('\n', " && ").replace('\r', "");

    let mut clipboard = arboard::Clipboard::new()?;
    clipboard.set_text(&clean_command)?;

    std::thread::sleep(std::time::Duration::from_millis(50));

    let mut enigo = enigo::Enigo::new(&enigo::Settings::default())?;

    #[cfg(target_os = "linux")]
    {
        use enigo::{Direction, Key, Keyboard};
        enigo.key(Key::Control, Direction::Press)?;
        enigo.key(Key::Shift, Direction::Press)?;
        enigo.key(Key::Unicode('v'), Direction::Click)?;
        enigo.key(Key::Shift, Direction::Release)?;
        enigo.key(Key::Control, Direction::Release)?;
    }

    #[cfg(not(target_os = "linux"))]
    {
        use enigo::{Direction, Key, Keyboard};
        enigo.key(Key::Control, Direction::Press)?;
        enigo.key(Key::Unicode('v'), Direction::Click)?;
        enigo.key(Key::Control, Direction::Release)?;
    }

    Ok(())
}

#[cfg(not(any(
    all(target_os = "linux", target_arch = "x86_64"),
    target_os = "windows",
    target_os = "macos"
)))]
pub fn inject_command(command: &str) -> Result<()> {
    let clean_command = command.replace('\n', " && ").replace('\r', "");

    let mut clipboard = arboard::Clipboard::new()?;
    clipboard.set_text(&clean_command)?;

    println!("Command copied to clipboard. Paste with Ctrl+Shift+V");
    Ok(())
}

#[cfg(any(
    all(target_os = "linux", target_arch = "x86_64"),
    target_os = "windows",
    target_os = "macos"
))]
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

#[cfg(not(any(
    all(target_os = "linux", target_arch = "x86_64"),
    target_os = "windows",
    target_os = "macos"
)))]
pub fn can_inject() -> bool {
    arboard::Clipboard::new().is_ok()
}
