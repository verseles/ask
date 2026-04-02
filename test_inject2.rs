fn inject_command(command: &str) -> String {
    let clean_command = command.replace('\n', " && ").replace('\r', "");
    clean_command
}

fn main() {
    let raw = "docker run \\\n  -it \\\n  ubuntu";
    println!("{}", inject_command(raw));
}
