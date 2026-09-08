use super::{IngestError, SessionRef, SessionSource, jsonl};
use crate::domain::{AgentSource, Event, Message, NormalizedSession, Role};
use serde_json::Value;
use std::path::PathBuf;

pub struct CodexSessionSource {
    root: PathBuf,
}

impl CodexSessionSource {
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }
}

impl SessionSource for CodexSessionSource {
    fn discover(&self) -> Result<Vec<SessionRef>, IngestError> {
        jsonl::discover_jsonl(&self.root)
    }

    fn parse(&self, session: &SessionRef) -> Result<NormalizedSession, IngestError> {
        let mut normalized = NormalizedSession::new(&session.id, AgentSource::Codex);
        jsonl::for_each_value(&session.path, |value| parse_value(&mut normalized, &value))?;
        Ok(normalized)
    }
}

fn parse_value(session: &mut NormalizedSession, value: &Value) {
    let kind = value
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let payload = value.get("payload").unwrap_or(value);

    if kind == "session_meta" {
        session.project = payload
            .get("cwd")
            .and_then(Value::as_str)
            .map(PathBuf::from);
        session.started_at = jsonl::timestamp(value).or_else(|| jsonl::timestamp(payload));
        if let Some(id) = payload.get("id").and_then(Value::as_str) {
            id.clone_into(&mut session.id);
        }
    }

    let is_message = kind == "message"
        || (kind == "response_item"
            && payload.get("type").and_then(Value::as_str) == Some("message"));
    let message = if is_message { Some(payload) } else { None };

    if let Some(message) = message {
        let role = match message.get("role").and_then(Value::as_str) {
            Some("user") => Role::User,
            Some("assistant") => Role::Assistant,
            Some("system" | "developer") => Role::System,
            Some("tool") => Role::Tool,
            _ => Role::Unknown,
        };
        if let Some(content) = message.get("content").and_then(jsonl::text_content) {
            session.messages.push(Message {
                id: message.get("id").and_then(Value::as_str).map(str::to_owned),
                role,
                content,
                timestamp: jsonl::timestamp(value),
            });
            return;
        }
    }

    session.events.push(Event {
        kind: kind.to_owned(),
        timestamp: jsonl::timestamp(value),
    });
}
