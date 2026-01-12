use anyhow::Result;

#[cfg(target_os = "linux")]
fn can_use_uinput() -> bool {
    use mouse_keyboard_input::VirtualDevice;
    VirtualDevice::default().is_ok()
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
fn try_uinput_inject(command: &str) -> Result<()> {
    use mouse_keyboard_input::key_codes::*;
    use mouse_keyboard_input::VirtualDevice;
    use std::collections::HashMap;
    use std::thread;
    use std::time::Duration;

    let mut device = VirtualDevice::default().map_err(|e| anyhow::anyhow!("{}", e))?;

    thread::sleep(Duration::from_millis(100));

    let key_map: HashMap<char, (u16, bool)> = [
        ('a', (KEY_A, false)),
        ('b', (KEY_B, false)),
        ('c', (KEY_C, false)),
        ('d', (KEY_D, false)),
        ('e', (KEY_E, false)),
        ('f', (KEY_F, false)),
        ('g', (KEY_G, false)),
        ('h', (KEY_H, false)),
        ('i', (KEY_I, false)),
        ('j', (KEY_J, false)),
        ('k', (KEY_K, false)),
        ('l', (KEY_L, false)),
        ('m', (KEY_M, false)),
        ('n', (KEY_N, false)),
        ('o', (KEY_O, false)),
        ('p', (KEY_P, false)),
        ('q', (KEY_Q, false)),
        ('r', (KEY_R, false)),
        ('s', (KEY_S, false)),
        ('t', (KEY_T, false)),
        ('u', (KEY_U, false)),
        ('v', (KEY_V, false)),
        ('w', (KEY_W, false)),
        ('x', (KEY_X, false)),
        ('y', (KEY_Y, false)),
        ('z', (KEY_Z, false)),
        ('A', (KEY_A, true)),
        ('B', (KEY_B, true)),
        ('C', (KEY_C, true)),
        ('D', (KEY_D, true)),
        ('E', (KEY_E, true)),
        ('F', (KEY_F, true)),
        ('G', (KEY_G, true)),
        ('H', (KEY_H, true)),
        ('I', (KEY_I, true)),
        ('J', (KEY_J, true)),
        ('K', (KEY_K, true)),
        ('L', (KEY_L, true)),
        ('M', (KEY_M, true)),
        ('N', (KEY_N, true)),
        ('O', (KEY_O, true)),
        ('P', (KEY_P, true)),
        ('Q', (KEY_Q, true)),
        ('R', (KEY_R, true)),
        ('S', (KEY_S, true)),
        ('T', (KEY_T, true)),
        ('U', (KEY_U, true)),
        ('V', (KEY_V, true)),
        ('W', (KEY_W, true)),
        ('X', (KEY_X, true)),
        ('Y', (KEY_Y, true)),
        ('Z', (KEY_Z, true)),
        ('0', (KEY_10, false)),
        ('1', (KEY_1, false)),
        ('2', (KEY_2, false)),
        ('3', (KEY_3, false)),
        ('4', (KEY_4, false)),
        ('5', (KEY_5, false)),
        ('6', (KEY_6, false)),
        ('7', (KEY_7, false)),
        ('8', (KEY_8, false)),
        ('9', (KEY_9, false)),
        (' ', (KEY_SPACE, false)),
        ('-', (KEY_MINUS, false)),
        ('_', (KEY_MINUS, true)),
        ('=', (KEY_EQUAL, false)),
        ('+', (KEY_EQUAL, true)),
        ('[', (KEY_LEFTBRACE, false)),
        ('{', (KEY_LEFTBRACE, true)),
        (']', (KEY_RIGHTBRACE, false)),
        ('}', (KEY_RIGHTBRACE, true)),
        ('\\', (KEY_BACKSLASH, false)),
        ('|', (KEY_BACKSLASH, true)),
        (';', (KEY_SEMICOLON, false)),
        (':', (KEY_SEMICOLON, true)),
        ('\'', (KEY_APOSTROPHE, false)),
        ('"', (KEY_APOSTROPHE, true)),
        ('`', (KEY_GRAVE, false)),
        ('~', (KEY_GRAVE, true)),
        (',', (KEY_COMMA, false)),
        ('<', (KEY_COMMA, true)),
        ('.', (KEY_DOT, false)),
        ('>', (KEY_DOT, true)),
        ('/', (KEY_SLASH, false)),
        ('?', (KEY_SLASH, true)),
        ('!', (KEY_1, true)),
        ('@', (KEY_2, true)),
        ('#', (KEY_3, true)),
        ('$', (KEY_4, true)),
        ('%', (KEY_5, true)),
        ('^', (KEY_6, true)),
        ('&', (KEY_7, true)),
        ('*', (KEY_8, true)),
        ('(', (KEY_9, true)),
        (')', (KEY_10, true)),
    ]
    .into_iter()
    .collect();

    for ch in command.chars() {
        if let Some(&(key, shift)) = key_map.get(&ch) {
            if shift {
                device
                    .press(KEY_LEFTSHIFT)
                    .map_err(|e| anyhow::anyhow!("{}", e))?;
            }
            device.click(key).map_err(|e| anyhow::anyhow!("{}", e))?;
            if shift {
                device
                    .release(KEY_LEFTSHIFT)
                    .map_err(|e| anyhow::anyhow!("{}", e))?;
            }
            thread::sleep(Duration::from_micros(500));
        }
    }

    Ok(())
}

#[cfg(target_os = "macos")]
fn try_enigo_type(command: &str) -> Result<()> {
    use enigo::{Enigo, Keyboard, Settings};
    use std::thread;
    use std::time::Duration;

    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| anyhow::anyhow!("{}", e))?;

    thread::sleep(Duration::from_millis(100));

    for ch in command.chars() {
        enigo
            .text(&ch.to_string())
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        thread::sleep(Duration::from_micros(500));
    }

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
        try_uinput_inject(&clean_command)
    }

    #[cfg(target_os = "macos")]
    {
        return try_enigo_type(&clean_command);
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            if clipboard.set_text(&clean_command).is_ok() {
                use enigo::{Direction, Enigo, Key, Keyboard, Settings};
                if let Ok(mut enigo) = Enigo::new(&Settings::default()) {
                    let _ = enigo.key(Key::Control, Direction::Press);
                    let _ = enigo.key(Key::Unicode('v'), Direction::Click);
                    let _ = enigo.key(Key::Control, Direction::Release);
                    return Ok(());
                }
            }
        }
        anyhow::bail!("Failed to inject via clipboard")
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
        if can_use_uinput() {
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
        return interactive_prompt(&clean_command);
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            if clipboard.set_text(&clean_command).is_ok() {
                use enigo::{Direction, Enigo, Key, Keyboard, Settings};
                if let Ok(mut enigo) = Enigo::new(&Settings::default()) {
                    let _ = enigo.key(Key::Control, Direction::Press);
                    let _ = enigo.key(Key::Unicode('v'), Direction::Click);
                    let _ = enigo.key(Key::Control, Direction::Release);
                    return Ok(None);
                }
            }
        }
        return interactive_prompt(&clean_command);
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
