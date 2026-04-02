use regex::Regex;

fn extract_code_block(text: &str) -> String {
    let re = Regex::new(r"(?s)```[a-zA-Z0-9]*\n(.*?)\n```").unwrap();
    if let Some(caps) = re.captures(text) {
        caps[1].trim().to_string()
    } else {
        text.trim().to_string()
    }
}

fn main() {
    println!("1: {}", extract_code_block("Here:\n```bash\nls -la\n```\nText"));
    println!("2: {}", extract_code_block("```bash\nls -la\n```"));
}
