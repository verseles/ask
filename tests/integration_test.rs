//! Integration tests for the ask CLI

use std::process::Command;

#[test]
fn help_flag_shows_usage() {
    let output = Command::new("cargo")
        .args(["run", "--", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("ask") || stdout.contains("Ask"));
}

#[test]
fn version_flag_shows_version() {
    let output = Command::new("cargo")
        .args(["run", "--", "--version"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("0.1.0") || stdout.contains("ask"));
}

#[test]
fn no_arguments_does_not_panic() {
    let output = Command::new("cargo")
        .args(["run", "--"])
        .output()
        .expect("Failed to execute command");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(!stderr.contains("panic"));
    assert!(!stdout.contains("panic"));
}

#[test]
fn json_flag_is_recognized() {
    let output = Command::new("cargo")
        .args(["run", "--", "--json", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
}

#[test]
fn raw_flag_is_recognized() {
    let output = Command::new("cargo")
        .args(["run", "--", "--raw", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
}
