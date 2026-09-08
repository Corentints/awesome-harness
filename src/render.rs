use crate::{
    domain::{RuleScope, Visibility},
    project::Project,
    storage::ReviewedCandidate,
};
use std::{fmt::Write as _, path::PathBuf};

pub const START_MARKER: &str = "<!-- agentctx:start -->";
pub const END_MARKER: &str = "<!-- agentctx:end -->";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Artifact {
    pub path: PathBuf,
    pub managed_section: String,
}

pub trait Renderer {
    fn render(&self, project: &Project, rules: &[ReviewedCandidate]) -> Artifact;
}

pub struct ClaudeRenderer;
pub struct CodexRenderer;

impl Renderer for ClaudeRenderer {
    fn render(&self, project: &Project, rules: &[ReviewedCandidate]) -> Artifact {
        Artifact {
            path: project.root.join("CLAUDE.md"),
            managed_section: render_section("AgentContext instructions", project, rules),
        }
    }
}

impl Renderer for CodexRenderer {
    fn render(&self, project: &Project, rules: &[ReviewedCandidate]) -> Artifact {
        Artifact {
            path: project.root.join("AGENTS.md"),
            managed_section: render_section("AgentContext instructions", project, rules),
        }
    }
}

fn render_section(title: &str, project: &Project, rules: &[ReviewedCandidate]) -> String {
    let mut applicable = rules
        .iter()
        .filter(|rule| rule.decision.visibility == Visibility::Shared)
        .filter(|rule| match &rule.decision.scope {
            RuleScope::Global => true,
            RuleScope::Project(root) => root == &project.root,
            RuleScope::Directory(_) | RuleScope::FilePattern(_) | RuleScope::SessionOnly => false,
        })
        .map(|rule| {
            (
                rule.decision
                    .edited_text
                    .as_deref()
                    .unwrap_or(&rule.canonical_text),
                rule.id.as_str(),
            )
        })
        .collect::<Vec<_>>();
    applicable.sort_unstable();

    let mut output = format!("{START_MARKER}\n## {title}\n\n");
    for (text, id) in applicable {
        let _ = writeln!(output, "- {text} <!-- {id} -->");
    }
    output.push_str(END_MARKER);
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{DecisionStatus, ReviewDecision};

    #[test]
    fn renders_only_shared_rules_applicable_to_the_project() {
        let project = Project {
            root: "/repo".into(),
            name: "repo".to_owned(),
        };
        let rules = vec![
            reviewed(
                "a",
                "Use pnpm.",
                RuleScope::Project("/repo".into()),
                Visibility::Shared,
            ),
            reviewed("b", "Be concise.", RuleScope::Global, Visibility::Personal),
            reviewed(
                "c",
                "Temporary.",
                RuleScope::SessionOnly,
                Visibility::Shared,
            ),
        ];

        let codex = CodexRenderer.render(&project, &rules);
        let claude = ClaudeRenderer.render(&project, &rules);

        assert_eq!(codex.path, PathBuf::from("/repo/AGENTS.md"));
        assert_eq!(
            format!("{}\n", codex.managed_section),
            include_str!("../tests/fixtures/render/AGENTS.md")
        );
        assert_eq!(claude.path, PathBuf::from("/repo/CLAUDE.md"));
        assert_eq!(
            format!("{}\n", claude.managed_section),
            include_str!("../tests/fixtures/render/CLAUDE.md")
        );
    }

    fn reviewed(
        id: &str,
        text: &str,
        scope: RuleScope,
        visibility: Visibility,
    ) -> ReviewedCandidate {
        ReviewedCandidate {
            id: id.to_owned(),
            canonical_text: text.to_owned(),
            occurrences: 1,
            decision: ReviewDecision {
                status: DecisionStatus::Accepted,
                edited_text: None,
                scope,
                visibility,
            },
        }
    }
}
