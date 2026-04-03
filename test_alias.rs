use std::collections::HashMap;

fn expand_aliases(args: Vec<String>, aliases: &HashMap<String, String>) -> Vec<String> {
    let mut expanded = Vec::new();
    for arg in args {
        if let Some(expansion) = aliases.get(&arg) {
            for part in expansion.split_whitespace() {
                expanded.push(part.to_string());
            }
        } else {
            expanded.push(arg);
        }
    }
    expanded
}

fn main() {
    let mut aliases = HashMap::new();
    aliases.insert("fast".to_string(), "-p fast --no-fallback".to_string());

    let args = vec!["how".to_string(), "to".to_string(), "make".to_string(), "rust".to_string(), "fast".to_string()];
    let expanded = expand_aliases(args, &aliases);
    println!("{:?}", expanded);
}
