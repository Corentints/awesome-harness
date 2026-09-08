mod corrections;
mod scope;
mod scoring;

pub use corrections::{CorrectionCandidate, CorrectionEvidence, find_corrections};
pub use scope::{ScopeInference, infer_scope, project_diversity};
pub use scoring::{ScoreBreakdown, score_candidate};
