mod corrections;
mod inputs;
mod scope;
mod scoring;

pub use corrections::{CorrectionCandidate, CorrectionEvidence, find_corrections};
pub use inputs::{
    InputPriority, PrioritizedInput, PriorityReason, is_durable_signal, prioritize_user_inputs,
};
pub use scope::{ScopeInference, infer_scope, project_diversity};
pub use scoring::{ScoreBreakdown, score_candidate};
