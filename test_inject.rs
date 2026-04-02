use std::process::Command;

fn main() {
    let cmd = "docker run \\\n  -it \\\n  ubuntu";
    let clean = cmd.replace('\n', " && ").replace('\r', "");
    println!("{}", clean);
}
