use super::RuleId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeKind {
    UserPreference,
    CodingConvention,
    ArchitectureRule,
    Workflow,
    Command,
    ProjectFact,
    DirectoryRule,
    ToolPreference,
    Prohibition,
    DebuggingKnowledge,
    TemporaryInstruction,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum RuleScope {
    Global,
    Project(PathBuf),
    Directory(PathBuf),
    FilePattern(String),
    SessionOnly,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    Shared,
    Personal,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleStatus {
    Candidate,
    Accepted,
    Rejected,
    Superseded,
    Stale,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    ExplicitInstruction,
    Correction,
    Repository,
    GitHistory,
    ExistingRule,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Evidence {
    pub kind: EvidenceKind,
    pub source: String,
    pub excerpt: Option<String>,
    pub observed_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rule {
    pub id: RuleId,
    pub canonical_text: String,
    pub kind: KnowledgeKind,
    pub scope: RuleScope,
    pub visibility: Visibility,
    pub confidence: f32,
    pub importance: f32,
    pub first_seen_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub occurrences: u32,
    pub evidence: Vec<Evidence>,
    pub status: RuleStatus,
}
