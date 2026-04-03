pub fn strip_code_fences(text: &str) -> String {
    let trimmed = text.trim();

    // Check if the entire response is a single code block
    let lines: Vec<&str> = trimmed.lines().collect();
    if lines.len() >= 3 {
        let first = lines.first().copied().unwrap_or("").trim();
        let last = lines.last().copied().unwrap_or("").trim();

        if first.starts_with("```") && last == "```" {
            // Check if there are any other code fences inside
            let inner_fences = lines[1..lines.len() - 1].iter().filter(|l| l.trim().starts_with("```")).count();
            if inner_fences == 0 {
                return lines[1..lines.len() - 1].join("\n").trim().to_string();
            }
        }
    }

    // Look for a code block anywhere in the text
    if let Some(start_idx) = trimmed.find("```") {
        let after_ticks = &trimmed[start_idx + 3..];
        if let Some(newline_idx) = after_ticks.find('\n') {
            let code_start = start_idx + 3 + newline_idx + 1;
            if let Some(end_idx) = trimmed[code_start..].find("\n```") {
                // Return only if there is exactly one block (we don't want to parse complex md)
                let remaining = &trimmed[code_start + end_idx + 4..];
                if !remaining.contains("```") {
                    return trimmed[code_start..code_start + end_idx].trim().to_string();
                }
            }
        }
    }

    trimmed.to_string()
}

fn main() {
    println!("1: {}", strip_code_fences("Here is the command:\n```bash\nls -la\n```"));
    println!("2: {}", strip_code_fences("```\ngit status\n```"));
    println!("3: {}", strip_code_fences("ls -la"));
    println!("4: {}", strip_code_fences("```bash\nls -la"));
    println!("5: {}", strip_code_fences("Here:\n```\nls\n```\nAnd more text"));
    println!("6: {}", strip_code_fences("```\nls\n```\nAnd more text"));
}
