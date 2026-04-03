fn strip_code_fences(text: &str) -> String {
    let trimmed = text.trim();
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
    println!("{}", strip_code_fences("Here is the command:\n```bash\nls -la\n```"));
}
