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
    assert!(stdout.contains("ask"));
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

#[test]
fn think_flag_is_recognized() {
    let output = Command::new("cargo")
        .args(["run", "--", "-t", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
}

#[test]
fn no_think_flag_is_recognized() {
    let output = Command::new("cargo")
        .args(["run", "--", "--no-think", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
}

#[test]
fn verbose_flag_is_recognized() {
    let output = Command::new("cargo")
        .args(["run", "--", "-v", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
}

#[test]
fn verbose_long_flag_is_recognized() {
    let output = Command::new("cargo")
        .args(["run", "--", "--verbose", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
}

#[test]
fn profiles_subcommand_works() {
    let output = Command::new("cargo")
        .args(["run", "--", "profiles"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Profiles") || stdout.contains("profile"));
}

#[test]
fn make_config_flag_outputs_template() {
    let output = Command::new("cargo")
        .args(["run", "--", "--make-config"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("[default]"));
    assert!(stdout.contains("[providers"));
    assert!(stdout.contains("[behavior]"));
}

#[test]
fn non_interactive_flag_is_recognized() {
    let output = Command::new("cargo")
        .args(["run", "--", "-n", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
}

#[test]
fn api_key_flag_is_recognized() {
    let output = Command::new("cargo")
        .args(["run", "--", "-k", "test-key", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
}

#[test]
fn profile_flag_is_recognized() {
    let output = Command::new("cargo")
        .args(["run", "--", "-P", "test", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
}

#[test]
fn search_flag_is_recognized() {
    let output = Command::new("cargo")
        .args(["run", "--", "-s", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
}

#[test]
fn make_config_includes_aggressive_option() {
    let output = Command::new("cargo")
        .args(["run", "--", "--make-config"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("[update]"));
    assert!(stdout.contains("aggressive"));
}

#[test]
fn help_env_includes_update_aggressive() {
    let output = Command::new("cargo")
        .args(["run", "--", "--help-env"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("ASK_UPDATE_AGGRESSIVE") || stdout.contains("ASK_UPDATE"));
}

#[test]
fn update_aggressive_env_is_recognized() {
    let output = Command::new("cargo")
        .env("ASK_UPDATE_AGGRESSIVE", "false")
        .args(["run", "--", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
}
