use super::CorrectionCandidate;

#[derive(Clone, Debug, PartialEq)]
pub struct ScoreBreakdown {
    pub confidence: f32,
    pub usefulness: f32,
    pub recurrence: f32,
    pub severity: f32,
    pub token_efficiency: f32,
    pub total: f32,
}

#[must_use]
pub fn score_candidate(candidate: &CorrectionCandidate) -> ScoreBreakdown {
    let occurrence_count = u16::try_from(candidate.occurrences.min(10)).unwrap_or(10);
    let recurrence = (f32::from(occurrence_count) / 5.0).clamp(0.0, 1.0);
    let confidence = (0.55 + 0.1 * f32::from(occurrence_count)).clamp(0.0, 0.95);
    let usefulness = if candidate
        .evidence
        .iter()
        .any(|evidence| evidence.preceding_agent_text.is_some())
    {
        0.9
    } else {
        0.65
    };
    let lowercase = candidate.canonical_text.to_lowercase();
    let severity = if ["never", "jamais", "do not", "ne pas", "n'utilise pas"]
        .iter()
        .any(|marker| lowercase.contains(marker))
    {
        1.0
    } else {
        0.7
    };
    let token_count = candidate
        .canonical_text
        .split_whitespace()
        .count()
        .clamp(1, 1_000);
    let estimated_tokens = f32::from(u16::try_from(token_count).unwrap_or(1_000));
    let token_efficiency = (8.0 / estimated_tokens).clamp(0.2, 1.0);
    let total = (confidence * 0.30
        + usefulness * 0.25
        + recurrence * 0.20
        + severity * 0.15
        + token_efficiency * 0.10)
        .clamp(0.0, 1.0);
    ScoreBreakdown {
        confidence,
        usefulness,
        recurrence,
        severity,
        token_efficiency,
        total,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn score_is_bounded_and_rewards_recurrence() {
        let one = CorrectionCandidate {
            canonical_text: "Use pnpm.".to_owned(),
            occurrences: 1,
            evidence: Vec::new(),
        };
        let repeated = CorrectionCandidate {
            canonical_text: "Use pnpm.".to_owned(),
            occurrences: 5,
            evidence: Vec::new(),
        };

        let one = score_candidate(&one);
        let repeated = score_candidate(&repeated);

        assert!((0.0..=1.0).contains(&one.total));
        assert!((0.0..=1.0).contains(&repeated.total));
        assert!(repeated.total > one.total);
    }
}
