use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path, PathBuf},
};
use thiserror::Error;

pub const EVALUATION_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HistoricalCase {
    pub schema_version: u32,
    pub id: String,
    pub description: String,
    pub prompt: String,
    #[serde(default)]
    pub fixture_files: Vec<FixtureFile>,
    pub oracle: CaseOracle,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixtureFile {
    pub path: PathBuf,
    pub content: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CaseOracle {
    pub require_success: bool,
    pub required_commands: Vec<String>,
    pub forbidden_commands: Vec<String>,
    pub required_paths: Vec<PathBuf>,
    pub forbidden_paths: Vec<PathBuf>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CaseObservation {
    pub success: bool,
    pub commands: Vec<String>,
    pub modified_paths: Vec<PathBuf>,
    pub corrections: usize,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub turns: Option<u64>,
    pub duration_ms: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CaseScore {
    pub passed: bool,
    pub violations: Vec<String>,
    pub corrections: usize,
    pub total_tokens: Option<u64>,
    pub turns: Option<u64>,
    pub duration_ms: Option<u64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BenchmarkVariant {
    NoContext,
    CurrentContext,
    CompiledContext,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationRun {
    pub schema_version: u32,
    pub variant: BenchmarkVariant,
    pub cases: BTreeMap<String, CaseObservation>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BenchmarkReport {
    pub variant: BenchmarkVariant,
    pub cases: usize,
    pub passed: usize,
    pub violations: usize,
    pub corrections: usize,
    pub total_tokens: Option<u64>,
    pub turns: Option<u64>,
    pub duration_ms: Option<u64>,
}

/// Loads and validates every JSON case directly contained in a corpus.
///
/// # Errors
///
/// Returns a localized error for unreadable, invalid or unsafe cases.
pub fn load_corpus(root: &Path) -> Result<Vec<HistoricalCase>, EvaluationError> {
    let entries = fs::read_dir(root).map_err(|source| EvaluationError::Io {
        path: root.to_path_buf(),
        source,
    })?;
    let mut paths = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .collect::<Vec<_>>();
    paths.sort();
    let mut cases = Vec::new();
    for path in paths {
        let content = fs::read_to_string(&path).map_err(|source| EvaluationError::Io {
            path: path.clone(),
            source,
        })?;
        let case = serde_json::from_str(&content).map_err(|source| EvaluationError::Json {
            path: path.clone(),
            source,
        })?;
        validate_case(&case).map_err(|message| EvaluationError::Invalid {
            path: path.clone(),
            message,
        })?;
        cases.push(case);
    }
    Ok(cases)
}

/// Loads a versioned set of observations produced by one benchmark variant.
///
/// # Errors
///
/// Returns an error when the file is unreadable or does not match the schema.
pub fn load_observation_run(path: &Path) -> Result<ObservationRun, EvaluationError> {
    let content = fs::read_to_string(path).map_err(|source| EvaluationError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let run = serde_json::from_str::<ObservationRun>(&content).map_err(|source| {
        EvaluationError::Json {
            path: path.to_path_buf(),
            source,
        }
    })?;
    if run.schema_version != EVALUATION_SCHEMA_VERSION {
        return Err(EvaluationError::Invalid {
            path: path.to_path_buf(),
            message: format!(
                "unsupported schema version {}, expected {EVALUATION_SCHEMA_VERSION}",
                run.schema_version
            ),
        });
    }
    Ok(run)
}

/// Validates case coverage and aggregates observable metrics for one variant.
///
/// # Errors
///
/// Returns an error when observations are missing or refer to unknown cases.
pub fn score_run(
    cases: &[HistoricalCase],
    run: &ObservationRun,
) -> Result<BenchmarkReport, EvaluationError> {
    let case_ids = cases
        .iter()
        .map(|case| case.id.as_str())
        .collect::<BTreeSet<_>>();
    let observation_ids = run
        .cases
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if case_ids != observation_ids {
        let missing = case_ids
            .difference(&observation_ids)
            .map(|id| (*id).to_owned())
            .collect::<Vec<_>>();
        let unknown = observation_ids
            .difference(&case_ids)
            .map(|id| (*id).to_owned())
            .collect::<Vec<_>>();
        return Err(EvaluationError::RunMismatch { missing, unknown });
    }

    let scores = cases
        .iter()
        .map(|case| score_case(case, &run.cases[&case.id]))
        .collect::<Vec<_>>();
    Ok(BenchmarkReport {
        variant: run.variant,
        cases: scores.len(),
        passed: scores.iter().filter(|score| score.passed).count(),
        violations: scores.iter().map(|score| score.violations.len()).sum(),
        corrections: scores.iter().map(|score| score.corrections).sum(),
        total_tokens: sum_complete(scores.iter().map(|score| score.total_tokens)),
        turns: sum_complete(scores.iter().map(|score| score.turns)),
        duration_ms: sum_complete(scores.iter().map(|score| score.duration_ms)),
    })
}

fn sum_complete(mut values: impl Iterator<Item = Option<u64>>) -> Option<u64> {
    values.try_fold(0_u64, |total, value| {
        value.map(|value| total.saturating_add(value))
    })
}

/// Scores only observable behavior. The oracle is never part of the task
/// prompt passed to an agent.
#[must_use]
pub fn score_case(case: &HistoricalCase, observation: &CaseObservation) -> CaseScore {
    let mut violations = Vec::new();
    if case.oracle.require_success && !observation.success {
        violations.push("task did not succeed".to_owned());
    }
    for command in &case.oracle.required_commands {
        if !observation.commands.iter().any(|actual| actual == command) {
            violations.push(format!("required command was not run: `{command}`"));
        }
    }
    for command in &case.oracle.forbidden_commands {
        if observation.commands.iter().any(|actual| actual == command) {
            violations.push(format!("forbidden command was run: `{command}`"));
        }
    }
    for path in &case.oracle.required_paths {
        if !observation
            .modified_paths
            .iter()
            .any(|actual| actual == path)
        {
            violations.push(format!(
                "required path was not modified: `{}`",
                path.display()
            ));
        }
    }
    for path in &case.oracle.forbidden_paths {
        if observation
            .modified_paths
            .iter()
            .any(|actual| actual == path)
        {
            violations.push(format!("forbidden path was modified: `{}`", path.display()));
        }
    }
    CaseScore {
        passed: violations.is_empty(),
        violations,
        corrections: observation.corrections,
        total_tokens: observation
            .input_tokens
            .zip(observation.output_tokens)
            .map(|(input, output)| input.saturating_add(output)),
        turns: observation.turns,
        duration_ms: observation.duration_ms,
    }
}

fn validate_case(case: &HistoricalCase) -> Result<(), String> {
    if case.schema_version != EVALUATION_SCHEMA_VERSION {
        return Err(format!(
            "unsupported schema version {}, expected {EVALUATION_SCHEMA_VERSION}",
            case.schema_version
        ));
    }
    if case.id.trim().is_empty() || case.prompt.trim().is_empty() {
        return Err("id and prompt must not be empty".to_owned());
    }
    let paths = case
        .fixture_files
        .iter()
        .map(|file| file.path.as_path())
        .chain(case.oracle.required_paths.iter().map(PathBuf::as_path))
        .chain(case.oracle.forbidden_paths.iter().map(PathBuf::as_path));
    if let Some(path) = paths.into_iter().find(|path| !is_safe_relative(path)) {
        return Err(format!(
            "path must stay relative to the fixture: {}",
            path.display()
        ));
    }
    Ok(())
}

fn is_safe_relative(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_) | Component::CurDir))
}

#[derive(Debug, Error)]
pub enum EvaluationError {
    #[error("could not read evaluation corpus at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid evaluation JSON at {path}: {source}")]
    Json {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("invalid evaluation case at {path}: {message}")]
    Invalid { path: PathBuf, message: String },
    #[error(
        "observation cases do not match the corpus (missing: {missing:?}, unknown: {unknown:?})"
    )]
    RunMismatch {
        missing: Vec<String>,
        unknown: Vec<String>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_the_versioned_fixture_corpus() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/evaluation");

        let cases = load_corpus(&root).expect("valid corpus");

        assert_eq!(cases.len(), 2);
        assert_eq!(cases[0].id, "generated-files");
        assert_eq!(cases[1].id, "package-manager");
    }

    #[test]
    fn scores_commands_paths_and_resource_metrics() {
        let case = HistoricalCase {
            schema_version: EVALUATION_SCHEMA_VERSION,
            id: "example".to_owned(),
            description: "Example".to_owned(),
            prompt: "Implement the change.".to_owned(),
            fixture_files: Vec::new(),
            oracle: CaseOracle {
                require_success: true,
                required_commands: vec!["pnpm test".to_owned()],
                forbidden_paths: vec!["src/generated/client.ts".into()],
                ..CaseOracle::default()
            },
        };
        let observation = CaseObservation {
            success: true,
            commands: vec!["npm test".to_owned()],
            modified_paths: vec!["src/generated/client.ts".into()],
            corrections: 2,
            input_tokens: Some(100),
            output_tokens: Some(40),
            turns: Some(3),
            duration_ms: Some(900),
        };

        let score = score_case(&case, &observation);

        assert!(!score.passed);
        assert_eq!(score.violations.len(), 2);
        assert_eq!(score.total_tokens, Some(140));
        assert_eq!(score.corrections, 2);
    }

    #[test]
    fn rejects_paths_that_escape_the_fixture() {
        let case = HistoricalCase {
            schema_version: EVALUATION_SCHEMA_VERSION,
            id: "unsafe".to_owned(),
            description: "Unsafe".to_owned(),
            prompt: "Do work.".to_owned(),
            fixture_files: vec![FixtureFile {
                path: "../outside".into(),
                content: String::new(),
            }],
            oracle: CaseOracle::default(),
        };

        assert!(
            validate_case(&case)
                .expect_err("unsafe path")
                .contains("relative")
        );
    }

    #[test]
    fn aggregates_a_complete_observation_run() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/evaluation");
        let cases = load_corpus(&root).expect("valid corpus");
        let run = ObservationRun {
            schema_version: EVALUATION_SCHEMA_VERSION,
            variant: BenchmarkVariant::CompiledContext,
            cases: BTreeMap::from([
                (
                    "generated-files".to_owned(),
                    CaseObservation {
                        success: true,
                        modified_paths: vec!["src/client.ts".into()],
                        input_tokens: Some(100),
                        output_tokens: Some(20),
                        turns: Some(2),
                        duration_ms: Some(500),
                        ..CaseObservation::default()
                    },
                ),
                (
                    "package-manager".to_owned(),
                    CaseObservation {
                        success: true,
                        commands: vec!["pnpm test".to_owned()],
                        input_tokens: Some(80),
                        output_tokens: Some(20),
                        turns: Some(1),
                        duration_ms: Some(300),
                        ..CaseObservation::default()
                    },
                ),
            ]),
        };

        let report = score_run(&cases, &run).expect("complete run");

        assert_eq!(report.passed, 2);
        assert_eq!(report.total_tokens, Some(220));
        assert_eq!(report.turns, Some(3));
        assert_eq!(report.duration_ms, Some(800));
    }
}
