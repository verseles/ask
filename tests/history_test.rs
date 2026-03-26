use chrono::{Duration, Utc};
use serde_json::json;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

fn ask_bin() -> &'static str {
    env!("CARGO_BIN_EXE_ask")
}

fn seed_context(
    storage_dir: &Path,
    id: &str,
    pwd: &Path,
    messages: &[(&str, &str)],
    last_used: chrono::DateTime<Utc>,
) {
    let stored_messages: Vec<_> = messages
        .iter()
        .map(|(role, content)| {
            json!({
                "role": role,
                "content": content,
                "timestamp": last_used,
            })
        })
        .collect();

    let entry = json!({
        "id": id,
        "pwd": pwd.to_string_lossy(),
        "messages": stored_messages,
        "created_at": last_used,
        "last_used": last_used,
    });

    fs::write(
        storage_dir.join(format!("{}.json", id)),
        serde_json::to_string_pretty(&entry).unwrap(),
    )
    .unwrap();
}

#[test]
fn history_show_specific_target_works() {
    let temp = tempdir().unwrap();
    let storage_dir = temp.path().join("contexts");
    let project_dir = temp.path().join("project-alpha");

    fs::create_dir_all(&storage_dir).unwrap();
    fs::create_dir_all(&project_dir).unwrap();

    seed_context(
        &storage_dir,
        "aaaa1111bbbb2222",
        &project_dir,
        &[
            ("user", "How do I run the tests?"),
            ("assistant", "Use cargo test"),
        ],
        Utc::now(),
    );

    let output = Command::new(ask_bin())
        .current_dir(temp.path())
        .env("ASK_CONTEXT_PATH", &storage_dir)
        .args(["history", "aaaa1111"])
        .output()
        .unwrap();

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Context for:"));
    assert!(stdout.contains(&*project_dir.to_string_lossy()));
    assert!(stdout.contains("Use cargo test"));
}

#[test]
fn history_search_matches_path_and_content() {
    let temp = tempdir().unwrap();
    let storage_dir = temp.path().join("contexts");
    let alpha_dir = temp.path().join("project-alpha");
    let docker_dir = temp.path().join("docker-app");
    let gamma_dir = temp.path().join("project-gamma");

    fs::create_dir_all(&storage_dir).unwrap();
    fs::create_dir_all(&alpha_dir).unwrap();
    fs::create_dir_all(&docker_dir).unwrap();
    fs::create_dir_all(&gamma_dir).unwrap();

    seed_context(
        &storage_dir,
        "11111111aaaaaaaa",
        &alpha_dir,
        &[
            ("user", "How do I build images?"),
            ("assistant", "Use docker compose up --build"),
        ],
        Utc::now(),
    );
    seed_context(
        &storage_dir,
        "22222222bbbbbbbb",
        &docker_dir,
        &[
            ("user", "What is the current status?"),
            ("assistant", "Everything looks good"),
        ],
        Utc::now() - Duration::minutes(1),
    );
    seed_context(
        &storage_dir,
        "33333333cccccccc",
        &gamma_dir,
        &[
            ("user", "How do I run tests?"),
            ("assistant", "Use cargo test"),
        ],
        Utc::now() - Duration::minutes(2),
    );

    let output = Command::new(ask_bin())
        .current_dir(temp.path())
        .env("ASK_CONTEXT_PATH", &storage_dir)
        .args(["history", "search", "docker"])
        .output()
        .unwrap();

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("History Search (2)"), "stdout: {stdout}");
    assert!(
        stdout.contains(&*alpha_dir.to_string_lossy()),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains(&*docker_dir.to_string_lossy()),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("docker compose up --build"),
        "stdout: {stdout}"
    );
    assert!(
        !stdout.contains(&*gamma_dir.to_string_lossy()),
        "stdout: {stdout}"
    );
}

#[test]
fn history_prune_removes_deleted_directories() {
    let temp = tempdir().unwrap();
    let storage_dir = temp.path().join("contexts");
    let live_dir = temp.path().join("live-project");
    let orphan_dir = temp.path().join("old-project");

    fs::create_dir_all(&storage_dir).unwrap();
    fs::create_dir_all(&live_dir).unwrap();
    fs::create_dir_all(&orphan_dir).unwrap();

    seed_context(
        &storage_dir,
        "44444444dddddddd",
        &live_dir,
        &[("user", "Keep me"), ("assistant", "I still exist")],
        Utc::now(),
    );
    seed_context(
        &storage_dir,
        "55555555eeeeeeee",
        &orphan_dir,
        &[("user", "Delete me"), ("assistant", "This project is gone")],
        Utc::now() - Duration::minutes(1),
    );

    fs::remove_dir_all(&orphan_dir).unwrap();

    let prune_output = Command::new(ask_bin())
        .current_dir(&live_dir)
        .env("ASK_CONTEXT_PATH", &storage_dir)
        .args(["-y", "history", "prune"])
        .output()
        .unwrap();

    assert!(prune_output.status.success());

    let prune_stdout = String::from_utf8_lossy(&prune_output.stdout);
    assert!(prune_stdout.contains("Pruned orphaned contexts: 1"));
    assert!(prune_stdout.contains(&*orphan_dir.to_string_lossy()));

    let list_output = Command::new(ask_bin())
        .current_dir(&live_dir)
        .env("ASK_CONTEXT_PATH", &storage_dir)
        .arg("history")
        .output()
        .unwrap();

    assert!(list_output.status.success());

    let list_stdout = String::from_utf8_lossy(&list_output.stdout);
    assert!(list_stdout.contains(&*live_dir.to_string_lossy()));
    assert!(!list_stdout.contains(&*orphan_dir.to_string_lossy()));
}
