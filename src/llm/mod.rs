mod codex_cli;

pub use codex_cli::CodexCliProvider;

use crate::{domain::MessageId, privacy::Redactor};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::BTreeMap, sync::Mutex};
use thiserror::Error;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct InferenceSegment {
    pub message_id: Option<MessageId>,
    pub text: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct InferenceRequest {
    pub segments: Vec<InferenceSegment>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InferenceResponse {
    pub rules: Vec<InferredRule>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InferredRule {
    pub text: String,
    pub kind: String,
    pub scope: String,
    pub confidence: f32,
    pub evidence_message_ids: Vec<MessageId>,
}

pub trait InferenceProvider {
    /// Infers normalized rules from an already-minimized and redacted request.
    ///
    /// # Errors
    ///
    /// Returns an error when the provider fails or returns an invalid response.
    fn infer(&self, request: &InferenceRequest) -> Result<InferenceResponse, InferenceError>;
}

/// Redacts every segment before handing the request to a provider.
///
/// # Errors
///
/// Returns the provider's error after redaction has completed.
pub fn infer_redacted(
    provider: &dyn InferenceProvider,
    request: &InferenceRequest,
    redactor: &mut Redactor,
) -> Result<InferenceResponse, InferenceError> {
    let request = InferenceRequest {
        segments: request
            .segments
            .iter()
            .map(|segment| InferenceSegment {
                message_id: segment.message_id.clone(),
                text: redactor.redact(&segment.text),
            })
            .collect(),
    };
    provider.infer(&request)
}

#[must_use]
pub fn batches(segments: &[InferenceSegment], batch_size: usize) -> Vec<InferenceRequest> {
    let batch_size = batch_size.max(1);
    segments
        .chunks(batch_size)
        .map(|segments| InferenceRequest {
            segments: segments.to_vec(),
        })
        .collect()
}

#[must_use]
pub fn output_schema() -> Value {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "type": "object",
        "additionalProperties": false,
        "required": ["rules"],
        "properties": {
            "rules": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["text", "kind", "scope", "confidence", "evidence_message_ids"],
                    "properties": {
                        "text": {"type": "string", "minLength": 1},
                        "kind": {"type": "string", "enum": ["user_preference", "coding_convention", "architecture_rule", "workflow", "command", "project_fact", "directory_rule", "tool_preference", "prohibition", "debugging_knowledge", "temporary_instruction"]},
                        "scope": {"type": "string", "enum": ["global", "project", "directory", "file_pattern", "session_only"]},
                        "confidence": {"type": "number", "minimum": 0, "maximum": 1},
                        "evidence_message_ids": {"type": "array", "items": {"type": "string"}}
                    }
                }
            }
        }
    })
}

#[derive(Default)]
pub struct FakeProvider {
    recorded: Mutex<Vec<InferenceRequest>>,
    responses: BTreeMap<String, InferredRule>,
}

impl FakeProvider {
    #[must_use]
    pub fn with_rule(trigger: impl Into<String>, rule: InferredRule) -> Self {
        Self {
            recorded: Mutex::new(Vec::new()),
            responses: BTreeMap::from([(trigger.into(), rule)]),
        }
    }

    #[must_use]
    pub fn recorded_requests(&self) -> Vec<InferenceRequest> {
        self.recorded
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}

impl InferenceProvider for FakeProvider {
    fn infer(&self, request: &InferenceRequest) -> Result<InferenceResponse, InferenceError> {
        self.recorded
            .lock()
            .map_err(|_| InferenceError::Provider("fake provider lock is poisoned".to_owned()))?
            .push(request.clone());
        let rules = self
            .responses
            .iter()
            .filter(|(trigger, _)| {
                request
                    .segments
                    .iter()
                    .any(|segment| segment.text.contains(*trigger))
            })
            .map(|(_, rule)| rule.clone())
            .collect();
        Ok(InferenceResponse { rules })
    }
}

#[derive(Debug, Error)]
pub enum InferenceError {
    #[error("provider failed: {0}")]
    Provider(String),
    #[error("provider returned invalid JSON: {0}")]
    InvalidResponse(#[from] serde_json::Error),
    #[error("provider filesystem error: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batches_never_create_empty_or_oversized_requests() {
        let segments = (0..5)
            .map(|index| InferenceSegment {
                message_id: None,
                text: index.to_string(),
            })
            .collect::<Vec<_>>();
        let requests = batches(&segments, 2);
        assert_eq!(
            requests
                .iter()
                .map(|request| request.segments.len())
                .collect::<Vec<_>>(),
            [2, 2, 1]
        );
    }

    #[test]
    fn secrets_do_not_reach_a_provider() {
        let provider = FakeProvider::default();
        let request = InferenceRequest {
            segments: vec![InferenceSegment {
                message_id: None,
                text: "Bearer abcdefghijklmnopqrstuvwxyz".to_owned(),
            }],
        };
        infer_redacted(&provider, &request, &mut Redactor::new()).expect("fake inference");
        let recorded = provider.recorded_requests();
        assert!(
            !recorded[0].segments[0]
                .text
                .contains("abcdefghijklmnopqrstuvwxyz")
        );
    }

    #[test]
    fn schema_rejects_unknown_output_fields_by_contract() {
        let schema = output_schema();
        assert_eq!(schema["additionalProperties"], false);
        assert_eq!(
            schema["properties"]["rules"]["items"]["additionalProperties"],
            false
        );
    }
}
