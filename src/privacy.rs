use regex::{Captures, Regex};
use std::{collections::BTreeMap, sync::LazyLock};

static SECRET_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?x)
        (?P<bearer>Bearer\s+[A-Za-z0-9._~+/=-]{16,})
        |(?P<known>(?:sk|ghp|github_pat|AKIA)[-_A-Za-z0-9]{16,})
        |(?P<jwt>eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+)
        |(?P<env>(?m)^[A-Z][A-Z0-9_]*(?:TOKEN|SECRET|PASSWORD|KEY)[A-Z0-9_]*\s*=\s*[^\s\#]+)",
    )
    .expect("the static secret pattern is valid")
});
static TOKEN_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[A-Za-z0-9_+/=-]{24,}").expect("the static token pattern is valid")
});

#[derive(Debug, Default)]
pub struct Redactor {
    replacements: BTreeMap<String, String>,
}

impl Redactor {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Redacts known credential formats with stable labels for this redactor.
    #[must_use]
    pub fn redact(&mut self, input: &str) -> String {
        let known_redacted = SECRET_PATTERN
            .replace_all(input, |captures: &Captures<'_>| {
                let Some(secret) = captures.get(0).map(|matched| matched.as_str()) else {
                    return "<REDACTED>".to_owned();
                };
                if let Some(existing) = self.replacements.get(secret) {
                    return existing.clone();
                }
                let replacement = format!("<REDACTED:{}>", self.replacements.len() + 1);
                self.replacements
                    .insert(secret.to_owned(), replacement.clone());
                replacement
            })
            .into_owned();
        TOKEN_PATTERN
            .replace_all(&known_redacted, |captures: &Captures<'_>| {
                let Some(token) = captures.get(0).map(|matched| matched.as_str()) else {
                    return "<REDACTED>".to_owned();
                };
                if !looks_high_entropy(token) {
                    return token.to_owned();
                }
                if let Some(existing) = self.replacements.get(token) {
                    return existing.clone();
                }
                let replacement = format!("<REDACTED:{}>", self.replacements.len() + 1);
                self.replacements
                    .insert(token.to_owned(), replacement.clone());
                replacement
            })
            .into_owned()
    }

    #[must_use]
    pub fn replacement_count(&self) -> usize {
        self.replacements.len()
    }
}

fn looks_high_entropy(token: &str) -> bool {
    let has_lower = token.bytes().any(|byte| byte.is_ascii_lowercase());
    let has_upper = token.bytes().any(|byte| byte.is_ascii_uppercase());
    let has_digit = token.bytes().any(|byte| byte.is_ascii_digit());
    if !(has_lower && has_upper && has_digit) {
        return false;
    }
    let mut counts = [0_u16; 256];
    for byte in token.bytes() {
        counts[usize::from(byte)] += 1;
    }
    let length = f64::from(u32::try_from(token.len()).unwrap_or(u32::MAX));
    counts
        .into_iter()
        .filter(|count| *count > 0)
        .map(|count| {
            let probability = f64::from(count) / length;
            -probability * probability.log2()
        })
        .sum::<f64>()
        >= 3.5
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_known_secrets_with_stable_placeholders() {
        let mut redactor = Redactor::new();
        let input =
            "Authorization: Bearer abcdefghijklmnopqrstuvwxyz\nAPI_SECRET=very-secret-value-123456";
        let first = redactor.redact(input);
        let second = redactor.redact(input);

        assert_eq!(first, second);
        assert!(!first.contains("abcdefghijklmnopqrstuvwxyz"));
        assert!(!first.contains("very-secret-value"));
        assert_eq!(redactor.replacement_count(), 2);
    }

    #[test]
    fn redacts_high_entropy_tokens_but_not_long_words() {
        let mut redactor = Redactor::new();
        let result = redactor.redact("id=A7mQ2vZ9kLp4Rx8Nc3Tw6Yh1 long=abcdefghijklmnopqrstuvwxyz");

        assert!(!result.contains("A7mQ2vZ9kLp4Rx8Nc3Tw6Yh1"));
        assert!(result.contains("abcdefghijklmnopqrstuvwxyz"));
    }
}
