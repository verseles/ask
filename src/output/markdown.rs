#![allow(dead_code)]

use termimad::MadSkin;

pub fn render_markdown(text: &str) -> String {
    let skin = MadSkin::default();
    skin.term_text(text).to_string()
}

/// Print markdown directly to terminal
pub fn print_markdown(text: &str) {
    let skin = MadSkin::default();
    skin.print_text(text);
}
