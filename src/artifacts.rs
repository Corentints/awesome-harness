use crate::render::{Artifact, END_MARKER, START_MARKER};
use std::{
    fmt::Write as _,
    fs,
    io::Write,
    path::{Path, PathBuf},
};
use tempfile::NamedTempFile;
use thiserror::Error;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlannedArtifact {
    pub path: PathBuf,
    pub original: Option<String>,
    pub content: String,
}

/// Merges generated sections with existing files without touching unmanaged text.
///
/// # Errors
///
/// Returns an error when markers are incomplete, nested, or duplicated.
pub fn plan(
    project_root: &Path,
    artifacts: &[Artifact],
) -> Result<Vec<PlannedArtifact>, ArtifactError> {
    artifacts
        .iter()
        .map(|artifact| {
            let is_allowed_name = matches!(
                artifact.path.file_name().and_then(|name| name.to_str()),
                Some("AGENTS.md" | "CLAUDE.md")
            );
            if artifact.path.parent() != Some(project_root) || !is_allowed_name {
                return Err(ArtifactError::UnsafeTarget {
                    path: artifact.path.clone(),
                });
            }
            if let Ok(metadata) = fs::symlink_metadata(&artifact.path)
                && metadata.file_type().is_symlink()
            {
                return Err(ArtifactError::UnsafeTarget {
                    path: artifact.path.clone(),
                });
            }
            let original = match fs::read_to_string(&artifact.path) {
                Ok(content) => Some(content),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
                Err(source) => {
                    return Err(ArtifactError::Io {
                        path: artifact.path.clone(),
                        source,
                    });
                }
            };
            let content = merge_managed(
                original.as_deref().unwrap_or_default(),
                &artifact.managed_section,
            )?;
            Ok(PlannedArtifact {
                path: artifact.path.clone(),
                original,
                content,
            })
        })
        .collect()
}

/// Writes planned files atomically after checking that their originals did not change.
///
/// # Errors
///
/// Returns an error on concurrent changes or filesystem failures.
pub fn apply(plans: &[PlannedArtifact]) -> Result<(), ArtifactError> {
    for plan in plans {
        let current = match fs::read_to_string(&plan.path) {
            Ok(content) => Some(content),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(source) => {
                return Err(ArtifactError::Io {
                    path: plan.path.clone(),
                    source,
                });
            }
        };
        if current != plan.original {
            return Err(ArtifactError::ConcurrentChange {
                path: plan.path.clone(),
            });
        }
    }

    for plan in plans {
        let parent = plan.path.parent().unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(parent).map_err(|source| ArtifactError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
        let mut temporary = NamedTempFile::new_in(parent).map_err(|source| ArtifactError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
        temporary
            .write_all(plan.content.as_bytes())
            .map_err(|source| ArtifactError::Io {
                path: plan.path.clone(),
                source,
            })?;
        temporary
            .persist(&plan.path)
            .map_err(|error| ArtifactError::Io {
                path: plan.path.clone(),
                source: error.error,
            })?;
    }
    Ok(())
}

#[must_use]
pub fn display_diff(plan: &PlannedArtifact) -> String {
    if plan.original.as_deref() == Some(plan.content.as_str()) {
        return format!("{}: unchanged", plan.path.display());
    }
    let mut output = format!("--- {}\n+++ {}\n", plan.path.display(), plan.path.display());
    if let Some(original) = &plan.original {
        for line in original.lines() {
            let _ = writeln!(output, "-{line}");
        }
    }
    for line in plan.content.lines() {
        let _ = writeln!(output, "+{line}");
    }
    output
}

fn merge_managed(existing: &str, managed: &str) -> Result<String, ArtifactError> {
    let starts = existing.match_indices(START_MARKER).collect::<Vec<_>>();
    let ends = existing.match_indices(END_MARKER).collect::<Vec<_>>();
    match (starts.as_slice(), ends.as_slice()) {
        ([], []) => {
            if existing.is_empty() {
                Ok(format!("{managed}\n"))
            } else {
                Ok(format!("{}\n\n{managed}\n", existing.trim_end()))
            }
        }
        ([(start, _)], [(end, _)]) if start < end => {
            let suffix_start = end + END_MARKER.len();
            Ok(format!(
                "{}{}{}",
                &existing[..*start],
                managed,
                &existing[suffix_start..]
            ))
        }
        _ => Err(ArtifactError::InvalidMarkers),
    }
}

#[derive(Debug, Error)]
pub enum ArtifactError {
    #[error("invalid or duplicated AgentContext section markers")]
    InvalidMarkers,
    #[error("{path} changed after the diff was computed")]
    ConcurrentChange { path: PathBuf },
    #[error("refusing to write unsafe instruction target {path}")]
    UnsafeTarget { path: PathBuf },
    #[error("filesystem error for {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_unmanaged_content_when_replacing_a_section() {
        let existing = "# Team rules\n\nBefore.\n\n<!-- agentctx:start -->\nold\n<!-- agentctx:end -->\n\nAfter.\n";
        let managed = "<!-- agentctx:start -->\nnew\n<!-- agentctx:end -->";

        let merged = merge_managed(existing, managed).expect("valid markers");

        assert_eq!(
            merged,
            "# Team rules\n\nBefore.\n\n<!-- agentctx:start -->\nnew\n<!-- agentctx:end -->\n\nAfter.\n"
        );
    }

    #[test]
    fn rejects_incomplete_markers() {
        assert!(matches!(
            merge_managed(START_MARKER, "new"),
            Err(ArtifactError::InvalidMarkers)
        ));
    }

    #[test]
    fn rejects_artifacts_outside_the_project_root() {
        let artifact = Artifact {
            path: PathBuf::from("/outside/AGENTS.md"),
            managed_section: "rules".to_owned(),
        };
        assert!(matches!(
            plan(Path::new("/repo"), &[artifact]),
            Err(ArtifactError::UnsafeTarget { .. })
        ));
    }

    #[test]
    fn detects_a_change_between_plan_and_apply() {
        let directory = tempfile::tempdir().expect("temp directory");
        let path = directory.path().join("AGENTS.md");
        fs::write(&path, "before\n").expect("initial file");
        let artifact = Artifact {
            path: path.clone(),
            managed_section: "rules".to_owned(),
        };
        let plans = plan(directory.path(), &[artifact]).expect("plan");
        fs::write(&path, "changed\n").expect("concurrent edit");

        assert!(matches!(
            apply(&plans),
            Err(ArtifactError::ConcurrentChange { .. })
        ));
        assert_eq!(fs::read_to_string(path).expect("content"), "changed\n");
    }

    #[cfg(unix)]
    #[test]
    fn refuses_a_symlink_target() {
        use std::os::unix::fs::symlink;

        let directory = tempfile::tempdir().expect("temp directory");
        let outside = tempfile::NamedTempFile::new().expect("outside file");
        let target = directory.path().join("AGENTS.md");
        symlink(outside.path(), &target).expect("symlink");
        let artifact = Artifact {
            path: target,
            managed_section: "rules".to_owned(),
        };

        assert!(matches!(
            plan(directory.path(), &[artifact]),
            Err(ArtifactError::UnsafeTarget { .. })
        ));
    }
}
