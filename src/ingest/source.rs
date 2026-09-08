use crate::domain::NormalizedSession;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionRef {
    pub id: String,
    pub path: PathBuf,
}

pub trait SessionSource {
    /// Finds the JSONL sessions available below the configured source root.
    ///
    /// # Errors
    ///
    /// Returns [`IngestError::Io`] when the source root cannot be traversed.
    fn discover(&self) -> Result<Vec<SessionRef>, IngestError>;

    /// Converts one source-specific JSONL file into the shared session model.
    ///
    /// # Errors
    ///
    /// Returns an error when the file cannot be read or contains invalid JSON.
    fn parse(&self, session: &SessionRef) -> Result<NormalizedSession, IngestError>;
}

#[derive(Debug, Error)]
pub enum IngestError {
    #[error("could not read {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid JSON at {path}:{line}: {source}")]
    InvalidJson {
        path: PathBuf,
        line: usize,
        #[source]
        source: serde_json::Error,
    },
}
