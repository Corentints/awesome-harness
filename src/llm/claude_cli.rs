use super::{
    InferenceError, InferenceProvider, InferenceRequest, InferenceResponse, output_schema,
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
    fn infer(&self, request: &InferenceRequest) -> Result<InferenceResponse, InferenceError> {
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
}

fn parse_output(bytes: &[u8]) -> Result<InferenceResponse, InferenceError> {
    if let Ok(response) = serde_json::from_slice(bytes) {
        return Ok(response);
    }
    let output = serde_json::from_slice::<ClaudeOutput>(bytes)?;
    if output.is_error {
        return Err(InferenceError::Provider(
            output
                .result
                .unwrap_or_else(|| "Claude returned an error".to_owned()),
        ));
    }
    if let Some(structured) = output.structured_output {
        return Ok(serde_json::from_value(structured)?);
    }
    if let Some(result) = output.result {
        return Ok(serde_json::from_str(&result)?);
    }
    Err(InferenceError::Provider(
        "Claude response contains no structured output".to_owned(),
    ))
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
            "result":""
        }"#;

        assert_eq!(parse_output(output).expect("valid output").rules, []);
    }

    #[test]
    fn accepts_json_encoded_result_for_older_cli_versions() {
        let output = br#"{
            "type":"result",
            "is_error":false,
            "result":"{\"rules\":[]}"
        }"#;

        assert_eq!(parse_output(output).expect("valid output").rules, []);
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
