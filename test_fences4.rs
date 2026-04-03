pub fn strip_code_fences(text: &str) -> String {
    let trimmed = text.trim();
    let lines: Vec<&str> = trimmed.lines().collect();

    // Check if the response starts and ends with a code block
    if lines.len() >= 3 {
        let first = lines.first().copied().unwrap_or("").trim();
        let last = lines.last().copied().unwrap_or("").trim();

        if first.starts_with("```") && last == "```" {
            // Ensure no other ``` in the middle (just one code block)
            let inner_fences = lines[1..lines.len() - 1]
                .iter()
                .filter(|l| l.trim().starts_with("```"))
                .count();

            if inner_fences == 0 {
                return lines[1..lines.len() - 1].join("\n").trim().to_string();
            }
        }
    }

    // Attempt to extract a single code block anywhere in the text
    if let Some(start_idx) = trimmed.find("```") {
        let after_ticks = &trimmed[start_idx + 3..];
        if let Some(newline_idx) = after_ticks.find('\n') {
            let code_start = start_idx + 3 + newline_idx + 1;
            if let Some(end_idx) = trimmed[code_start..].find("\n```") {
                // Return only if there are no more code fences in the remaining text
                let remaining = &trimmed[code_start + end_idx + 4..];
                if !remaining.contains("```") {
                    return trimmed[code_start..code_start + end_idx].trim().to_string();
                }
            }
        }
    }

    trimmed.to_string()
}
fn main() {}
