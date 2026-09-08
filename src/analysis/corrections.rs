use crate::domain::{MessageId, NormalizedSession, Role, SessionId};
use std::collections::BTreeMap;
use std::path::PathBuf;

const SIGNALS: &[&str] = &[
    "always",
    "never",
    "instead",
    "from now on",
    "don't ",
    "do not ",
    "prefer ",
    "use ",
    "toujours",
    "jamais",
    "plutôt que",
    "a l'avenir",
    "à l'avenir",
    "n'utilise pas",
    "ne pas ",
    "préfère",
    "utilise ",
    "non,",
    "no,",
    "je t'ai déjà dit",
];
const CORRECTION_PREFIXES: &[&str] = &["no,", "non,", "no:", "non:", "je t'ai déjà dit,"];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CorrectionEvidence {
    pub source_path: Option<PathBuf>,
    pub session_id: SessionId,
    pub message_id: Option<MessageId>,
    pub user_text: String,
    pub preceding_agent_text: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CorrectionCandidate {
    pub canonical_text: String,
    pub occurrences: usize,
    pub evidence: Vec<CorrectionEvidence>,
}

/// Finds explicit instructions and corrections, grouping equivalent normalized text.
#[must_use]
pub fn find_corrections(sessions: &[NormalizedSession]) -> Vec<CorrectionCandidate> {
    let mut grouped = BTreeMap::<String, CorrectionCandidate>::new();

    for session in sessions {
        for (index, message) in session.messages.iter().enumerate() {
            if message.role != Role::User || !is_signal(&message.content) {
                continue;
            }

            let canonical_text = canonicalize(&message.content);
            let key = normalize_for_grouping(&canonical_text);
            if key.is_empty() {
                continue;
            }
            let preceding_agent_text = session.messages[..index]
                .iter()
                .rev()
                .find(|candidate| candidate.role == Role::Assistant)
                .map(|candidate| candidate.content.clone());
            let evidence = CorrectionEvidence {
                source_path: session.origin.clone(),
                session_id: session.id.clone(),
                message_id: message.id.clone(),
                user_text: message.content.clone(),
                preceding_agent_text,
            };

            grouped
                .entry(key)
                .and_modify(|candidate| {
                    candidate.occurrences += 1;
                    candidate.evidence.push(evidence.clone());
                })
                .or_insert_with(|| CorrectionCandidate {
                    canonical_text,
                    occurrences: 1,
                    evidence: vec![evidence],
                });
        }
    }

    let mut candidates = grouped.into_values().collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        right
            .occurrences
            .cmp(&left.occurrences)
            .then_with(|| left.canonical_text.cmp(&right.canonical_text))
    });
    candidates
}

fn is_signal(text: &str) -> bool {
    let normalized = text.to_lowercase();
    SIGNALS.iter().any(|signal| normalized.contains(signal))
}

fn canonicalize(text: &str) -> String {
    let trimmed = text.trim();
    let lowercase = trimmed.to_lowercase();
    let without_prefix = CORRECTION_PREFIXES
        .iter()
        .find_map(|prefix| {
            lowercase
                .starts_with(prefix)
                .then(|| &trimmed[prefix.len()..])
        })
        .unwrap_or(trimmed)
        .trim();

    let mut chars = without_prefix.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

fn normalize_for_grouping(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|character| {
            if character.is_alphanumeric() {
                character
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{AgentSource, Message};

    fn message(role: Role, content: &str) -> Message {
        Message {
            id: None,
            role,
            content: content.to_owned(),
            timestamp: None,
        }
    }

    #[test]
    fn groups_repeated_corrections_and_keeps_agent_context() {
        let mut first = NormalizedSession::new("one", AgentSource::Claude);
        first.messages = vec![
            message(Role::Assistant, "I will use npm."),
            message(Role::User, "No, always use pnpm."),
        ];
        let mut second = NormalizedSession::new("two", AgentSource::Codex);
        second.messages = vec![
            message(Role::Assistant, "Installing with npm."),
            message(Role::User, "Always use pnpm!"),
        ];

        let candidates = find_corrections(&[first, second]);

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].canonical_text, "Always use pnpm.");
        assert_eq!(candidates[0].occurrences, 2);
        assert_eq!(
            candidates[0].evidence[0].preceding_agent_text.as_deref(),
            Some("I will use npm.")
        );
    }

    #[test]
    fn ignores_generic_user_requests() {
        let mut session = NormalizedSession::new("one", AgentSource::Claude);
        session.messages = vec![message(Role::User, "Please implement the feature.")];

        assert!(find_corrections(&[session]).is_empty());
    }
}
