use super::{MessageId, SessionId};
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
    pub id: Option<MessageId>,
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
    pub id: SessionId,
    pub source: AgentSource,
    pub origin: Option<PathBuf>,
    pub project: Option<PathBuf>,
    pub started_at: Option<DateTime<Utc>>,
    pub messages: Vec<Message>,
    pub events: Vec<Event>,
}

impl NormalizedSession {
    #[must_use]
    pub fn new(id: impl Into<String>, source: AgentSource) -> Self {
        Self {
            id: SessionId::new(id),
            source,
            origin: None,
            project: None,
            started_at: None,
            messages: Vec::new(),
            events: Vec::new(),
        }
    }
}
