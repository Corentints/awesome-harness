use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Project {
    pub root: PathBuf,
    pub name: String,
}

impl Project {
    /// Finds the closest Git repository containing `start`.
    ///
    /// # Errors
    ///
    /// Returns an error when `start` cannot be resolved or no `.git` entry exists
    /// in any ancestor.
    pub fn detect(start: &Path) -> Result<Self, ProjectError> {
        let absolute = start.canonicalize().map_err(|source| ProjectError::Io {
            path: start.to_path_buf(),
            source,
        })?;
        let mut cursor = absolute.as_path();

        loop {
            if cursor.join(".git").exists() {
                let root = cursor.to_path_buf();
                let name = root.file_name().map_or_else(
                    || root.display().to_string(),
                    |value| value.to_string_lossy().into_owned(),
                );
                return Ok(Self { root, name });
            }
            cursor = cursor.parent().ok_or(ProjectError::NotGitRepository {
                start: absolute.clone(),
            })?;
        }
    }

    #[must_use]
    pub fn contains_path(&self, path: &Path) -> bool {
        path == self.root || path.starts_with(&self.root)
    }
}

#[derive(Debug, Error)]
pub enum ProjectError {
    #[error("could not resolve {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("{start} is not inside a Git repository")]
    NotGitRepository { start: PathBuf },
}
