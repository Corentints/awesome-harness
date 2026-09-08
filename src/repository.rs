use crate::project::Project;
use ignore::WalkBuilder;
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use thiserror::Error;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RepositoryFacts {
    pub tracked_files: Vec<PathBuf>,
    pub package_manager: Option<String>,
    pub test_tools: Vec<String>,
    pub commands: Vec<String>,
    pub generated_paths: Vec<String>,
    pub instructions: Vec<PathBuf>,
    pub instruction_contents: Vec<ExistingInstruction>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExistingInstruction {
    pub path: PathBuf,
    pub content: String,
}

/// Collects deterministic facts from a Git project without invoking an LLM.
///
/// # Errors
///
/// Returns an error when a relevant repository file cannot be read or parsed.
pub fn scan(project: &Project) -> Result<RepositoryFacts, RepositoryError> {
    let tracked_files = tracked_files(project);
    let mut facts = RepositoryFacts {
        tracked_files,
        ..RepositoryFacts::default()
    };

    facts.instructions = facts
        .tracked_files
        .iter()
        .filter(|path| is_instruction(path))
        .cloned()
        .collect();
    facts.instruction_contents = facts
        .instructions
        .iter()
        .map(|relative_path| {
            let path = project.root.join(relative_path);
            fs::read_to_string(&path)
                .map(|content| ExistingInstruction {
                    path: relative_path.clone(),
                    content,
                })
                .map_err(|source| RepositoryError::Io { path, source })
        })
        .collect::<Result<Vec<_>, _>>()?;
    inspect_package_json(project, &mut facts)?;
    let tracked_files = facts.tracked_files.clone();
    detect_from_paths(&tracked_files, &mut facts);
    inspect_gitignore(project, &mut facts)?;
    facts.test_tools.sort();
    facts.test_tools.dedup();
    facts.commands.sort();
    facts.commands.dedup();
    Ok(facts)
}

fn tracked_files(project: &Project) -> Vec<PathBuf> {
    let output = Command::new("git")
        .args([
            "-C",
            project.root.to_string_lossy().as_ref(),
            "ls-files",
            "-z",
        ])
        .output();
    if let Ok(output) = output
        && output.status.success()
    {
        let mut files = output
            .stdout
            .split(|byte| *byte == 0)
            .filter(|path| !path.is_empty())
            .map(|path| PathBuf::from(String::from_utf8_lossy(path).into_owned()))
            .collect::<Vec<_>>();
        files.sort();
        return files;
    }

    let mut files = WalkBuilder::new(&project.root)
        .hidden(false)
        .git_ignore(true)
        .git_exclude(true)
        .build()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_some_and(|kind| kind.is_file()))
        .filter_map(|entry| {
            entry
                .path()
                .strip_prefix(&project.root)
                .ok()
                .map(Path::to_path_buf)
        })
        .collect::<Vec<_>>();
    files.sort();
    files
}

fn inspect_package_json(
    project: &Project,
    facts: &mut RepositoryFacts,
) -> Result<(), RepositoryError> {
    let path = project.root.join("package.json");
    if !path.exists() {
        return Ok(());
    }
    let content = fs::read_to_string(&path).map_err(|source| RepositoryError::Io {
        path: path.clone(),
        source,
    })?;
    let package: Value =
        serde_json::from_str(&content).map_err(|source| RepositoryError::Json {
            path: path.clone(),
            source,
        })?;
    facts.package_manager = package
        .get("packageManager")
        .and_then(Value::as_str)
        .and_then(|value| value.split('@').next())
        .map(str::to_owned);
    if let Some(scripts) = package.get("scripts").and_then(Value::as_object) {
        facts.commands.extend(
            scripts
                .keys()
                .map(|name| match facts.package_manager.as_deref() {
                    Some("pnpm") => format!("pnpm {name}"),
                    Some("yarn") => format!("yarn {name}"),
                    _ => format!("npm run {name}"),
                }),
        );
    }
    for section in ["dependencies", "devDependencies"] {
        if let Some(dependencies) = package.get(section).and_then(Value::as_object) {
            for tool in ["vitest", "jest", "@playwright/test"] {
                if dependencies.contains_key(tool) {
                    facts.test_tools.push(tool.to_owned());
                }
            }
        }
    }
    Ok(())
}

fn detect_from_paths(paths: &[PathBuf], facts: &mut RepositoryFacts) {
    if facts.package_manager.is_none() {
        facts.package_manager = if paths.iter().any(|path| path == Path::new("pnpm-lock.yaml")) {
            Some("pnpm".to_owned())
        } else if paths.iter().any(|path| path == Path::new("yarn.lock")) {
            Some("yarn".to_owned())
        } else if paths
            .iter()
            .any(|path| path == Path::new("package-lock.json"))
        {
            Some("npm".to_owned())
        } else {
            None
        };
    }
    for (path, tool) in [
        ("vitest.config.ts", "vitest"),
        ("jest.config.js", "jest"),
        ("playwright.config.ts", "@playwright/test"),
    ] {
        if paths.iter().any(|candidate| candidate == Path::new(path)) {
            facts.test_tools.push(tool.to_owned());
        }
    }
}

fn inspect_gitignore(
    project: &Project,
    facts: &mut RepositoryFacts,
) -> Result<(), RepositoryError> {
    let path = project.root.join(".gitignore");
    if !path.exists() {
        return Ok(());
    }
    let content = fs::read_to_string(&path).map_err(|source| RepositoryError::Io {
        path: path.clone(),
        source,
    })?;
    facts.generated_paths = content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#') && !line.starts_with('!'))
        .map(str::to_owned)
        .collect();
    Ok(())
}

fn is_instruction(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|name| name.to_str()),
        Some("AGENTS.md" | "CLAUDE.md" | "CLAUDE.local.md")
    ) || path.starts_with(".claude")
}

#[derive(Debug, Error)]
pub enum RepositoryError {
    #[error("could not read {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid JSON in {path}: {source}")]
    Json {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifies_instruction_files() {
        assert!(is_instruction(Path::new("AGENTS.md")));
        assert!(is_instruction(Path::new("backend/CLAUDE.md")));
        assert!(is_instruction(Path::new(".claude/rules/testing.md")));
        assert!(!is_instruction(Path::new("README.md")));
    }
}
