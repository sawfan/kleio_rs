use std::collections::BTreeSet;

use super::{LocalAuthoringError, LocalMarkdownRecord, LocalTomlDocument};

pub(super) fn validate_local_data(
    markdown_records: &[LocalMarkdownRecord],
    toml_documents: &[LocalTomlDocument],
) -> Result<(), LocalAuthoringError> {
    let mut ids = BTreeSet::new();

    for record in markdown_records {
        validate_id(&record.id, &record.path)?;
        if !ids.insert(record.id.clone()) {
            return Err(LocalAuthoringError::Validation {
                message: format!("duplicate id `{}`", record.id),
            });
        }
    }

    for document in toml_documents {
        if let Some(id) = &document.id {
            validate_id(id, &document.path)?;
            if !ids.insert(id.clone()) {
                return Err(LocalAuthoringError::Validation {
                    message: format!("duplicate id `{id}`"),
                });
            }
        }
    }

    for record in markdown_records {
        for related_id in &record.related {
            if !ids.contains(related_id) {
                return Err(LocalAuthoringError::Validation {
                    message: format!(
                        "{} references missing related id `{related_id}`",
                        record.path
                    ),
                });
            }
        }

        if let Some(place_id) = &record.place
            && !ids.contains(place_id)
        {
            return Err(LocalAuthoringError::Validation {
                message: format!("{} references missing place `{place_id}`", record.path),
            });
        }

        if let Some(participants) = record.attributes.get("participants") {
            validate_entity_reference_items(record, participants, &ids, "participants")?;
        }

        if let Some(places) = record.attributes.get("places") {
            validate_entity_reference_items(record, places, &ids, "places")?;
        }

        if let Some(assertions) = record.attributes.get("assertions") {
            validate_assertion_items(record, assertions, &ids)?;
        }

        if let Some(sources) = record.attributes.get("sources") {
            validate_id_references(record, sources, &ids, "sources")?;
        }

        if record.path.starts_with("assertions/") {
            validate_assertion_record(record, &ids)?;
        }
    }

    for document in toml_documents {
        match document.kind.as_deref() {
            Some("relationship") => validate_relationship_document(document, &ids)?,
            Some("event-collection") => validate_event_collection_document(document, &ids)?,
            Some("timeline-view") => validate_optional_view_entity_reference(
                document,
                &["subject", "entity"],
                &ids,
                "timeline subject",
            )?,
            Some("tree-view") => validate_optional_view_entity_reference(
                document,
                &["root", "entity"],
                &ids,
                "tree root",
            )?,
            _ => {}
        }
    }

    Ok(())
}

fn validate_relationship_document(
    document: &LocalTomlDocument,
    ids: &BTreeSet<String>,
) -> Result<(), LocalAuthoringError> {
    let Some(source) = document
        .data
        .get("source")
        .and_then(serde_json::Value::as_str)
    else {
        return Err(LocalAuthoringError::Validation {
            message: format!("{} relationship missing `source`", document.path),
        });
    };
    let Some(target) = document
        .data
        .get("target")
        .and_then(serde_json::Value::as_str)
    else {
        return Err(LocalAuthoringError::Validation {
            message: format!("{} relationship missing `target`", document.path),
        });
    };

    for (field, person_id) in [("source", source), ("target", target)] {
        if !ids.contains(person_id) {
            return Err(LocalAuthoringError::Validation {
                message: format!(
                    "{} references missing relationship {field} `{person_id}`",
                    document.path
                ),
            });
        }
    }

    if let Some(sources) = document.data.get("sources") {
        let Some(sources) = sources.as_array() else {
            return Err(LocalAuthoringError::Validation {
                message: format!("{} `sources` must be an array", document.path),
            });
        };
        for source_id in sources {
            let Some(source_id) = source_id.as_str() else {
                return Err(LocalAuthoringError::Validation {
                    message: format!("{} `sources` must contain only strings", document.path),
                });
            };
            if !ids.contains(source_id) {
                return Err(LocalAuthoringError::Validation {
                    message: format!(
                        "{} references missing relationship source `{source_id}`",
                        document.path
                    ),
                });
            }
        }
    }

    Ok(())
}

