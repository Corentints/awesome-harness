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
        SECRET_PATTERN
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
            .into_owned()
    }

    #[must_use]
    pub fn replacement_count(&self) -> usize {
        self.replacements.len()
    }
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
}
