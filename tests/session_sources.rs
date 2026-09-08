use agentctx::{
    domain::{AgentSource, Role},
    ingest::{ClaudeSessionSource, CodexSessionSource, SessionSource},
};
use std::path::PathBuf;

fn fixture(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(path)
}

#[test]
fn parses_claude_messages_and_tolerates_unknown_events() {
    let source = ClaudeSessionSource::new(fixture("claude"));
    let references = source.discover().expect("fixture discovery should work");
    let session = source.parse(&references[0]).expect("fixture should parse");

    assert_eq!(session.source, AgentSource::Claude);
    assert_eq!(session.project, Some(PathBuf::from("/projects/example")));
    assert_eq!(session.messages.len(), 3);
    assert_eq!(session.messages[2].role, Role::User);
    assert!(session.messages[2].content.contains("pnpm"));
    assert_eq!(session.events.len(), 2);
}

#[test]
fn parses_codex_rollout_messages_and_metadata() {
    let source = CodexSessionSource::new(fixture("codex"));
    let references = source.discover().expect("fixture discovery should work");
    let session = source.parse(&references[0]).expect("fixture should parse");

    assert_eq!(session.id.as_str(), "codex-session-1");
    assert_eq!(session.source, AgentSource::Codex);
    assert_eq!(session.project, Some(PathBuf::from("/projects/example")));
    assert_eq!(session.messages.len(), 3);
    assert!(session.messages[2].content.contains("src/generated"));
    assert_eq!(session.events.len(), 2);
}

#[test]
fn reports_the_line_of_invalid_json() {
    let directory = tempfile::tempdir().expect("temp directory");
    let path = directory.path().join("broken.jsonl");
    std::fs::write(&path, "{}\nnot-json\n").expect("write fixture");
    let source = ClaudeSessionSource::new(directory.path());
    let reference = source.discover().expect("discover").remove(0);

    let error = source
        .parse(&reference)
        .expect_err("invalid line must fail");
    assert!(error.to_string().contains(":2:"));
    assert!(!error.to_string().contains("not-json"));
}
