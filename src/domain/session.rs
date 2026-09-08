use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentSource {
    Claude,
    Codex,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    User,
    Assistant,
    System,
    Tool,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub id: Option<String>,
    pub role: Role,
    pub content: String,
    pub timestamp: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Event {
    pub kind: String,
    pub timestamp: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NormalizedSession {
    pub id: String,
    pub source: AgentSource,
    pub project: Option<PathBuf>,
    pub started_at: Option<DateTime<Utc>>,
    pub messages: Vec<Message>,
    pub events: Vec<Event>,
}

impl NormalizedSession {
    #[must_use]
    pub fn new(id: impl Into<String>, source: AgentSource) -> Self {
        Self {
            id: id.into(),
            source,
            project: None,
            started_at: None,
            messages: Vec::new(),
            events: Vec::new(),
        }
    }
}
