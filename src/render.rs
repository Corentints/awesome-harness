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
    fn render(&self, project: &Project, rules: &[ReviewedCandidate]) -> Vec<Artifact>;
}

pub struct ClaudeRenderer;
pub struct CodexRenderer;

impl Renderer for ClaudeRenderer {
    fn render(&self, project: &Project, rules: &[ReviewedCandidate]) -> Vec<Artifact> {
        let mut artifacts = vec![Artifact {
            path: project.root.join("CLAUDE.md"),
            managed_section: render_section(
                "AgentContext instructions",
                rules.iter().filter(|rule| {
                    rule.decision.visibility == Visibility::Shared
                        && is_root_scope(&rule.decision.scope, project)
                }),
            ),
        }];
        let personal = rules
            .iter()
            .filter(|rule| {
                rule.decision.visibility == Visibility::Personal
                    && is_root_scope(&rule.decision.scope, project)
            })
            .collect::<Vec<_>>();
        if !personal.is_empty() {
            artifacts.push(Artifact {
                path: project.root.join("CLAUDE.local.md"),
                managed_section: render_section("Personal AgentContext instructions", personal),
            });
        }
        for rule in rules.iter().filter(|rule| {
            rule.decision.visibility == Visibility::Shared
                && matches!(
                    rule.decision.scope,
                    RuleScope::Directory(_) | RuleScope::FilePattern(_)
                )
        }) {
            let pattern = match &rule.decision.scope {
                RuleScope::Directory(directory) => format!("{}/**", directory.display()),
                RuleScope::FilePattern(pattern) => pattern.clone(),
                _ => continue,
            };
            let section = render_section("AgentContext scoped instruction", [rule]);
            artifacts.push(Artifact {
                path: project
                    .root
                    .join(format!(".claude/rules/agentctx-{}.md", rule.id)),
                managed_section: format!("---\npaths:\n  - \"{pattern}\"\n---\n\n{section}"),
            });
        }
        artifacts
    }
}

impl Renderer for CodexRenderer {
    fn render(&self, project: &Project, rules: &[ReviewedCandidate]) -> Vec<Artifact> {
        let mut root_rules = rules
            .iter()
            .filter(|rule| {
                rule.decision.visibility == Visibility::Shared
                    && (is_root_scope(&rule.decision.scope, project)
                        || matches!(rule.decision.scope, RuleScope::FilePattern(_)))
            })
            .collect::<Vec<_>>();
        root_rules.sort_by_key(|rule| rule.id.as_str());
        let mut artifacts = vec![Artifact {
            path: project.root.join("AGENTS.md"),
            managed_section: render_codex_root(&root_rules),
        }];
        for rule in rules.iter().filter(|rule| {
            rule.decision.visibility == Visibility::Shared
                && matches!(rule.decision.scope, RuleScope::Directory(_))
        }) {
            let RuleScope::Directory(directory) = &rule.decision.scope else {
                continue;
            };
            artifacts.push(Artifact {
                path: project.root.join(directory).join("AGENTS.md"),
                managed_section: render_section("AgentContext directory instructions", [rule]),
            });
        }
        artifacts
    }
}

fn render_codex_root(rules: &[&ReviewedCandidate]) -> String {
    let mut output = format!("{START_MARKER}\n## AgentContext instructions\n\n");
    for rule in rules {
        let text = effective_text(rule);
        if let RuleScope::FilePattern(pattern) = &rule.decision.scope {
            let _ = writeln!(
                output,
                "- For files matching `{pattern}`: {text} <!-- {} -->",
                rule.id
            );
        } else {
            let _ = writeln!(output, "- {text} <!-- {} -->", rule.id);
        }
    }
    output.push_str(END_MARKER);
    output
}

fn render_section<'a>(
    title: &str,
    rules: impl IntoIterator<Item = &'a ReviewedCandidate>,
) -> String {
    let mut applicable = rules.into_iter().collect::<Vec<_>>();
    applicable.sort_by(|left, right| effective_text(left).cmp(effective_text(right)));
    let mut output = format!("{START_MARKER}\n## {title}\n\n");
    for rule in applicable {
        let _ = writeln!(output, "- {} <!-- {} -->", effective_text(rule), rule.id);
    }
    output.push_str(END_MARKER);
    output
}

fn is_root_scope(scope: &RuleScope, project: &Project) -> bool {
    matches!(scope, RuleScope::Global)
        || matches!(scope, RuleScope::Project(root) if root == &project.root)
}

fn effective_text(rule: &ReviewedCandidate) -> &str {
    rule.decision
        .edited_text
        .as_deref()
        .unwrap_or(&rule.canonical_text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{DecisionStatus, ReviewDecision};

    #[test]
    fn renders_root_personal_and_native_directory_targets_without_duplication() {
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
                "Never use Prisma.",
                RuleScope::Directory("packages/api".into()),
                Visibility::Shared,
            ),
        ];

        let codex = CodexRenderer.render(&project, &rules);
        let claude = ClaudeRenderer.render(&project, &rules);

        assert_eq!(codex.len(), 2);
        assert_eq!(codex[0].path, PathBuf::from("/repo/AGENTS.md"));
        assert_eq!(
            format!("{}\n", codex[0].managed_section),
            include_str!("../tests/fixtures/render/AGENTS.md")
        );
        assert_eq!(codex[1].path, PathBuf::from("/repo/packages/api/AGENTS.md"));
        assert!(!codex[0].managed_section.contains("Prisma"));
        assert!(codex[1].managed_section.contains("Prisma"));

        assert_eq!(claude.len(), 3);
        assert_eq!(
            format!("{}\n", claude[0].managed_section),
            include_str!("../tests/fixtures/render/CLAUDE.md")
        );
        assert_eq!(claude[1].path, PathBuf::from("/repo/CLAUDE.local.md"));
        assert!(claude[1].managed_section.contains("Be concise"));
        assert!(
            claude[2]
                .path
                .starts_with(std::path::Path::new("/repo/.claude/rules"))
        );
        assert!(claude[2].managed_section.contains("packages/api/**"));
    }

    #[test]
    fn adapts_file_patterns_to_each_agent() {
        let project = Project {
            root: "/repo".into(),
            name: "repo".to_owned(),
        };
        let rules = [reviewed(
            "pattern",
            "Run the frontend linter.",
            RuleScope::FilePattern("src/**/*.ts".to_owned()),
            Visibility::Shared,
        )];

        let claude = ClaudeRenderer.render(&project, &rules);
        let codex = CodexRenderer.render(&project, &rules);

        assert!(claude[1].managed_section.contains("src/**/*.ts"));
        assert!(
            codex[0]
                .managed_section
                .contains("For files matching `src/**/*.ts`")
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
                last_confirmed_at: None,
                last_used_at: None,
                valid_until: None,
            },
        }
    }
}
