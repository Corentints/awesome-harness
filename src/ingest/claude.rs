use super::{IngestError, SessionRef, SessionSource, jsonl};
use crate::domain::{AgentSource, Event, Message, MessageId, NormalizedSession, Role};
use serde_json::Value;
use std::path::PathBuf;

pub struct ClaudeSessionSource {
    root: PathBuf,
}

impl ClaudeSessionSource {
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }
}

impl SessionSource for ClaudeSessionSource {
    fn discover(&self) -> Result<Vec<SessionRef>, IngestError> {
        jsonl::discover_jsonl(&self.root)
    }

    fn parse(&self, session: &SessionRef) -> Result<NormalizedSession, IngestError> {
        let mut normalized = NormalizedSession::new(&session.id, AgentSource::Claude);
        jsonl::for_each_value(&session.path, |value| parse_value(&mut normalized, &value))?;
        Ok(normalized)
    }
}

fn parse_value(session: &mut NormalizedSession, value: &Value) {
    if session.project.is_none() {
        session.project = value.get("cwd").and_then(Value::as_str).map(PathBuf::from);
    }
    if session.started_at.is_none() {
        session.started_at = jsonl::timestamp(value);
    }

    let kind = value
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let message = value.get("message").unwrap_or(value);
    let role = match message.get("role").and_then(Value::as_str).unwrap_or(kind) {
        "user" | "human" => Role::User,
        "assistant" => Role::Assistant,
        "system" => Role::System,
        "tool" | "tool_result" => Role::Tool,
        _ => Role::Unknown,
    };

    if let Some(content) = message.get("content").and_then(jsonl::text_content) {
        session.messages.push(Message {
            id: value
                .get("uuid")
                .and_then(Value::as_str)
                .map(MessageId::from),
            role,
            content,
            timestamp: jsonl::timestamp(value),
        });
    } else {
        session.events.push(Event {
            kind: kind.to_owned(),
            timestamp: jsonl::timestamp(value),
        });
    }
}
