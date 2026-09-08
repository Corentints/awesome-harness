use crate::storage::ReviewedCandidate;

#[derive(Clone, Debug)]
pub struct Selection {
    pub selected: Vec<ReviewedCandidate>,
    pub omitted: Vec<ReviewedCandidate>,
    pub tokens_before: usize,
    pub tokens_after: usize,
}

#[must_use]
pub fn select_under_budget(rules: &[ReviewedCandidate], budget: usize) -> Selection {
    let mut ranked = rules.to_vec();
    ranked.sort_by(|left, right| {
        utility(right)
            .total_cmp(&utility(left))
            .then_with(|| left.id.cmp(&right.id))
    });
    let tokens_before = ranked.iter().map(estimated_tokens).sum();
    let mut tokens_after: usize = 0;
    let mut selected = Vec::new();
    let mut omitted = Vec::new();
    for rule in ranked {
        let cost = estimated_tokens(&rule);
        if tokens_after.saturating_add(cost) <= budget {
            tokens_after += cost;
            selected.push(rule);
        } else {
            omitted.push(rule);
        }
    }
    Selection {
        selected,
        omitted,
        tokens_before,
        tokens_after,
    }
}

fn utility(rule: &ReviewedCandidate) -> f64 {
    let occurrences = f64::from(u32::try_from(rule.occurrences.min(10_000)).unwrap_or(10_000));
    occurrences / f64::from(u32::try_from(estimated_tokens(rule)).unwrap_or(u32::MAX))
}

fn estimated_tokens(rule: &ReviewedCandidate) -> usize {
    let text = rule
        .decision
        .edited_text
        .as_deref()
        .unwrap_or(&rule.canonical_text);
    text.split_whitespace().count().max(1) + 2
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::{RuleScope, Visibility},
        storage::{DecisionStatus, ReviewDecision},
    };

    #[test]
    fn keeps_high_recurrence_compact_rules_first() {
        let compact = rule("compact", "Use pnpm.", 10);
        let verbose = rule(
            "verbose",
            "This is a considerably longer instruction with many words.",
            1,
        );

        let selection = select_under_budget(&[verbose, compact], 4);

        assert_eq!(selection.selected[0].id, "compact");
        assert_eq!(selection.omitted.len(), 1);
        assert!(selection.tokens_after <= 4);
    }

    fn rule(id: &str, text: &str, occurrences: usize) -> ReviewedCandidate {
        ReviewedCandidate {
            id: id.to_owned(),
            canonical_text: text.to_owned(),
            occurrences,
            decision: ReviewDecision {
                status: DecisionStatus::Accepted,
                edited_text: None,
                scope: RuleScope::Global,
                visibility: Visibility::Shared,
                last_confirmed_at: None,
                last_used_at: None,
                valid_until: None,
            },
        }
    }
}
