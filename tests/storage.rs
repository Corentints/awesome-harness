use agentctx::{
    analysis::{
        CorrectionCandidate, CorrectionEvidence, InputPriority, PrioritizedInput, PriorityReason,
    },
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

fn input(text: &str) -> PrioritizedInput {
    PrioritizedInput {
        session_id: SessionId::new("session-1"),
        project_path: Some("/projects/example".into()),
        message_index: 1,
        message_id: Some(MessageId::new("message-2")),
        text: text.to_owned(),
        priority: InputPriority::High,
        reason: PriorityReason::ExplicitCorrection,
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

#[test]
fn preserves_analysis_state_for_unchanged_inputs_and_requeues_changed_text() {
    let directory = tempfile::tempdir().expect("temp directory");
    let source_path = directory.path().join("session.jsonl");
    std::fs::write(&source_path, "{}\n").expect("source");
    let fingerprint = SourceFingerprint::from_path(&source_path, 1).expect("fingerprint");
    let mut database = Database::in_memory().expect("database");
    let first = input("Always use pnpm.");

    database
        .replace_source_analysis(&fingerprint, &[], std::slice::from_ref(&first), 1)
        .expect("index input");
    let pending = database.pending_analysis_inputs(1).expect("pending");
    assert_eq!(pending.len(), 1);
    database
        .mark_analysis_inputs_analyzed(&[pending[0].id.clone()])
        .expect("complete input");

    database
        .replace_source_analysis(&fingerprint, &[], std::slice::from_ref(&first), 1)
        .expect("reindex unchanged input");
    assert!(
        database
            .pending_analysis_inputs(1)
            .expect("pending")
            .is_empty()
    );

    let changed = input("Always use pnpm and Corepack.");
    database
        .replace_source_analysis(&fingerprint, &[], &[changed], 1)
        .expect("reindex changed input");
    assert_eq!(
        database.pending_analysis_inputs(1).expect("pending").len(),
        1
    );
}

#[test]
fn a_new_analysis_version_requeues_unchanged_inputs() {
    let directory = tempfile::tempdir().expect("temp directory");
    let source_path = directory.path().join("session.jsonl");
    std::fs::write(&source_path, "{}\n").expect("source");
    let fingerprint = SourceFingerprint::from_path(&source_path, 1).expect("fingerprint");
    let mut database = Database::in_memory().expect("database");
    let input = input("Always use pnpm.");

    database
        .replace_source_analysis(&fingerprint, &[], std::slice::from_ref(&input), 1)
        .expect("index input");
    let pending = database.pending_analysis_inputs(1).expect("pending");
    database
        .mark_analysis_inputs_analyzed(&[pending[0].id.clone()])
        .expect("complete input");
    database
        .replace_source_analysis(&fingerprint, &[], &[input], 2)
        .expect("upgrade analysis");

    assert_eq!(
        database.pending_analysis_inputs(2).expect("pending").len(),
        1
    );
}

#[test]
fn merges_inferred_evidence_without_losing_existing_evidence() {
    let mut database = Database::in_memory().expect("database");
    let first = candidate();
    database
        .upsert_candidates(std::slice::from_ref(&first))
        .expect("deterministic candidate");
    let mut inferred = candidate();
    inferred.evidence[0].session_id = SessionId::new("session-2");
    inferred.evidence[0].message_id = Some(MessageId::new("message-3"));
    inferred.evidence[0].user_text = "Please remember to use pnpm.".to_owned();

    database
        .merge_candidates(&[inferred])
        .expect("merge inferred candidate");

    let stored = database.load_candidates().expect("load");
    assert_eq!(stored[0].occurrences, 2);
    assert_eq!(stored[0].evidence.len(), 2);
}

#[test]
fn recovers_when_a_column_was_added_before_its_migration_was_recorded() {
    let directory = tempfile::tempdir().expect("temp directory");
    let path = directory.path().join("state.db");
    drop(Database::open(&path).expect("initial database"));
    let connection = rusqlite::Connection::open(&path).expect("raw database");
    connection
        .execute("DELETE FROM schema_migrations WHERE version = 6", [])
        .expect("simulate interrupted migration");
    drop(connection);

    let database = Database::open(&path).expect("recover migration");

    assert!(
        database
            .pending_analysis_inputs(1)
            .expect("query migrated table")
            .is_empty()
    );
}
