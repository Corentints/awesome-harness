use crate::{repository::RepositoryFacts, storage::ReviewedCandidate};
use chrono::{DateTime, Duration, Utc};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IssueKind {
    Conflict,
    Stale,
    RepositoryContradiction,
    InvalidCommand,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DoctorIssue {
    pub kind: IssueKind,
    pub rule_ids: Vec<String>,
    pub message: String,
    pub recommendation: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DoctorReport {
    pub health: u8,
    pub issues: Vec<DoctorIssue>,
}

#[must_use]
pub fn diagnose(
    rules: &[ReviewedCandidate],
    facts: &RepositoryFacts,
    now: DateTime<Utc>,
) -> DoctorReport {
    let mut issues = Vec::new();
    detect_package_manager_conflicts(rules, facts, &mut issues);
    detect_staleness(rules, now, &mut issues);
    detect_invalid_commands(rules, facts, &mut issues);
    let penalty = u8::try_from(issues.len().saturating_mul(10).min(100)).unwrap_or(100);
    DoctorReport {
        health: 100 - penalty,
        issues,
    }
}

fn detect_package_manager_conflicts(
    rules: &[ReviewedCandidate],
    facts: &RepositoryFacts,
    issues: &mut Vec<DoctorIssue>,
) {
    let managers = ["npm", "pnpm", "yarn"];
    let mut referenced = BTreeSet::new();
    let mut ids = Vec::new();
    for rule in rules {
        let text = effective_text(rule).to_lowercase();
        for manager in managers {
            if contains_word(&text, manager) {
                referenced.insert(manager);
                ids.push(rule.id.clone());
            }
        }
        if let Some(current) = facts.package_manager.as_deref() {
            for other in managers.into_iter().filter(|manager| *manager != current) {
                if contains_word(&text, other) && is_directive(&text) {
                    issues.push(DoctorIssue {
                        kind: IssueKind::RepositoryContradiction,
                        rule_ids: vec![rule.id.clone()],
                        message: format!(
                            "rule references {other}, but the repository uses {current}"
                        ),
                        recommendation: Some(format!("replace {other} with {current}")),
                    });
                }
            }
        }
    }
    if referenced.len() > 1 {
        ids.sort();
        ids.dedup();
        issues.push(DoctorIssue {
            kind: IssueKind::Conflict,
            rule_ids: ids,
            message: format!(
                "accepted rules reference incompatible package managers: {}",
                referenced.into_iter().collect::<Vec<_>>().join(", ")
            ),
            recommendation: facts
                .package_manager
                .as_ref()
                .map(|manager| format!("keep the rule matching {manager} and review the others")),
        });
    }
}

fn detect_staleness(
    rules: &[ReviewedCandidate],
    now: DateTime<Utc>,
    issues: &mut Vec<DoctorIssue>,
) {
    for rule in rules {
        let expired = rule.decision.valid_until.is_some_and(|date| date < now);
        let old = rule
            .decision
            .last_confirmed_at
            .is_some_and(|date| now.signed_duration_since(date) > Duration::days(180));
        if expired || old {
            issues.push(DoctorIssue {
                kind: IssueKind::Stale,
                rule_ids: vec![rule.id.clone()],
                message: if expired {
                    "rule validity period has expired".to_owned()
                } else {
                    "rule has not been confirmed for more than 180 days".to_owned()
                },
                recommendation: Some(
                    "review the evidence before keeping or replacing this rule".to_owned(),
                ),
            });
        }
    }
}

fn detect_invalid_commands(
    rules: &[ReviewedCandidate],
    facts: &RepositoryFacts,
    issues: &mut Vec<DoctorIssue>,
) {
    for rule in rules {
        let text = effective_text(rule);
        let Some(command) = extract_command(text) else {
            continue;
        };
        if !facts.commands.iter().any(|known| known == &command) {
            issues.push(DoctorIssue {
                kind: IssueKind::InvalidCommand,
                rule_ids: vec![rule.id.clone()],
                message: format!("command `{command}` is not declared by the repository"),
                recommendation: facts
                    .commands
                    .first()
                    .map(|known| format!("verify whether `{known}` is the intended replacement")),
            });
        }
    }
}

fn effective_text(rule: &ReviewedCandidate) -> &str {
    rule.decision
        .edited_text
        .as_deref()
        .unwrap_or(&rule.canonical_text)
}

fn extract_command(text: &str) -> Option<String> {
    if let Some((_, rest)) = text.split_once('`')
        && let Some((command, _)) = rest.split_once('`')
    {
        return is_known_command_family(command).then(|| command.to_owned());
    }
    let lowercase = text.to_lowercase();
    let command = lowercase
        .strip_prefix("run ")?
        .trim_end_matches(['.', '!', ';']);
    is_known_command_family(command).then(|| command.to_owned())
}

fn is_known_command_family(command: &str) -> bool {
    ["npm ", "pnpm ", "yarn ", "cargo "]
        .iter()
        .any(|prefix| command.starts_with(prefix))
}

fn is_directive(text: &str) -> bool {
    ["use ", "prefer ", "run ", "utilise ", "préfère "]
        .iter()
        .any(|marker| text.contains(marker))
}

fn contains_word(text: &str, word: &str) -> bool {
    text.split(|character: char| !character.is_alphanumeric())
        .any(|candidate| candidate == word)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::{RuleScope, Visibility},
        storage::{DecisionStatus, ReviewDecision},
    };

    #[test]
    fn reports_conflicts_repository_drift_and_invalid_commands() {
        let rules = vec![
            reviewed("npm", "Use npm."),
            reviewed("pnpm", "Run `pnpm missing`."),
        ];
        let facts = RepositoryFacts {
            package_manager: Some("pnpm".to_owned()),
            commands: vec!["pnpm test".to_owned()],
            ..RepositoryFacts::default()
        };

        let report = diagnose(&rules, &facts, Utc::now());

        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.kind == IssueKind::Conflict)
        );
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.kind == IssueKind::RepositoryContradiction)
        );
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.kind == IssueKind::InvalidCommand)
        );
    }

    fn reviewed(id: &str, text: &str) -> ReviewedCandidate {
        ReviewedCandidate {
            id: id.to_owned(),
            canonical_text: text.to_owned(),
            occurrences: 1,
            decision: ReviewDecision {
                status: DecisionStatus::Accepted,
                edited_text: None,
                scope: RuleScope::Global,
                visibility: Visibility::Shared,
                last_confirmed_at: Some(Utc::now()),
                last_used_at: None,
                valid_until: None,
            },
        }
    }
}
