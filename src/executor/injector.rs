use anyhow::Result;
use colored::Colorize;

#[cfg(target_os = "linux")]
fn try_uinput_inject(command: &str) -> Result<()> {
    use mouse_keyboard_input::key_codes::*;
    use mouse_keyboard_input::VirtualDevice;
    use std::collections::HashMap;

    let mut device = VirtualDevice::default().map_err(|e| anyhow::anyhow!("{}", e))?;

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
        }
    }

    Ok(())
}

fn interactive_prompt(command: &str) -> Result<bool> {
    use std::io::{self, Write};

    println!("{} {}", "Command:".green(), command.bright_white().bold());
    print!(
        "{}",
        "Press Enter to execute, Ctrl+C to cancel: ".bright_black()
    );
    io::stdout().flush()?;

    let mut input = String::new();
    match io::stdin().read_line(&mut input) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

pub fn inject_command(command: &str) -> Result<Option<bool>> {
    let clean_command = command.replace('\n', " && ").replace('\r', "");

    #[cfg(target_os = "linux")]
    {
        if try_uinput_inject(&clean_command).is_ok() {
            return Ok(None);
        }
    }

    #[cfg(any(target_os = "windows", target_os = "macos"))]
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
    }

    interactive_prompt(&clean_command).map(Some)
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
