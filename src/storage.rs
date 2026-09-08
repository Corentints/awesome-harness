use crate::{
    analysis::{CorrectionCandidate, CorrectionEvidence},
    domain::{MessageId, RuleScope, SessionId, Visibility},
};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{self, Read},
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};
use thiserror::Error;

const SCHEMA_VERSION: u32 = 4;

pub struct Database {
    connection: Connection,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceFingerprint {
    pub path: PathBuf,
    pub size: u64,
    pub modified_ns: u128,
    pub content_hash: String,
    pub parser_version: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionStatus {
    Accepted,
    Rejected,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReviewDecision {
    pub status: DecisionStatus,
    pub edited_text: Option<String>,
    pub scope: RuleScope,
    pub visibility: Visibility,
    pub last_confirmed_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub valid_until: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReviewedCandidate {
    pub id: String,
    pub canonical_text: String,
    pub occurrences: usize,
    pub decision: ReviewDecision,
}

impl SourceFingerprint {
    /// Hashes a source and records metadata used by incremental indexing.
    ///
    /// # Errors
    ///
    /// Returns an error when file metadata or contents cannot be read.
    pub fn from_path(path: &Path, parser_version: u32) -> Result<Self, StorageError> {
        let metadata = path.metadata().map_err(|source| StorageError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        let modified_ns = metadata
            .modified()
            .ok()
            .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
            .map_or(0, |duration| duration.as_nanos());
        let mut file = File::open(path).map_err(|source| StorageError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        let mut hasher = blake3::Hasher::new();
        let mut buffer = [0_u8; 16 * 1024];
        loop {
            let read = file.read(&mut buffer).map_err(|source| StorageError::Io {
                path: path.to_path_buf(),
                source,
            })?;
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
        }
        Ok(Self {
            path: path.to_path_buf(),
            size: metadata.len(),
            modified_ns,
            content_hash: hasher.finalize().to_hex().to_string(),
            parser_version,
        })
    }
}

impl Database {
    /// Opens or creates the local database and applies all schema migrations.
    ///
    /// # Errors
    ///
    /// Returns an error when `SQLite` cannot open or migrate the database.
    pub fn open(path: &Path) -> Result<Self, StorageError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|source| StorageError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        let connection = Connection::open(path)?;
        let database = Self { connection };
        database.migrate()?;
        Ok(database)
    }

    /// Opens a temporary in-memory database, primarily for isolated workflows.
    ///
    /// # Errors
    ///
    /// Returns an error when `SQLite` cannot initialize the schema.
    pub fn in_memory() -> Result<Self, StorageError> {
        let connection = Connection::open_in_memory()?;
        let database = Self { connection };
        database.migrate()?;
        Ok(database)
    }

    fn migrate(&self) -> Result<(), StorageError> {
        self.connection.execute_batch(
            "PRAGMA foreign_keys = ON;
             PRAGMA journal_mode = WAL;
             CREATE TABLE IF NOT EXISTS schema_migrations (
                 version INTEGER PRIMARY KEY,
                 applied_at TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS sources (
                 path TEXT PRIMARY KEY,
                 size INTEGER NOT NULL,
                 modified_ns TEXT NOT NULL,
                 content_hash TEXT NOT NULL,
                 parser_version INTEGER NOT NULL,
                 processed_at TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS candidates (
                 id TEXT PRIMARY KEY,
                 canonical_text TEXT NOT NULL,
                 occurrences INTEGER NOT NULL,
                 updated_at TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS evidence (
                 candidate_id TEXT NOT NULL REFERENCES candidates(id) ON DELETE CASCADE,
                 session_id TEXT NOT NULL,
                 message_id TEXT,
                 user_text TEXT NOT NULL,
                 preceding_agent_text TEXT,
                 UNIQUE(candidate_id, session_id, message_id, user_text)
             );
             CREATE TABLE IF NOT EXISTS review_decisions (
                 candidate_id TEXT PRIMARY KEY REFERENCES candidates(id) ON DELETE CASCADE,
                 status TEXT NOT NULL,
                 edited_text TEXT,
                 scope_json TEXT NOT NULL,
                 visibility TEXT NOT NULL,
                 decided_at TEXT NOT NULL
             );",
        )?;
        let version = self.connection.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get::<_, u32>(0),
        )?;
        if version < 1 {
            self.connection.execute(
                "INSERT INTO schema_migrations(version, applied_at) VALUES (1, ?1)",
                [Utc::now().to_rfc3339()],
            )?;
        }
        if version < 2 {
            self.connection.execute(
                "ALTER TABLE evidence ADD COLUMN source_path TEXT NOT NULL DEFAULT ''",
                [],
            )?;
            self.connection.execute(
                "INSERT INTO schema_migrations(version, applied_at) VALUES (?1, ?2)",
                params![2, Utc::now().to_rfc3339()],
            )?;
        }
        if version < 3 {
            self.connection.execute(
                "ALTER TABLE evidence ADD COLUMN project_path TEXT NOT NULL DEFAULT ''",
                [],
            )?;
            self.connection.execute(
                "INSERT INTO schema_migrations(version, applied_at) VALUES (?1, ?2)",
                params![3, Utc::now().to_rfc3339()],
            )?;
        }
        if version < SCHEMA_VERSION {
            self.connection.execute_batch(
                "ALTER TABLE review_decisions ADD COLUMN last_confirmed_at TEXT;
                 ALTER TABLE review_decisions ADD COLUMN last_used_at TEXT;
                 ALTER TABLE review_decisions ADD COLUMN valid_until TEXT;",
            )?;
            self.connection.execute(
                "INSERT INTO schema_migrations(version, applied_at) VALUES (?1, ?2)",
                params![SCHEMA_VERSION, Utc::now().to_rfc3339()],
            )?;
        }
        Ok(())
    }

    /// Checks whether a source has already been processed with identical content
    /// and parser version.
    ///
    /// # Errors
    ///
    /// Returns an error when `SQLite` cannot query the source record.
    pub fn is_source_current(&self, source: &SourceFingerprint) -> Result<bool, StorageError> {
        let current = self
            .connection
            .query_row(
                "SELECT size, modified_ns, content_hash, parser_version FROM sources WHERE path = ?1",
                [source.path.to_string_lossy().as_ref()],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, u32>(3)?,
                    ))
                },
            )
            .optional()?;
        Ok(
            current.is_some_and(|(size, modified_ns, hash, parser_version)| {
                Some(size) == i64::try_from(source.size).ok()
                    && modified_ns == source.modified_ns.to_string()
                    && hash == source.content_hash
                    && parser_version == source.parser_version
            }),
        )
    }

    /// Records a successfully processed source fingerprint.
    ///
    /// # Errors
    ///
    /// Returns an error when `SQLite` cannot persist the source record.
    pub fn record_source(&self, source: &SourceFingerprint) -> Result<(), StorageError> {
        let size = i64::try_from(source.size).map_err(|_| StorageError::IntegerOverflow)?;
        self.connection.execute(
            "INSERT INTO sources(path, size, modified_ns, content_hash, parser_version, processed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(path) DO UPDATE SET size=excluded.size, modified_ns=excluded.modified_ns,
             content_hash=excluded.content_hash, parser_version=excluded.parser_version,
             processed_at=excluded.processed_at",
            params![
                source.path.to_string_lossy(),
                size,
                source.modified_ns.to_string(),
                source.content_hash,
                source.parser_version,
                Utc::now().to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    /// Replaces the stored evidence for correction candidates while retaining
    /// any review decision attached to the same stable candidate identifier.
    ///
    /// # Errors
    ///
    /// Returns an error when `SQLite` cannot persist the candidates atomically.
    pub fn upsert_candidates(
        &mut self,
        candidates: &[CorrectionCandidate],
    ) -> Result<(), StorageError> {
        let transaction = self.connection.transaction()?;
        for candidate in candidates {
            let id = candidate_id(&candidate.canonical_text);
            let occurrences =
                i64::try_from(candidate.occurrences).map_err(|_| StorageError::IntegerOverflow)?;
            transaction.execute(
                "INSERT INTO candidates(id, canonical_text, occurrences, updated_at)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(id) DO UPDATE SET canonical_text=excluded.canonical_text,
                 occurrences=excluded.occurrences, updated_at=excluded.updated_at",
                params![
                    id,
                    candidate.canonical_text,
                    occurrences,
                    Utc::now().to_rfc3339()
                ],
            )?;
            transaction.execute("DELETE FROM evidence WHERE candidate_id = ?1", [&id])?;
            for evidence in &candidate.evidence {
                transaction.execute(
                    "INSERT OR IGNORE INTO evidence(candidate_id, session_id, message_id, user_text, preceding_agent_text, source_path, project_path)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        id,
                        evidence.session_id.as_str(),
                        evidence.message_id.as_ref().map(MessageId::as_str),
                        evidence.user_text,
                        evidence.preceding_agent_text,
                        evidence.source_path.as_ref().map(|path| path.to_string_lossy()),
                        evidence.project_path.as_ref().map(|path| path.to_string_lossy()),
                    ],
                )?;
            }
        }
        transaction.commit()?;
        Ok(())
    }

    /// Replaces the extracted evidence associated with one changed source and
    /// records its fingerprint in the same transaction.
    ///
    /// # Errors
    ///
    /// Returns an error when the fingerprint cannot fit in `SQLite` or the
    /// transaction cannot be committed.
    pub fn replace_source_candidates(
        &mut self,
        source: &SourceFingerprint,
        candidates: &[CorrectionCandidate],
    ) -> Result<(), StorageError> {
        let source_path = source.path.to_string_lossy();
        let size = i64::try_from(source.size).map_err(|_| StorageError::IntegerOverflow)?;
        let transaction = self.connection.transaction()?;
        transaction.execute(
            "DELETE FROM evidence WHERE source_path = ?1",
            [&source_path],
        )?;

        for candidate in candidates {
            let id = candidate_id(&candidate.canonical_text);
            transaction.execute(
                "INSERT INTO candidates(id, canonical_text, occurrences, updated_at)
                 VALUES (?1, ?2, 0, ?3)
                 ON CONFLICT(id) DO UPDATE SET canonical_text=excluded.canonical_text,
                 updated_at=excluded.updated_at",
                params![id, candidate.canonical_text, Utc::now().to_rfc3339()],
            )?;
            for evidence in &candidate.evidence {
                transaction.execute(
                    "INSERT OR IGNORE INTO evidence(candidate_id, session_id, message_id, user_text, preceding_agent_text, source_path, project_path)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        id,
                        evidence.session_id.as_str(),
                        evidence.message_id.as_ref().map(MessageId::as_str),
                        evidence.user_text,
                        evidence.preceding_agent_text,
                        source_path,
                        evidence.project_path.as_ref().map(|path| path.to_string_lossy()),
                    ],
                )?;
            }
        }
        transaction.execute(
            "UPDATE candidates SET occurrences = (
                SELECT COUNT(*) FROM evidence WHERE evidence.candidate_id = candidates.id
             )",
            [],
        )?;
        transaction.execute(
            "INSERT INTO sources(path, size, modified_ns, content_hash, parser_version, processed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(path) DO UPDATE SET size=excluded.size, modified_ns=excluded.modified_ns,
             content_hash=excluded.content_hash, parser_version=excluded.parser_version,
             processed_at=excluded.processed_at",
            params![
                source_path,
                size,
                source.modified_ns.to_string(),
                source.content_hash,
                source.parser_version,
                Utc::now().to_rfc3339(),
            ],
        )?;
        transaction.commit()?;
        Ok(())
    }

    /// Loads all correction candidates and their evidence.
    ///
    /// # Errors
    ///
    /// Returns an error when `SQLite` cannot decode a stored row.
    pub fn load_candidates(&self) -> Result<Vec<CorrectionCandidate>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT id, canonical_text, occurrences FROM candidates
             WHERE occurrences > 0 ORDER BY occurrences DESC, canonical_text",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })?;
        let mut candidates = Vec::new();
        for row in rows {
            let (id, canonical_text, occurrences) = row?;
            let occurrences =
                usize::try_from(occurrences).map_err(|_| StorageError::IntegerOverflow)?;
            let mut evidence_statement = self.connection.prepare(
                "SELECT session_id, message_id, user_text, preceding_agent_text, source_path, project_path
                 FROM evidence WHERE candidate_id = ?1 ORDER BY session_id, message_id",
            )?;
            let evidence = evidence_statement
                .query_map([id], |row| {
                    let source_path = row.get::<_, String>(4)?;
                    let project_path = row.get::<_, String>(5)?;
                    Ok(CorrectionEvidence {
                        source_path: (!source_path.is_empty()).then(|| PathBuf::from(source_path)),
                        project_path: (!project_path.is_empty())
                            .then(|| PathBuf::from(project_path)),
                        session_id: SessionId::new(row.get::<_, String>(0)?),
                        message_id: row.get::<_, Option<String>>(1)?.map(MessageId::new),
                        user_text: row.get(2)?,
                        preceding_agent_text: row.get(3)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;
            candidates.push(CorrectionCandidate {
                canonical_text,
                occurrences,
                evidence,
            });
        }
        Ok(candidates)
    }

    /// Persists an accept/reject decision independently from future re-analysis.
    ///
    /// # Errors
    ///
    /// Returns an error when the candidate is missing or `SQLite` cannot persist
    /// the decision.
    pub fn record_decision(
        &self,
        candidate_text: &str,
        decision: &ReviewDecision,
    ) -> Result<(), StorageError> {
        let id = candidate_id(candidate_text);
        let scope_json = serde_json::to_string(&decision.scope)?;
        let visibility = serde_json::to_string(&decision.visibility)?;
        let status = serde_json::to_string(&decision.status)?;
        let confirmed_at = decision.last_confirmed_at.unwrap_or_else(Utc::now);
        self.connection.execute(
            "INSERT INTO review_decisions(candidate_id, status, edited_text, scope_json, visibility, decided_at, last_confirmed_at, last_used_at, valid_until)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(candidate_id) DO UPDATE SET status=excluded.status, edited_text=excluded.edited_text,
             scope_json=excluded.scope_json, visibility=excluded.visibility, decided_at=excluded.decided_at,
             last_confirmed_at=excluded.last_confirmed_at, last_used_at=excluded.last_used_at,
             valid_until=excluded.valid_until",
            params![
                id,
                status,
                decision.edited_text,
                scope_json,
                visibility,
                Utc::now().to_rfc3339(),
                confirmed_at.to_rfc3339(),
                decision.last_used_at.map(|date| date.to_rfc3339()),
                decision.valid_until.map(|date| date.to_rfc3339()),
            ],
        )?;
        Ok(())
    }

    /// Retrieves the latest review decision for a candidate.
    ///
    /// # Errors
    ///
    /// Returns an error when `SQLite` or the stored JSON representation is invalid.
    pub fn decision(&self, candidate_text: &str) -> Result<Option<ReviewDecision>, StorageError> {
        let id = candidate_id(candidate_text);
        let stored = self
            .connection
            .query_row(
                "SELECT status, edited_text, scope_json, visibility, last_confirmed_at, last_used_at, valid_until
                 FROM review_decisions WHERE candidate_id = ?1",
                [id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, Option<String>>(4)?,
                        row.get::<_, Option<String>>(5)?,
                        row.get::<_, Option<String>>(6)?,
                    ))
                },
            )
            .optional()?;
        stored
            .map(
                |(
                    status,
                    edited_text,
                    scope,
                    visibility,
                    last_confirmed_at,
                    last_used_at,
                    valid_until,
                )| {
                    Ok(ReviewDecision {
                        status: serde_json::from_str(&status)?,
                        edited_text,
                        scope: serde_json::from_str(&scope)?,
                        visibility: serde_json::from_str(&visibility)?,
                        last_confirmed_at: parse_optional_date(last_confirmed_at)?,
                        last_used_at: parse_optional_date(last_used_at)?,
                        valid_until: parse_optional_date(valid_until)?,
                    })
                },
            )
            .transpose()
    }

    /// Loads candidates accepted during review with their effective metadata.
    ///
    /// # Errors
    ///
    /// Returns an error when `SQLite` or stored decision JSON is invalid.
    pub fn accepted_candidates(&self) -> Result<Vec<ReviewedCandidate>, StorageError> {
        let mut accepted = Vec::new();
        for candidate in self.load_candidates()? {
            if let Some(decision) = self.decision(&candidate.canonical_text)?
                && decision.status == DecisionStatus::Accepted
            {
                accepted.push(ReviewedCandidate {
                    id: candidate_id(&candidate.canonical_text),
                    canonical_text: candidate.canonical_text,
                    occurrences: candidate.occurrences,
                    decision,
                });
            }
        }
        Ok(accepted)
    }
}

#[must_use]
pub fn candidate_id(text: &str) -> String {
    format!(
        "candidate_{}",
        &blake3::hash(text.as_bytes()).to_hex()[..16]
    )
}

fn parse_optional_date(value: Option<String>) -> Result<Option<DateTime<Utc>>, StorageError> {
    value
        .map(|value| {
            DateTime::parse_from_rfc3339(&value)
                .map(|date| date.with_timezone(&Utc))
                .map_err(StorageError::InvalidDate)
        })
        .transpose()
}

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("could not read {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("invalid stored JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("integer value cannot be represented by SQLite or this platform")]
    IntegerOverflow,
    #[error("invalid stored date: {0}")]
    InvalidDate(chrono::ParseError),
}
