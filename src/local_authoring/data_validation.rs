use std::collections::BTreeSet;

use super::{
    LocalAuthoringError, LocalMarkdownRecord, LocalTomlDocument,
    event_profiles::{event_participants, validate_event_type_value},
    locations::{normalize_place_entity_id, validate_inline_location_value, validate_place_item},
    refs::{normalize_person_id, normalize_source_id, validate_contextual_id},
    sources::validate_source_items,
};

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
            validate_participant_items(record, participants, &ids)?;
        }

        if let Some(places) = record.attributes.get("places") {
            validate_place_items(record, places, &ids)?;
        }

        if let Some(location) = record.attributes.get("location") {
            validate_inline_location_value(&record.path, location, "location")?;
        }

        if let Some(locations) = record.attributes.get("locations") {
            validate_inline_location_value(&record.path, locations, "locations")?;
        }

        if let Some(assertions) = record.attributes.get("assertions") {
            validate_assertion_items(record, assertions, &ids)?;
        }

        if let Some(sources) = record.attributes.get("sources") {
            validate_source_items(&record.path, sources, &ids, "sources")?;
        }

        if record.path.starts_with("events/") && record.kind != "event" {
            return Err(LocalAuthoringError::Validation {
                message: format!("{} event records must use `kind = \"event\"`", record.path),
            });
        }

        if record.kind == "event" {
            let Some(event_type) = record
                .attributes
                .get("type")
                .and_then(serde_json::Value::as_str)
            else {
                return Err(LocalAuthoringError::Validation {
                    message: format!("{} event record missing `type`", record.path),
                });
            };
            validate_event_type_value(event_type, &record.path)?;
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
        validate_contextual_id(person_id, &format!("relationship {field}"))?;
        let person_id = resolve_contextual_id_from_set(person_id, ids, normalize_person_id);
        if !ids.contains(&person_id) {
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
            validate_contextual_id(source_id, "relationship source reference")?;
            let source_id = resolve_contextual_id_from_set(source_id, ids, normalize_source_id);
            if !ids.contains(&source_id) {
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

fn resolve_contextual_id_from_set(
    id: &str,
    ids: &BTreeSet<String>,
    normalize: fn(&str) -> String,
) -> String {
    if ids.contains(id) {
        id.to_string()
    } else {
        normalize(id)
    }
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

    let entity_id = resolve_contextual_id_from_set(entity_id, ids, normalize_person_id);
    if !ids.contains(&entity_id) {
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

fn validate_participant_items(
    record: &LocalMarkdownRecord,
    items: &serde_json::Value,
    ids: &BTreeSet<String>,
) -> Result<(), LocalAuthoringError> {
    let Some(raw_items) = items.as_array() else {
        return Err(LocalAuthoringError::Validation {
            message: format!("{} `participants` must be an array", record.path),
        });
    };

    for raw_item in raw_items {
        match raw_item {
            serde_json::Value::String(entity_id) => {
                validate_contextual_id(entity_id, "event participant")?;
                validate_referenced_id(
                    record,
                    &normalize_person_id(entity_id),
                    ids,
                    "participants",
                )?;
            }
            serde_json::Value::Object(_) => {}
            _ => {
                return Err(LocalAuthoringError::Validation {
                    message: format!(
                        "{} `participants` entries must be entity ids or participant tables",
                        record.path
                    ),
                });
            }
        }
    }

    for item in event_participants(record) {
        let Some(entity_id) = item.get("entity").and_then(serde_json::Value::as_str) else {
            return Err(LocalAuthoringError::Validation {
                message: format!("{} participant item missing `entity`", record.path),
            });
        };
        validate_referenced_id(record, entity_id, ids, "participants")?;
    }

    Ok(())
}

fn validate_referenced_id(
    record: &LocalMarkdownRecord,
    id: &str,
    ids: &BTreeSet<String>,
    field: &str,
) -> Result<(), LocalAuthoringError> {
    if !ids.contains(id) {
        return Err(LocalAuthoringError::Validation {
            message: format!("{} references missing {field} entity `{id}`", record.path),
        });
    }
    Ok(())
}

fn validate_place_items(
    record: &LocalMarkdownRecord,
    items: &serde_json::Value,
    ids: &BTreeSet<String>,
) -> Result<(), LocalAuthoringError> {
    let Some(items) = items.as_array() else {
        return Err(LocalAuthoringError::Validation {
            message: format!("{} `places` must be an array", record.path),
        });
    };

    for item in items {
        match item {
            serde_json::Value::String(entity_id) => {
                validate_contextual_id(entity_id, "event place")?;
                let entity_id = normalize_place_entity_id(entity_id);
                if !ids.contains(&entity_id) {
                    return Err(LocalAuthoringError::Validation {
                        message: format!(
                            "{} references missing places entity `{entity_id}`",
                            record.path
                        ),
                    });
                }
            }
            _ => validate_place_item(&record.path, item, ids)?,
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
        validate_source_items(&record.path, sources, ids, "inline assertion `sources`")?;
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
        let id = normalize_source_id(id);
        if !ids.contains(&id) {
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
