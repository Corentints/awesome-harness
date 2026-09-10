#![cfg(unix)]

use serde_json::json;
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::Path,
    process::{Command, Output},
};

#[test]
fn infers_an_ordinary_request_once_and_persists_its_evidence() {
    let directory = tempfile::tempdir().expect("temp directory");
    let root = directory.path();
    assert!(
        Command::new("git")
            .args(["init", "--quiet"])
            .current_dir(root)
            .status()
            .expect("git init")
            .success()
    );
    fs::write(
        root.join(".agentctx.toml"),
        "[llm]\nprovider = \"codex-cli\"\n\n[privacy]\nallow_remote_inference = true\n",
    )
    .expect("configuration");
    let sessions = root.join("sessions");
    fs::create_dir(&sessions).expect("sessions");
    let session = json!({
        "type": "user",
        "uuid": "ordinary-request",
        "cwd": root,
        "message": {
            "role": "user",
            "content": "Could you keep functions small?"
        }
    });
    fs::write(sessions.join("one.jsonl"), format!("{session}\n")).expect("session");

    let fake_bin = root.join("bin");
    fs::create_dir(&fake_bin).expect("fake bin");
    let fake_codex = fake_bin.join("codex");
    fs::write(
        &fake_codex,
        r#"#!/bin/sh
output=""
while [ "$#" -gt 0 ]; do
  if [ "$1" = "--output-last-message" ]; then
    shift
    output="$1"
  fi
  shift
done
printf '%s' '{"rules":[{"text":"Keep functions small.","kind":"coding_convention","scope":"project","confidence":0.8,"evidence_message_ids":["ordinary-request"]}]}' > "$output"
"#,
    )
    .expect("fake codex");
    let mut permissions = fs::metadata(&fake_codex).expect("metadata").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&fake_codex, permissions).expect("executable fake");
    let database = root.join("state.db");

    let first = run(root, &sessions, &database, &fake_bin);
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    let first_stdout = String::from_utf8_lossy(&first.stdout);
    assert!(first_stdout.contains("1 user inputs"));
    assert!(first_stdout.contains("Semantic suggestions: 1"));

    let review = Command::new(env!("CARGO_BIN_EXE_agentctx"))
        .args(["review", "--database", path(&database)])
        .current_dir(root)
        .env("HOME", root.join("empty-home"))
        .output()
        .expect("review");
    assert!(String::from_utf8_lossy(&review.stdout).contains("Keep functions small."));

    let second = run(root, &sessions, &database, &fake_bin);
    assert!(second.status.success());
    let second_stdout = String::from_utf8_lossy(&second.stdout);
    assert!(second_stdout.contains("Unchanged sessions: 1"));
    assert!(second_stdout.contains("0 user inputs"));
    assert!(second_stdout.contains("Semantic suggestions: 0"));
}

fn run(root: &Path, sessions: &Path, database: &Path, fake_bin: &Path) -> Output {
    let inherited_path = std::env::var_os("PATH").unwrap_or_default();
    let path_value = std::env::join_paths(
        std::iter::once(fake_bin.to_path_buf()).chain(std::env::split_paths(&inherited_path)),
    )
    .expect("PATH");
    Command::new(env!("CARGO_BIN_EXE_agentctx"))
        .args([
            "analyze",
            "--claude-root",
            path(sessions),
            "--database",
            path(database),
        ])
        .current_dir(root)
        .env("HOME", root.join("empty-home"))
        .env("PATH", path_value)
        .output()
        .expect("analyze")
}

fn path(value: &Path) -> &str {
    value.to_str().expect("UTF-8 test path")
}
