fn clean_command(command: &str) -> String {
    let mut clean_command = command.to_string();

    // First remove line continuations followed immediately by a newline
    clean_command = clean_command.replace("\\\n", "");
    // And \r\n just in case
    clean_command = clean_command.replace("\\\r\n", "");

    // Then replace any remaining newlines with &&
    clean_command = clean_command.replace('\n', " && ").replace('\r', "");

    clean_command
}

fn main() {
    let raw1 = "docker run \\\n  -it \\\n  ubuntu";
    println!("1: {}", clean_command(raw1));

    let raw2 = "cd /tmp\nls -la";
    println!("2: {}", clean_command(raw2));
}
