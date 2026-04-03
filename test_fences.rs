pub fn strip_code_fences(text: &str) -> String {
    let trimmed = text.trim();

    if let Some(start_idx) = trimmed.find("```") {
        if let Some(newline_idx) = trimmed[start_idx..].find('\n') {
            let code_start = start_idx + newline_idx + 1;
            if let Some(end_idx) = trimmed[code_start..].find("```") {
                return trimmed[code_start..code_start + end_idx].trim().to_string();
            }
        }
    }

    let lines: Vec<&str> = trimmed.lines().collect();
    if lines.len() < 3 {
        return trimmed.to_string();
    }

    let first = lines.first().copied().unwrap_or("").trim();
    let last = lines.last().copied().unwrap_or("").trim();

    if !first.starts_with("```") || last != "```" {
        return trimmed.to_string();
    }

    lines[1..lines.len() - 1].join("\n").trim().to_string()
}

fn main() {
    println!("1: {}", strip_code_fences("Here is the command:\n```bash\nls -la\n```"));
    println!("2: {}", strip_code_fences("```\ngit status\n```"));
    println!("3: {}", strip_code_fences("ls -la"));
    println!("4: {}", strip_code_fences("```bash\nls -la"));
    println!("5: {}", strip_code_fences("Here:\n```\nls\n```\nAnd more text"));
}
