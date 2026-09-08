use agentctx::{
    analysis::{CorrectionCandidate, CorrectionEvidence},
    domain::{MessageId, RuleScope, SessionId, Visibility},
    storage::{Database, DecisionStatus, ReviewDecision, SourceFingerprint},
};

fn candidate() -> CorrectionCandidate {
    CorrectionCandidate {
        canonical_text: "Always use pnpm.".to_owned(),
        occurrences: 2,
        evidence: vec![CorrectionEvidence {
            source_path: Some("/sessions/session.jsonl".into()),
            project_path: Some("/projects/example".into()),
            session_id: SessionId::new("session-1"),
            message_id: Some(MessageId::new("message-2")),
            user_text: "No, always use pnpm.".to_owned(),
            preceding_agent_text: Some("I will use npm.".to_owned()),
        }],
    }
}

#[test]
fn tracks_source_content_and_parser_versions() {
    let directory = tempfile::tempdir().expect("temp directory");
    let source_path = directory.path().join("session.jsonl");
    std::fs::write(&source_path, "{}\n").expect("source");
    let database = Database::in_memory().expect("database");
    let first = SourceFingerprint::from_path(&source_path, 1).expect("fingerprint");

    assert!(!database.is_source_current(&first).expect("query"));
    database.record_source(&first).expect("record");
    assert!(database.is_source_current(&first).expect("query"));

    let newer_parser = SourceFingerprint::from_path(&source_path, 2).expect("fingerprint");
    assert!(!database.is_source_current(&newer_parser).expect("query"));
    std::fs::write(&source_path, "{\"changed\":true}\n").expect("source");
    let changed = SourceFingerprint::from_path(&source_path, 1).expect("fingerprint");
    assert!(!database.is_source_current(&changed).expect("query"));
}

#[test]
fn preserves_review_decisions_when_candidates_are_reanalyzed() {
    let mut database = Database::in_memory().expect("database");
    let candidate = candidate();
    database
        .upsert_candidates(std::slice::from_ref(&candidate))
        .expect("upsert");
    let decision = ReviewDecision {
        status: DecisionStatus::Accepted,
        edited_text: Some("Use pnpm.".to_owned()),
        scope: RuleScope::Project("/projects/example".into()),
        visibility: Visibility::Shared,
        last_confirmed_at: Some(
            "2026-01-01T00:00:00Z"
                .parse()
                .expect("valid confirmation date"),
        ),
        last_used_at: None,
        valid_until: None,
    };
    database
        .record_decision(&candidate.canonical_text, &decision)
        .expect("decision");

    database
        .upsert_candidates(std::slice::from_ref(&candidate))
        .expect("reanalyze");

    assert_eq!(database.load_candidates().expect("load"), [candidate]);
    assert_eq!(
        database
            .decision("Always use pnpm.")
            .expect("load decision"),
        Some(decision)
    );
}

#[test]
fn replaces_only_the_evidence_owned_by_a_changed_source() {
    let directory = tempfile::tempdir().expect("temp directory");
    let source_path = directory.path().join("session.jsonl");
    std::fs::write(&source_path, "{}\n").expect("source");
    let fingerprint = SourceFingerprint::from_path(&source_path, 1).expect("fingerprint");
    let mut database = Database::in_memory().expect("database");
    let mut candidate = candidate();
    candidate.evidence[0].source_path = Some(source_path.clone());

    database
        .replace_source_candidates(&fingerprint, std::slice::from_ref(&candidate))
        .expect("first analysis");
    assert_eq!(database.load_candidates().expect("load")[0].occurrences, 1);

    database
        .replace_source_candidates(&fingerprint, &[])
        .expect("changed analysis");
    assert!(database.load_candidates().expect("load").is_empty());
}