fn validate_event_collection_document(
    document: &LocalTomlDocument,
    ids: &BTreeSet<String>,
) -> Result<(), LocalAuthoringError> {
    let collection_kind = document
        .data
        .get("collection_kind")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("set");
    if !matches!(collection_kind, "set" | "sequence") {
        return Err(LocalAuthoringError::Validation {
            message: format!(
                "{} event collection has invalid collection_kind `{collection_kind}`",
                document.path
            ),
        });
    }

    let Some(members) = document.data.get("members") else {
        return Ok(());
    };
    let Some(members) = members.as_array() else {
        return Err(LocalAuthoringError::Validation {
            message: format!("{} `members` must be an array", document.path),
        });
    };

    for member in members {
        let Some(event_id) = member.get("event").and_then(serde_json::Value::as_str) else {
            return Err(LocalAuthoringError::Validation {
                message: format!("{} collection member missing `event`", document.path),
            });
        };
        if !ids.contains(event_id) {
            return Err(LocalAuthoringError::Validation {
                message: format!(
                    "{} references missing collection event `{event_id}`",
                    document.path
                ),
            });
        }
    }

    Ok(())
}

fn validate_optional_view_entity_reference(
    document: &LocalTomlDocument,
    path: &[&str],
    ids: &BTreeSet<String>,
    label: &str,
) -> Result<(), LocalAuthoringError> {
    let Some(entity_id) = nested_string(&document.data, path) else {
        return Ok(());
    };

    if !ids.contains(entity_id) {
        return Err(LocalAuthoringError::Validation {
            message: format!(
                "{} references missing {label} entity `{entity_id}`",
                document.path
            ),
        });
    }

    Ok(())
}

fn nested_string<'a>(value: &'a serde_json::Value, path: &[&str]) -> Option<&'a str> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_str()
}

fn validate_entity_reference_items(
    record: &LocalMarkdownRecord,
    items: &serde_json::Value,
    ids: &BTreeSet<String>,
    field: &str,
) -> Result<(), LocalAuthoringError> {
    let Some(items) = items.as_array() else {
        return Err(LocalAuthoringError::Validation {
            message: format!("{} `{field}` must be an array", record.path),
        });
    };

    for item in items {
        let Some(entity_id) = item.get("entity").and_then(serde_json::Value::as_str) else {
            return Err(LocalAuthoringError::Validation {
                message: format!("{} {field} item missing `entity`", record.path),
            });
        };
        if !ids.contains(entity_id) {
            return Err(LocalAuthoringError::Validation {
                message: format!(
                    "{} references missing {field} entity `{entity_id}`",
                    record.path
                ),
            });
        }
    }

    Ok(())
}

fn validate_assertion_items(
    record: &LocalMarkdownRecord,
    values: &serde_json::Value,
    ids: &BTreeSet<String>,
) -> Result<(), LocalAuthoringError> {
    let Some(values) = values.as_array() else {
        return Err(LocalAuthoringError::Validation {
            message: format!("{} `assertions` must be an array", record.path),
        });
    };

    for value in values {
        if let Some(id) = value.as_str() {
            if !ids.contains(id) {
                return Err(LocalAuthoringError::Validation {
                    message: format!("{} references missing assertions id `{id}`", record.path),
                });
            }
            continue;
        }

        let Some(assertion) = value.as_object() else {
            return Err(LocalAuthoringError::Validation {
                message: format!(
                    "{} `assertions` entries must be assertion ids or inline assertion tables",
                    record.path
                ),
            });
        };
        validate_inline_assertion(record, assertion, ids)?;
    }

    Ok(())
}

