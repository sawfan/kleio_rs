use std::collections::BTreeSet;

use super::{LocalAuthoringError, refs::normalize_source_id};

pub(super) fn source_items(value: Option<&serde_json::Value>) -> Vec<serde_json::Value> {
    value
        .and_then(serde_json::Value::as_array)
        .map(|values| values.iter().filter_map(source_item).collect())
        .unwrap_or_default()
}

pub(super) fn validate_source_items(
    path: &str,
    values: &serde_json::Value,
    ids: &BTreeSet<String>,
    field: &str,
) -> Result<(), LocalAuthoringError> {
    let Some(values) = values.as_array() else {
        return Err(LocalAuthoringError::Validation {
            message: format!("{path} `{field}` must be an array"),
        });
    };

    for value in values {
        match value {
            serde_json::Value::String(id) => {
                let id = normalize_source_id(id);
                if !ids.contains(&id) {
                    return Err(LocalAuthoringError::Validation {
                        message: format!("{path} references missing {field} id `{id}`"),
                    });
                }
            }
            serde_json::Value::Object(source) => validate_inline_source(path, source, field)?,
            _ => {
                return Err(LocalAuthoringError::Validation {
                    message: format!(
                        "{path} `{field}` entries must be source ids or inline source tables"
                    ),
                });
            }
        }
    }

    Ok(())
}

fn source_item(value: &serde_json::Value) -> Option<serde_json::Value> {
    match value {
        serde_json::Value::String(id) if !id.trim().is_empty() => {
            Some(serde_json::Value::String(normalize_source_id(id)))
        }
        serde_json::Value::Object(_) => Some(value.clone()),
        _ => None,
    }
}

fn validate_inline_source(
    path: &str,
    source: &serde_json::Map<String, serde_json::Value>,
    field: &str,
) -> Result<(), LocalAuthoringError> {
    let has_identity = [
        "label", "title", "file", "path", "uri", "url", "hash", "sha256",
    ]
    .into_iter()
    .any(|key| {
        source
            .get(key)
            .and_then(serde_json::Value::as_str)
            .is_some_and(|value| !value.trim().is_empty())
    });

    if !has_identity {
        return Err(LocalAuthoringError::Validation {
            message: format!(
                "{path} inline `{field}` source must include `label`, `title`, `file`, `uri`, `hash`, or `sha256`"
            ),
        });
    }

    Ok(())
}
