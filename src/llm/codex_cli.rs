use super::{
    InferenceError, InferenceProvider, InferenceRequest, InferenceResponse, output_schema,
};
use std::{
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

pub struct CodexCliProvider {
    executable: PathBuf,
    working_directory: PathBuf,
}

impl CodexCliProvider {
    #[must_use]
    pub fn new(working_directory: impl Into<PathBuf>) -> Self {
        Self {
            executable: PathBuf::from("codex"),
            working_directory: working_directory.into(),
        }
    }

    #[must_use]
    pub fn with_executable(mut self, executable: impl Into<PathBuf>) -> Self {
        self.executable = executable.into();
        self
    }
}

impl InferenceProvider for CodexCliProvider {
    fn infer(&self, request: &InferenceRequest) -> Result<InferenceResponse, InferenceError> {
        let directory = tempfile::tempdir()?;
        let schema_path = directory.path().join("schema.json");
        let output_path = directory.path().join("output.json");
        std::fs::write(&schema_path, serde_json::to_vec_pretty(&output_schema())?)?;
        let prompt = super::prompt(request)?;

        let mut child = Command::new(&self.executable)
            .args([
                "exec",
                "--ephemeral",
                "--sandbox",
                "read-only",
                "--skip-git-repo-check",
                "--output-schema",
                path(&schema_path)?,
                "--output-last-message",
                path(&output_path)?,
                "--cd",
                path(&self.working_directory)?,
                "-",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| super::cli_spawn_error("Codex", error))?;
        child
            .stdin
            .as_mut()
            .ok_or_else(|| InferenceError::Provider("Codex stdin unavailable".to_owned()))?
            .write_all(prompt.as_bytes())?;
        let output = child.wait_with_output()?;
        if !output.status.success() {
            return Err(super::classify_cli_failure("Codex", &output.stderr));
        }
        let response = std::fs::read_to_string(output_path)?;
        Ok(serde_json::from_str(&response)?)
    }
}

fn path(path: &Path) -> Result<&str, InferenceError> {
    path.to_str().ok_or_else(|| {
        InferenceError::Provider(format!("path is not valid UTF-8: {}", path.display()))
    })
}
