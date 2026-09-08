use super::IngestError;
use serde_json::Value;
use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};

pub(crate) fn for_each_value(path: &Path, mut visit: impl FnMut(Value)) -> Result<(), IngestError> {
    let file = File::open(path).map_err(|source| IngestError::Io {
        path: path.to_path_buf(),
        source,
    })?;

    for (index, line) in BufReader::new(file).lines().enumerate() {
        let line = line.map_err(|source| IngestError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        if line.trim().is_empty() {
            continue;
        }
        let value = serde_json::from_str(&line).map_err(|source| IngestError::InvalidJson {
            path: path.to_path_buf(),
            line: index + 1,
            source,
        })?;
        visit(value);
    }

    Ok(())
}

pub(crate) fn timestamp(value: &Value) -> Option<chrono::DateTime<chrono::Utc>> {
    value
        .get("timestamp")
        .and_then(Value::as_str)
        .and_then(|raw| chrono::DateTime::parse_from_rfc3339(raw).ok())
        .map(|timestamp| timestamp.with_timezone(&chrono::Utc))
}

pub(crate) fn text_content(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Array(items) => {
            let parts = items
                .iter()
                .filter_map(|item| {
                    item.as_str()
                        .map(str::to_owned)
                        .or_else(|| item.get("text").and_then(Value::as_str).map(str::to_owned))
                })
                .collect::<Vec<_>>();
            (!parts.is_empty()).then(|| parts.join("\n"))
        }
        Value::Object(_) => value.get("text").and_then(Value::as_str).map(str::to_owned),
        _ => None,
    }
}

pub(crate) fn discover_jsonl(root: &Path) -> Result<Vec<super::SessionRef>, IngestError> {
    let mut sessions = Vec::new();
    discover_recursive(root, &mut sessions)?;
    sessions.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(sessions)
}

fn discover_recursive(
    directory: &Path,
    sessions: &mut Vec<super::SessionRef>,
) -> Result<(), IngestError> {
    let entries = std::fs::read_dir(directory).map_err(|source| IngestError::Io {
        path: directory.to_path_buf(),
        source,
    })?;

    for entry in entries {
        let entry = entry.map_err(|source| IngestError::Io {
            path: directory.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        if path.is_dir() {
            discover_recursive(&path, sessions)?;
        } else if path
            .extension()
            .is_some_and(|extension| extension == "jsonl")
        {
            let id = path
                .file_stem()
                .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
            sessions.push(super::SessionRef { id, path });
        }
    }
    Ok(())
}
