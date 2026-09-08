use super::CorrectionCandidate;
use crate::domain::{RuleScope, Visibility};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScopeInference {
    pub scope: RuleScope,
    pub visibility: Visibility,
    pub reason: String,
}

#[must_use]
pub fn infer_scope(candidate: &CorrectionCandidate, current_project: &Path) -> ScopeInference {
    let text = candidate.canonical_text.to_lowercase();
    if [
        "for this migration",
        "for this task",
        "pour cette migration",
        "pour cette tâche",
    ]
    .iter()
    .any(|marker| text.contains(marker))
    {
        return ScopeInference {
            scope: RuleScope::SessionOnly,
            visibility: infer_visibility(&text),
            reason: "the instruction explicitly describes temporary work".to_owned(),
        };
    }
    if let Some(directory) = extract_directory(&text) {
        return ScopeInference {
            scope: RuleScope::Directory(directory.clone()),
            visibility: infer_visibility(&text),
            reason: format!(
                "the instruction explicitly names directory {}",
                directory.display()
            ),
        };
    }
    let diversity = project_diversity(candidate);
    let explicitly_global = ["all my projects", "every project", "tous mes projets"]
        .iter()
        .any(|marker| text.contains(marker));
    if explicitly_global || diversity >= 3 {
        ScopeInference {
            scope: RuleScope::Global,
            visibility: infer_visibility(&text),
            reason: if explicitly_global {
                "the instruction explicitly applies across projects".to_owned()
            } else {
                format!("independent evidence exists in {diversity} projects")
            },
        }
    } else {
        ScopeInference {
            scope: RuleScope::Project(current_project.to_path_buf()),
            visibility: infer_visibility(&text),
            reason: "evidence is limited to the current project".to_owned(),
        }
    }
}

#[must_use]
pub fn project_diversity(candidate: &CorrectionCandidate) -> usize {
    candidate
        .evidence
        .iter()
        .filter_map(|evidence| evidence.project_path.as_ref())
        .collect::<BTreeSet<_>>()
        .len()
}

fn infer_visibility(text: &str) -> Visibility {
    if [
        "respond",
        "réponds",
        "ask for confirmation",
        "demande confirmation",
        "be concise",
    ]
    .iter()
    .any(|marker| text.contains(marker))
    {
        Visibility::Personal
    } else {
        Visibility::Shared
    }
}

fn extract_directory(text: &str) -> Option<PathBuf> {
    for marker in [" in ", " dans "] {
        let Some((_, suffix)) = text.split_once(marker) else {
            continue;
        };
        let candidate = suffix
            .split_whitespace()
            .next()?
            .trim_matches(|character: char| {
                !character.is_alphanumeric() && !"./_-".contains(character)
            })
            .trim_end_matches(['.', ',', ';', ':']);
        if candidate.contains('/') && !candidate.starts_with('/') && !candidate.contains("..") {
            return Some(PathBuf::from(candidate));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{analysis::CorrectionEvidence, domain::SessionId};

    fn candidate(text: &str, projects: &[&str]) -> CorrectionCandidate {
        CorrectionCandidate {
            canonical_text: text.to_owned(),
            occurrences: projects.len(),
            evidence: projects
                .iter()
                .map(|project| CorrectionEvidence {
                    source_path: None,
                    project_path: Some(PathBuf::from(project)),
                    session_id: SessionId::new(project.to_string()),
                    message_id: None,
                    user_text: text.to_owned(),
                    preceding_agent_text: None,
                })
                .collect(),
        }
    }

    #[test]
    fn promotes_cross_project_evidence_to_global_scope() {
        let inferred = infer_scope(
            &candidate("Prefer pnpm.", &["/a", "/b", "/c"]),
            Path::new("/a"),
        );
        assert_eq!(inferred.scope, RuleScope::Global);
        assert!(inferred.reason.contains("3 projects"));
    }

    #[test]
    fn keeps_single_project_evidence_at_project_scope() {
        let inferred = infer_scope(&candidate("Use pnpm.", &["/a"]), Path::new("/a"));
        assert_eq!(inferred.scope, RuleScope::Project(PathBuf::from("/a")));
    }

    #[test]
    fn recognizes_directory_and_personal_preferences() {
        let directory = infer_scope(
            &candidate("Never use Prisma in packages/api.", &["/a"]),
            Path::new("/a"),
        );
        assert_eq!(
            directory.scope,
            RuleScope::Directory(PathBuf::from("packages/api"))
        );

        let personal = infer_scope(
            &candidate("Be concise when you respond.", &["/a"]),
            Path::new("/a"),
        );
        assert_eq!(personal.visibility, Visibility::Personal);
    }
}
