use serde_json::json;
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

#[test]
#[allow(clippy::too_many_lines)]
fn runs_the_local_workflow_without_losing_existing_instructions() {
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
        root.join("AGENTS.md"),
        "# Existing instructions\n\nKeep this.\n",
    )
    .expect("existing instructions");
    assert!(
        Command::new("git")
            .args(["add", "AGENTS.md"])
            .current_dir(root)
            .status()
            .expect("git add")
            .success()
    );

    let sessions = root.join("sessions");
    fs::create_dir(&sessions).expect("sessions");
    write_session(&sessions.join("one.jsonl"), root, "one");
    write_session(&sessions.join("two.jsonl"), root, "two");
    let database = root.join("state/state.db");

    let blocked = run(
        root,
        &[
            "analyze",
            "--claude-root",
            path(&sessions),
            "--database",
            path(&database),
            "--provider",
            "codex-cli",
        ],
    );
    assert!(!blocked.status.success());
    assert!(String::from_utf8_lossy(&blocked.stderr).contains("allow_remote_inference = true"));

    let analysis = run(
        root,
        &[
            "analyze",
            "--claude-root",
            path(&sessions),
            "--database",
            path(&database),
        ],
    );
    assert!(
        analysis.status.success(),
        "{}",
        String::from_utf8_lossy(&analysis.stderr)
    );
    assert!(String::from_utf8_lossy(&analysis.stdout).contains("Processed sessions: 2"));

    let listing = run(root, &["review", "--database", path(&database)]);
    let listing = String::from_utf8(listing.stdout).expect("review output");
    let id = listing.split_whitespace().next().expect("candidate id");
    assert!(id.starts_with("candidate_"));

    let decision = run(
        root,
        &["review", "--database", path(&database), "--accept", id],
    );
    assert!(
        decision.status.success(),
        "{}",
        String::from_utf8_lossy(&decision.stderr)
    );
    let explanation = run(root, &["explain", id, "--database", path(&database)]);
    let explanation = String::from_utf8_lossy(&explanation.stdout);
    assert!(explanation.contains("Evidence:"));
    assert!(explanation.contains("session one"));
    let preview = run(root, &["diff", "--database", path(&database)]);
    assert!(String::from_utf8_lossy(&preview.stdout).contains("Always use pnpm."));

    let applied = run(root, &["apply", "--database", path(&database), "--yes"]);
    assert!(
        applied.status.success(),
        "{}",
        String::from_utf8_lossy(&applied.stderr)
    );
    let agents = fs::read_to_string(root.join("AGENTS.md")).expect("AGENTS output");
    assert!(agents.contains("Keep this."));
    assert!(agents.contains("Always use pnpm."));
    assert!(root.join("CLAUDE.md").exists());

    let health = run(root, &["doctor", "--database", path(&database)]);
    assert!(String::from_utf8_lossy(&health.stdout).contains("Context health: 100/100"));

    let incremental = run(
        root,
        &[
            "analyze",
            "--claude-root",
            path(&sessions),
            "--database",
            path(&database),
        ],
    );
    assert!(String::from_utf8_lossy(&incremental.stdout).contains("Unchanged sessions: 2"));

    let global = run(
        root,
        &[
            "analyze",
            "--global",
            "--claude-root",
            path(&sessions),
            "--database",
            path(&database),
        ],
    );
    assert!(String::from_utf8_lossy(&global.stdout).contains("Processed sessions: 2"));
}

fn write_session(path: &Path, project: &Path, id: &str) {
    let lines = [
        json!({"type":"assistant","uuid":format!("{id}-a"),"cwd":project,"message":{"role":"assistant","content":"I will use npm."}}),
        json!({"type":"user","uuid":format!("{id}-u"),"cwd":project,"message":{"role":"user","content":"Always use pnpm."}}),
    ];
    let content = lines
        .into_iter()
        .map(|line| line.to_string())
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    fs::write(path, content).expect("session fixture");
}

fn run(root: &Path, arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_agentctx"))
        .args(arguments)
        .current_dir(root)
        .env("HOME", root.join("empty-home"))
        .output()
        .expect("run agentctx")
}

fn path(value: &Path) -> &str {
    value.to_str().expect("UTF-8 test path")
}
