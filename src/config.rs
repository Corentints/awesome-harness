use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};
use thiserror::Error;

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub general: GeneralConfig,
    pub analysis: AnalysisConfig,
    pub llm: LlmConfig,
    pub privacy: PrivacyConfig,
    pub targets: TargetsConfig,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    pub language: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AnalysisConfig {
    pub min_occurrences: usize,
    pub max_context_tokens: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LlmConfig {
    pub provider: String,
    pub max_batch_characters: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PrivacyConfig {
    pub allow_remote_inference: bool,
    pub redact_secrets: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TargetsConfig {
    pub claude: bool,
    pub codex: bool,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            language: "auto".to_owned(),
        }
    }
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self {
            min_occurrences: 2,
            max_context_tokens: 2_500,
        }
    }
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider: "none".to_owned(),
            max_batch_characters: 24_000,
        }
    }
}

impl Default for PrivacyConfig {
    fn default() -> Self {
        Self {
            allow_remote_inference: false,
            redact_secrets: true,
        }
    }
}

impl Default for TargetsConfig {
    fn default() -> Self {
        Self {
            claude: true,
            codex: true,
        }
    }
}

impl Config {
    /// Loads user and project configuration, with project values taking priority.
    ///
    /// # Errors
    ///
    /// Returns an error when an existing configuration cannot be read or parsed.
    pub fn load(user_path: Option<&Path>, project_root: &Path) -> Result<Self, ConfigError> {
        let mut value = toml::Value::try_from(Self::default())
            .map_err(|source| ConfigError::Serialize { source })?;
        if let Some(path) = user_path.filter(|path| path.exists()) {
            merge(&mut value, read_toml(path)?);
        }
        let project_path = project_root.join(".agentctx.toml");
        if project_path.exists() {
            merge(&mut value, read_toml(&project_path)?);
        }
        value.try_into().map_err(|source| ConfigError::Parse {
            path: project_path,
            source,
        })
    }

    /// Writes a documented project configuration unless one already exists.
    ///
    /// # Errors
    ///
    /// Returns an error when the configuration cannot be serialized or written.
    pub fn write_project_default(project_root: &Path) -> Result<PathBuf, ConfigError> {
        let path = project_root.join(".agentctx.toml");
        if path.exists() {
            return Ok(path);
        }
        let content = toml::to_string_pretty(&Self::default())
            .map_err(|source| ConfigError::Serialize { source })?;
        fs::write(&path, content).map_err(|source| ConfigError::Io {
            path: path.clone(),
            source,
        })?;
        Ok(path)
    }
}

#[must_use]
pub fn default_user_config_path() -> Option<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .map(|root| root.join("agentctx/config.toml"))
}

fn read_toml(path: &Path) -> Result<toml::Value, ConfigError> {
    let content = fs::read_to_string(path).map_err(|source| ConfigError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    toml::from_str(&content).map_err(|source| ConfigError::Parse {
        path: path.to_path_buf(),
        source,
    })
}

fn merge(base: &mut toml::Value, overlay: toml::Value) {
    match (base, overlay) {
        (toml::Value::Table(base), toml::Value::Table(overlay)) => {
            for (key, value) in overlay {
                match base.get_mut(&key) {
                    Some(current) => merge(current, value),
                    None => {
                        base.insert(key, value);
                    }
                }
            }
        }
        (base, overlay) => *base = overlay,
    }
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("could not read or write {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid configuration {path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },
    #[error("could not serialize default configuration: {source}")]
    Serialize {
        #[source]
        source: toml::ser::Error,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_values_override_user_values_without_losing_privacy_defaults() {
        let directory = tempfile::tempdir().expect("temp directory");
        let user = directory.path().join("user.toml");
        fs::write(&user, "[analysis]\nmin_occurrences = 4\n").expect("user config");
        fs::write(
            directory.path().join(".agentctx.toml"),
            "[analysis]\nmax_context_tokens = 900\n",
        )
        .expect("project config");

        let config = Config::load(Some(&user), directory.path()).expect("merged config");

        assert_eq!(config.analysis.min_occurrences, 4);
        assert_eq!(config.analysis.max_context_tokens, 900);
        assert!(!config.privacy.allow_remote_inference);
        assert!(config.privacy.redact_secrets);
        assert_eq!(config.llm.max_batch_characters, 24_000);
    }
}
