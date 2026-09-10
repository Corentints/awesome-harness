use super::{
    InferenceError, InferenceOutcome, InferenceProvider, InferenceRequest, InferenceUsage,
    output_schema,
};
use serde::Deserialize;
use serde_json::Value;
use std::{
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};

pub struct ClaudeCliProvider {
    executable: PathBuf,
    working_directory: PathBuf,
}

impl ClaudeCliProvider {
    #[must_use]
    pub fn new(working_directory: impl Into<PathBuf>) -> Self {
        Self {
            executable: PathBuf::from("claude"),
            working_directory: working_directory.into(),
        }
    }

    #[must_use]
    pub fn with_executable(mut self, executable: impl Into<PathBuf>) -> Self {
        self.executable = executable.into();
        self
    }
}

impl InferenceProvider for ClaudeCliProvider {
    fn infer(&self, request: &InferenceRequest) -> Result<InferenceOutcome, InferenceError> {
        let schema = serde_json::to_string(&output_schema())?;
        let prompt = super::prompt(request)?;
        let mut child = Command::new(&self.executable)
            .args([
                "--print",
                "--output-format",
                "json",
                "--json-schema",
                &schema,
                "--no-session-persistence",
                "--safe-mode",
                "--tools",
                "",
            ])
            .current_dir(&self.working_directory)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| super::cli_spawn_error("Claude", error))?;
        child
            .stdin
            .as_mut()
            .ok_or_else(|| InferenceError::Provider("Claude stdin unavailable".to_owned()))?
            .write_all(prompt.as_bytes())?;
        let output = child.wait_with_output()?;
        if !output.status.success() {
            return Err(super::classify_cli_failure("Claude", &output.stderr));
        }
        parse_output(&output.stdout)
    }
}

#[derive(Deserialize)]
struct ClaudeOutput {
    #[serde(default)]
    is_error: bool,
    result: Option<String>,
    structured_output: Option<Value>,
    total_cost_usd: Option<f64>,
    duration_ms: Option<u64>,
    num_turns: Option<u64>,
    usage: Option<ClaudeUsage>,
}

#[derive(Deserialize)]
struct ClaudeUsage {
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
}

fn parse_output(bytes: &[u8]) -> Result<InferenceOutcome, InferenceError> {
    if let Ok(response) = serde_json::from_slice(bytes) {
        return Ok(InferenceOutcome {
            response,
            usage: InferenceUsage::default(),
        });
    }
    let output = serde_json::from_slice::<ClaudeOutput>(bytes)?;
    if output.is_error {
        return Err(InferenceError::Provider(
            output
                .result
                .unwrap_or_else(|| "Claude returned an error".to_owned()),
        ));
    }
    let response = if let Some(structured) = output.structured_output {
        serde_json::from_value(structured)?
    } else if let Some(result) = output.result {
        serde_json::from_str(&result)?
    } else {
        return Err(InferenceError::Provider(
            "Claude response contains no structured output".to_owned(),
        ));
    };
    Ok(InferenceOutcome {
        response,
        usage: InferenceUsage {
            input_tokens: output.usage.as_ref().and_then(|usage| usage.input_tokens),
            output_tokens: output.usage.as_ref().and_then(|usage| usage.output_tokens),
            cost_usd: output.total_cost_usd,
            duration_ms: output.duration_ms,
            turns: output.num_turns,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_current_structured_output_envelope() {
        let output = br#"{
            "type":"result",
            "subtype":"success",
            "is_error":false,
            "structured_output":{"rules":[]},
            "result":"",
            "total_cost_usd":0.004,
            "duration_ms":1200,
            "num_turns":1,
            "usage":{"input_tokens":40,"output_tokens":12}
        }"#;

        let outcome = parse_output(output).expect("valid output");
        assert_eq!(outcome.response.rules, []);
        assert_eq!(outcome.usage.input_tokens, Some(40));
        assert_eq!(outcome.usage.output_tokens, Some(12));
        assert_eq!(outcome.usage.cost_usd, Some(0.004));
        assert_eq!(outcome.usage.duration_ms, Some(1200));
        assert_eq!(outcome.usage.turns, Some(1));
    }

    #[test]
    fn accepts_json_encoded_result_for_older_cli_versions() {
        let output = br#"{
            "type":"result",
            "is_error":false,
            "result":"{\"rules\":[]}"
        }"#;

        assert_eq!(
            parse_output(output).expect("valid output").response.rules,
            []
        );
    }

    #[test]
    fn reports_claude_error_envelopes() {
        let output = br#"{"type":"result","is_error":true,"result":"quota exceeded"}"#;

        assert!(
            parse_output(output)
                .expect_err("provider error")
                .to_string()
                .contains("quota exceeded")
        );
    }
}
