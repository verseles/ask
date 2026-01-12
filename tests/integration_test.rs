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
fn json_flag_is_applied() {
    let output = Command::new("cargo")
        .env("ASK_GEMINI_API_KEY", "dummy")
        .env("ASK_PROVIDER", "gemini")
        .args(["run", "--", "-v", "--json", "test query"])
        .output()
        .expect("Failed to execute command");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("json=true"));
}

#[test]
fn raw_flag_is_applied() {
    let output = Command::new("cargo")
        .env("ASK_GEMINI_API_KEY", "dummy")
        .env("ASK_PROVIDER", "gemini")
        .args(["run", "--", "-v", "--raw", "test query"])
        .output()
        .expect("Failed to execute command");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("raw=true"));
}

#[test]
fn think_flag_is_applied() {
    let output = Command::new("cargo")
        .env("ASK_GEMINI_API_KEY", "dummy")
        .env("ASK_PROVIDER", "gemini")
        .args(["run", "--", "-v", "-t", "test query"])
        .output()
        .expect("Failed to execute command");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("think=Some(true)"));
}

#[test]
fn no_think_flag_is_applied() {
    let output = Command::new("cargo")
        .env("ASK_GEMINI_API_KEY", "dummy")
        .env("ASK_PROVIDER", "gemini")
        .args(["run", "--", "-v", "--think=false", "test query"])
        .output()
        .expect("Failed to execute command");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("think=Some(false)"));
}

#[test]
fn search_flag_is_applied() {
    let output = Command::new("cargo")
        .env("ASK_GEMINI_API_KEY", "dummy")
        .env("ASK_PROVIDER", "gemini")
        .args(["run", "--", "-v", "-s", "test query"])
        .output()
        .expect("Failed to execute command");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("search=true"));
}

#[test]
fn context_flag_is_applied() {
    let output = Command::new("cargo")
        .env("ASK_GEMINI_API_KEY", "dummy")
        .env("ASK_PROVIDER", "gemini")
        .args(["run", "--", "-v", "-c60", "test query"])
        .output()
        .expect("Failed to execute command");

    let stderr = String::from_utf8_lossy(&output.stderr);
    // context is Option<u64>, so debug output is Some(60)
    assert!(stderr.contains("context=Some(60)"));
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
    assert!(stdout.contains("[profiles.main]"));
    assert!(stdout.contains("[behavior]"));
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
    // -p is for profile (changed in v0.16.0)
    let output = Command::new("cargo")
        .args(["run", "--", "-p", "test", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
}

#[test]
fn provider_flag_is_recognized() {
    // -P is for provider (changed in v0.16.0)
    let output = Command::new("cargo")
        .args(["run", "--", "-P", "gemini", "--help"])
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

#[test]
fn verbose_flag_shows_flags() {
    // We use a dummy key to ensure provider creation succeeds so we reach handle_query
    // The command will likely fail due to invalid key, but we only care about stderr output
    // We force Gemini to avoid relying on user config
    let output = Command::new("cargo")
        .env("ASK_GEMINI_API_KEY", "dummy")
        .env("ASK_PROVIDER", "gemini")
        .args(["run", "--", "-v", "test query"])
        .output()
        .expect("Failed to execute command");

    // We don't assert success because it might fail on API call
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("[verbose] flags:"));
    assert!(stderr.contains("context="));
    assert!(stderr.contains("command_mode="));
    assert!(stderr.contains("json="));
}

#[test]
fn combined_flags_vt0_applies_both() {
    let output = Command::new("cargo")
        .env("ASK_GEMINI_API_KEY", "dummy")
        .env("ASK_PROVIDER", "gemini")
        .args(["run", "--", "-vt0", "test query"])
        .output()
        .expect("Failed to execute command");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("think=Some(false)"),
        "Expected think=Some(false), got: {}",
        stderr
    );
}

#[test]
fn combined_flags_t0v_posix_ignores_trailing() {
    // POSIX: -t0 consumes "0v", so v is part of the value (ignored)
    let output = Command::new("cargo")
        .env("ASK_GEMINI_API_KEY", "dummy")
        .env("ASK_PROVIDER", "gemini")
        .args(["run", "--", "-t0v", "-v", "test query"])
        .output()
        .expect("Failed to execute command");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("think=Some(false)"),
        "Expected think=Some(false), got: {}",
        stderr
    );
}

#[test]
fn combined_flags_xy_applies_both() {
    let output = Command::new("cargo")
        .env("ASK_GEMINI_API_KEY", "dummy")
        .env("ASK_PROVIDER", "gemini")
        .args(["run", "--", "-xy", "-v", "test query"])
        .output()
        .expect("Failed to execute command");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("command_mode=true"),
        "Expected command_mode=true"
    );
    assert!(stderr.contains("yes=true"), "Expected yes=true");
}

#[test]
fn context_with_value_c60() {
    let output = Command::new("cargo")
        .env("ASK_GEMINI_API_KEY", "dummy")
        .env("ASK_PROVIDER", "gemini")
        .args(["run", "--", "-c60", "-v", "test query"])
        .output()
        .expect("Failed to execute command");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("context=Some(60)"),
        "Expected context=Some(60), got: {}",
        stderr
    );
}
