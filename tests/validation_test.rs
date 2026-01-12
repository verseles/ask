use std::process::Command;

#[test]
fn test_cli_profile_provider_mutex() {
    let output = Command::new("cargo")
        .args(["run", "--", "-p", "main", "-P", "gemini", "hello"])
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Cannot use --profile (-p) and --provider (-P) together"));
}

#[test]
fn test_env_profile_provider_mutex() {
    let output = Command::new("cargo")
        .env("ASK_PROFILE", "main")
        .env("ASK_PROVIDER", "gemini")
        .args(["run", "--", "hello"])
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Cannot use ASK_PROFILE and ASK_PROVIDER together"));
}

#[test]
fn test_adhoc_requires_api_key() {
    // Unset any potential API keys from env to ensure test isolation
    let output = Command::new("cargo")
        .env_remove("ASK_GEMINI_API_KEY")
        .env_remove("GEMINI_API_KEY")
        .args(["run", "--", "-P", "gemini", "hello"])
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Ad-hoc mode requires an API key"));
}

#[test]
fn test_adhoc_with_api_key_works_validation() {
    // This test passes validation but might fail on actual network request
    // We just want to ensure it doesn't fail with the validation error
    let output = Command::new("cargo")
        .args(["run", "--", "-P", "gemini", "-k", "dummy_key", "hello"])
        .output()
        .expect("Failed to execute command");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("Ad-hoc mode requires an API key"));
    assert!(!stderr.contains("Cannot use --profile"));
}
