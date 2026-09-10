use crate::domain::{MessageId, NormalizedSession, Role, SessionId};
use std::collections::BTreeSet;

const TECHNICAL_ONLY_PREFIXES: &[&str] = &[
    "transcript delta start",
    "treat the transcript delta",
    "<environment_context>",
    "<permissions instructions>",
    "<system-reminder>",
];
const STRONG_SIGNALS: &[&str] = &[
    "always",
    "never",
    "instead",
    "from now on",
    "don't ",
    "do not ",
    "toujours",
    "jamais",
    "plutôt que",
    "a l'avenir",
    "à l'avenir",
    "n'utilise pas",
    "ne pas ",
    "je t'ai déjà dit",
];
const CORRECTION_PREFIXES: &[&str] = &["no,", "non,", "no:", "non:"];
const PREFERENCE_PREFIXES: &[&str] = &["prefer ", "préfère ", "use ", "utilise "];

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum InputPriority {
    Low,
    Medium,
    High,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PriorityReason {
    OrdinaryRequest,
    Preference,
    ExplicitCorrection,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PrioritizedInput {
    pub session_id: SessionId,
    pub message_index: usize,
    pub message_id: Option<MessageId>,
    pub text: String,
    pub priority: InputPriority,
    pub reason: PriorityReason,
}

/// Extracts every genuine user input and orders it from strongest to weakest
/// durable-rule signal. Exact duplicates inside one session are ignored.
#[must_use]
pub fn prioritize_user_inputs(session: &NormalizedSession) -> Vec<PrioritizedInput> {
    let mut seen = BTreeSet::new();
    let mut inputs = session
        .messages
        .iter()
        .enumerate()
        .filter(|(_, message)| message.role == Role::User)
        .filter_map(|(message_index, message)| {
            let text = genuine_user_text(&message.content)?;
            let duplicate_key = normalize_for_deduplication(&text);
            if duplicate_key.is_empty() || !seen.insert(duplicate_key) {
                return None;
            }
            let (priority, reason) = priority(&text);
            Some(PrioritizedInput {
                session_id: session.id.clone(),
                message_index,
                message_id: message.id.clone(),
                text,
                priority,
                reason,
            })
        })
        .collect::<Vec<_>>();
    inputs.sort_by_key(|input| std::cmp::Reverse(input.priority));
    inputs
}

#[must_use]
pub fn is_durable_signal(input: &PrioritizedInput) -> bool {
    input.priority != InputPriority::Low
}

fn genuine_user_text(content: &str) -> Option<String> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return None;
    }

    let extracted = ["## My request:", "## Ma demande :", "## Ma demande:"]
        .iter()
        .find_map(|marker| {
            trimmed
                .rsplit_once(marker)
                .map(|(_, request)| request.trim())
        })
        .unwrap_or(trimmed);
    if extracted.is_empty() {
        return None;
    }

    let normalized = extracted.to_lowercase();
    if TECHNICAL_ONLY_PREFIXES
        .iter()
        .any(|prefix| normalized.starts_with(prefix))
    {
        return None;
    }
    Some(extracted.to_owned())
}

fn priority(text: &str) -> (InputPriority, PriorityReason) {
    let normalized = text.trim().to_lowercase();
    let without_politeness = normalized
        .strip_prefix("please ")
        .or_else(|| normalized.strip_prefix("s'il te plaît "))
        .or_else(|| normalized.strip_prefix("stp "))
        .unwrap_or(&normalized);
    if STRONG_SIGNALS
        .iter()
        .any(|signal| normalized.contains(signal))
        || CORRECTION_PREFIXES
            .iter()
            .any(|prefix| without_politeness.starts_with(prefix))
    {
        return (InputPriority::High, PriorityReason::ExplicitCorrection);
    }
    if PREFERENCE_PREFIXES
        .iter()
        .any(|prefix| without_politeness.starts_with(prefix))
    {
        return (InputPriority::Medium, PriorityReason::Preference);
    }
    (InputPriority::Low, PriorityReason::OrdinaryRequest)
}

fn normalize_for_deduplication(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{AgentSource, Message};

    fn user(content: &str) -> Message {
        Message {
            id: None,
            role: Role::User,
            content: content.to_owned(),
            timestamp: None,
        }
    }

    #[test]
    fn keeps_ordinary_requests_at_low_priority() {
        let mut session = NormalizedSession::new("one", AgentSource::Codex);
        session.messages = vec![user("Could you refactor this module?")];

        let inputs = prioritize_user_inputs(&session);

        assert_eq!(inputs.len(), 1);
        assert_eq!(inputs[0].priority, InputPriority::Low);
        assert_eq!(inputs[0].reason, PriorityReason::OrdinaryRequest);
    }

    #[test]
    fn extracts_the_request_from_ide_context() {
        let mut session = NormalizedSession::new("one", AgentSource::Codex);
        session.messages = vec![user(
            "# Context from my IDE setup:\n\n## Active file: main.rs\n\n## My request:\nAlways run clippy.",
        )];

        let inputs = prioritize_user_inputs(&session);

        assert_eq!(inputs[0].text, "Always run clippy.");
        assert_eq!(inputs[0].priority, InputPriority::High);
    }

    #[test]
    fn excludes_technical_messages_and_session_duplicates() {
        let mut session = NormalizedSession::new("one", AgentSource::Codex);
        session.messages = vec![
            user("<environment_context>private metadata</environment_context>"),
            user("Use pnpm."),
            user("  use   pnpm.  "),
        ];

        let inputs = prioritize_user_inputs(&session);

        assert_eq!(inputs.len(), 1);
        assert_eq!(inputs[0].text, "Use pnpm.");
    }

    #[test]
    fn orders_explicit_corrections_before_other_inputs() {
        let mut session = NormalizedSession::new("one", AgentSource::Claude);
        session.messages = vec![
            user("Implement the feature."),
            user("Prefer small functions."),
            user("No, never edit generated files."),
        ];

        let inputs = prioritize_user_inputs(&session);

        assert_eq!(inputs[0].priority, InputPriority::High);
        assert_eq!(inputs[1].priority, InputPriority::Medium);
        assert_eq!(inputs[2].priority, InputPriority::Low);
    }
}