fn validate_inline_assertion(
    record: &LocalMarkdownRecord,
    assertion: &serde_json::Map<String, serde_json::Value>,
    ids: &BTreeSet<String>,
) -> Result<(), LocalAuthoringError> {
    let Some(target) = assertion.get("target").and_then(serde_json::Value::as_str) else {
        return Err(LocalAuthoringError::Validation {
            message: format!("{} inline assertion missing `target`", record.path),
        });
    };
    validate_assertion_target(record, target, ids)?;

    if let Some(sources) = assertion.get("sources") {
        validate_inline_sources(record, sources, ids)?;
    }

    Ok(())
}

fn validate_inline_sources(
    record: &LocalMarkdownRecord,
    values: &serde_json::Value,
    ids: &BTreeSet<String>,
) -> Result<(), LocalAuthoringError> {
    let Some(values) = values.as_array() else {
        return Err(LocalAuthoringError::Validation {
            message: format!(
                "{} inline assertion `sources` must be an array",
                record.path
            ),
        });
    };

    for value in values {
        let Some(id) = value.as_str() else {
            return Err(LocalAuthoringError::Validation {
                message: format!(
                    "{} inline assertion `sources` must contain only strings",
                    record.path
                ),
            });
        };
        if !ids.contains(id) {
            return Err(LocalAuthoringError::Validation {
                message: format!("{} references missing sources id `{id}`", record.path),
            });
        }
    }

    Ok(())
}

fn validate_assertion_target(
    record: &LocalMarkdownRecord,
    target: &str,
    ids: &BTreeSet<String>,
) -> Result<(), LocalAuthoringError> {
    let target_base = if target.starts_with('#') {
        record.id.as_str()
    } else {
        target_base_id(target)
    };

    if !ids.contains(target_base) {
        return Err(LocalAuthoringError::Validation {
            message: format!(
                "{} references missing assertion target `{target_base}`",
                record.path
            ),
        });
    }

    Ok(())
}

fn validate_id_references(
    record: &LocalMarkdownRecord,
    values: &serde_json::Value,
    ids: &BTreeSet<String>,
    field: &str,
) -> Result<(), LocalAuthoringError> {
    let Some(values) = values.as_array() else {
        return Err(LocalAuthoringError::Validation {
            message: format!("{} `{field}` must be an array", record.path),
        });
    };

    for value in values {
        let Some(id) = value.as_str() else {
            return Err(LocalAuthoringError::Validation {
                message: format!("{} `{field}` must contain only strings", record.path),
            });
        };
        if !ids.contains(id) {
            return Err(LocalAuthoringError::Validation {
                message: format!("{} references missing {field} id `{id}`", record.path),
            });
        }
    }

    Ok(())
}

fn validate_assertion_record(
    record: &LocalMarkdownRecord,
    ids: &BTreeSet<String>,
) -> Result<(), LocalAuthoringError> {
    let target = record
        .attributes
        .get("target")
        .and_then(serde_json::Value::as_str);
    let Some(target) = target else {
        return Err(LocalAuthoringError::Validation {
            message: format!("{} assertion missing `target`", record.path),
        });
    };
    validate_assertion_target(record, target, ids)?;

    if let Some(sources) = record.attributes.get("sources") {
        validate_id_references(record, sources, ids, "sources")?;
    }

    if assertion_requires_value(record) && !record.attributes.contains_key("value") {
        return Err(LocalAuthoringError::Validation {
            message: format!("{} assertion missing `value`", record.path),
        });
    }

    Ok(())
}

fn assertion_requires_value(record: &LocalMarkdownRecord) -> bool {
    !record.attributes.contains_key("target")
}

fn target_base_id(target: &str) -> &str {
    target
        .split_once('#')
        .map(|(base, _)| base)
        .unwrap_or(target)
}

fn validate_id(id: &str, path: &str) -> Result<(), LocalAuthoringError> {
    if id.trim().is_empty() {
        return Err(LocalAuthoringError::Validation {
            message: format!("{path} has an empty id"),
        });
    }

    if id.chars().any(char::is_whitespace) {
        return Err(LocalAuthoringError::Validation {
            message: format!("{path} id `{id}` contains whitespace"),
        });
    }

    Ok(())
}
