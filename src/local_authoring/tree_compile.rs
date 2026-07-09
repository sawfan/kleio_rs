use std::collections::BTreeMap;

use crate::{
    Attribute, DateValue, Event, EventId, EventKind, Name, Person, PersonId, Provenance,
    RelationshipKind, Sex, SourceRef, Tag, TreeDocument,
};

use super::{LocalAuthoringError, LocalDataBundle, LocalMarkdownRecord, LocalTomlDocument};

pub(super) fn tree_from_local_data_bundle(
    bundle: &LocalDataBundle,
) -> Result<TreeDocument, LocalAuthoringError> {
    let registry = bundle
        .toml_documents
        .iter()
        .find(|document| document.kind.as_deref() == Some("registry"));
    let tree_id = registry
        .and_then(|document| document.data.get("tree"))
        .and_then(|tree| tree.get("id"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("local-tree");
    let tree_title = registry
        .and_then(|document| document.data.get("tree"))
        .and_then(|tree| tree.get("title"))
        .and_then(serde_json::Value::as_str)
        .or_else(|| registry.and_then(|document| document.title.as_deref()))
        .unwrap_or("Local private tree");

    let mut tree = TreeDocument::empty(tree_id, tree_title);
    tree.metadata.description = registry
        .and_then(|document| document.data.get("tree"))
        .and_then(|tree| tree.get("description"))
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned);

    let mut person_ids = BTreeMap::<String, PersonId>::new();
    for record in bundle
        .markdown_records
        .iter()
        .filter(|record| record.kind == "person")
    {
        let person_id = tree.next_person_id();
        person_ids.insert(record.id.clone(), person_id);
        let display = record.title.clone().unwrap_or_else(|| record.id.clone());
        let sex = record
            .attributes
            .get("sex")
            .and_then(serde_json::Value::as_str)
            .map(parse_sex);
        let mut provenance = local_record_provenance(record);
        provenance.tags.extend(record.tags.iter().cloned().map(Tag));
        tree.people.push(Person {
            id: person_id,
            names: vec![Name {
                display,
                given: record
                    .attributes
                    .get("given")
                    .and_then(serde_json::Value::as_str)
                    .map(ToOwned::to_owned),
                surname: record
                    .attributes
                    .get("surname")
                    .and_then(serde_json::Value::as_str)
                    .map(ToOwned::to_owned),
                aliases: string_array_attribute(record.attributes.get("aliases")),
                provenance: local_record_provenance(record),
            }],
            sex,
            events: Vec::new(),
            families_as_child: Vec::new(),
            families_as_spouse: Vec::new(),
            notes: Vec::new(),
            source_record: Some(SourceRef(format!("local:{}", record.id))),
            provenance,
        });
    }

    for (index, record) in bundle
        .markdown_records
        .iter()
        .filter(|record| record.kind == "person")
        .enumerate()
    {
        let Some(person_id) = person_ids.get(&record.id).copied() else {
            continue;
        };
        tree.layout.set_position(
            person_id,
            numeric_attribute(record.attributes.get("x")).unwrap_or((index as f32) * 180.0),
            numeric_attribute(record.attributes.get("y")).unwrap_or(0.0),
        );
        add_person_life_event(&mut tree, person_id, record, "birth_date", EventKind::Birth);
        add_person_life_event(&mut tree, person_id, record, "death_date", EventKind::Death);
    }

    for record in bundle
        .markdown_records
        .iter()
        .filter(|record| is_timeline_event_record(record))
    {
        let participants = event_participants(record, &person_ids)?;
        if participants.is_empty() {
            return Err(LocalAuthoringError::Validation {
                message: format!("{} event has no known person participants", record.path),
            });
        }

        let event_id = next_event_id(&tree);
        let mut provenance = local_record_provenance(record);
        for source_id in string_array_attribute(record.attributes.get("sources")) {
            provenance.sources.push(SourceRef(source_id));
        }
        let event = Event {
            id: event_id,
            kind: event_kind_from_local_kind(&record.kind),
            date: record.date.as_ref().map(|date| {
                DateValue::from_original(date.clone(), local_record_provenance(record))
            }),
            time: record
                .attributes
                .get("time")
                .and_then(toml_json_value_as_string),
            time_zone: record
                .attributes
                .get("time_zone")
                .and_then(toml_json_value_as_string),
            place: None,
            description: record.title.clone().or_else(|| record.summary.clone()),
            participants: participants.clone(),
            provenance,
        };

        tree.events.push(event);
        for person_id in participants {
            if let Some(person) = tree.people.iter_mut().find(|person| person.id == person_id) {
                person.events.push(event_id);
            }
        }
    }

    for document in bundle
        .toml_documents
        .iter()
        .filter(|document| document.kind.as_deref() == Some("relationship"))
    {
        let source = required_json_string(document, "source")?;
        let target = required_json_string(document, "target")?;
        let source_id =
            person_ids
                .get(source)
                .copied()
                .ok_or_else(|| LocalAuthoringError::Validation {
                    message: format!(
                        "{} references missing source person `{source}`",
                        document.path
                    ),
                })?;
        let target_id =
            person_ids
                .get(target)
                .copied()
                .ok_or_else(|| LocalAuthoringError::Validation {
                    message: format!(
                        "{} references missing target person `{target}`",
                        document.path
                    ),
                })?;
        let kind = document
            .data
            .get("relationship")
            .and_then(serde_json::Value::as_str)
            .or_else(|| {
                document
                    .data
                    .get("relationship_kind")
                    .and_then(serde_json::Value::as_str)
            })
            .or_else(|| {
                document
                    .data
                    .get("relation")
                    .and_then(serde_json::Value::as_str)
            })
            .unwrap_or("associate");
        let relationship_id =
            tree.add_relationship(RelationshipKind::from_value(kind), source_id, target_id);
        if let Some(relationship) = tree
            .relationships
            .iter_mut()
            .find(|relationship| relationship.id == relationship_id)
        {
            relationship.label = document.title.clone();
            relationship.provenance.sources.push(SourceRef(format!(
                "local:{}",
                document.id.clone().unwrap_or_else(|| document.path.clone())
            )));
        }
    }

    if let Some(main_person) = registry
        .and_then(|document| document.data.get("tree"))
        .and_then(|tree| tree.get("main_person"))
        .and_then(serde_json::Value::as_str)
        .and_then(|id| person_ids.get(id).copied())
        .or_else(|| tree.people.first().map(|person| person.id))
    {
        tree.main_person = Some(main_person);
    }

    Ok(tree)
}

fn is_timeline_event_record(record: &LocalMarkdownRecord) -> bool {
    record.path.starts_with("events/") || record.attributes.contains_key("participants")
}

fn event_kind_from_local_kind(kind: &str) -> EventKind {
    match kind {
        "birth" => EventKind::Birth,
        "death" => EventKind::Death,
        "marriage" => EventKind::Marriage,
        "baptism" => EventKind::Baptism,
        "burial" => EventKind::Burial,
        "residence" => EventKind::Residence,
        "occupation" => EventKind::Occupation,
        other => EventKind::Other(other.to_string()),
    }
}

fn event_participants(
    record: &LocalMarkdownRecord,
    person_ids: &BTreeMap<String, PersonId>,
) -> Result<Vec<PersonId>, LocalAuthoringError> {
    let Some(value) = record.attributes.get("participants") else {
        return Ok(record
            .related
            .iter()
            .filter_map(|id| person_ids.get(id).copied())
            .collect());
    };

    let Some(items) = value.as_array() else {
        return Err(LocalAuthoringError::Validation {
            message: format!("{} `participants` must be an array", record.path),
        });
    };

    let mut participants = Vec::new();
    for item in items {
        let entity_id = item
            .get("entity")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| LocalAuthoringError::Validation {
                message: format!("{} participant missing `entity`", record.path),
            })?;
        let person_id =
            person_ids
                .get(entity_id)
                .copied()
                .ok_or_else(|| LocalAuthoringError::Validation {
                    message: format!(
                        "{} references missing participant `{entity_id}`",
                        record.path
                    ),
                })?;
        if !participants.contains(&person_id) {
            participants.push(person_id);
        }
    }

    Ok(participants)
}

fn required_json_string<'a>(
    document: &'a LocalTomlDocument,
    key: &str,
) -> Result<&'a str, LocalAuthoringError> {
    document
        .data
        .get(key)
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| LocalAuthoringError::Validation {
            message: format!("{} missing required `{key}`", document.path),
        })
}

fn parse_sex(value: &str) -> Sex {
    match value {
        "male" | "m" | "Male" => Sex::Male,
        "female" | "f" | "Female" => Sex::Female,
        "other" | "Other" => Sex::Other,
        _ => Sex::Unknown,
    }
}

fn add_person_life_event(
    tree: &mut TreeDocument,
    person_id: PersonId,
    record: &LocalMarkdownRecord,
    field: &str,
    kind: EventKind,
) {
    let Some(date) = record
        .attributes
        .get(field)
        .and_then(toml_json_value_as_string)
    else {
        return;
    };

    let id = next_event_id(tree);
    let label = match kind {
        EventKind::Birth => "Birth",
        EventKind::Death => "Death",
        _ => "Life event",
    };
    let event = Event {
        id,
        kind,
        date: Some(DateValue::from_original(
            date,
            local_record_provenance(record),
        )),
        time: record
            .attributes
            .get(&format!("{field}_time"))
            .and_then(toml_json_value_as_string),
        time_zone: record
            .attributes
            .get(&format!("{field}_time_zone"))
            .and_then(toml_json_value_as_string),
        place: None,
        description: Some(format!("{label} for {}", record.id)),
        participants: vec![person_id],
        provenance: local_record_provenance(record),
    };

    tree.events.push(event);
    if let Some(person) = tree.people.iter_mut().find(|person| person.id == person_id) {
        person.events.push(id);
    }
}

fn next_event_id(tree: &TreeDocument) -> EventId {
    EventId(
        tree.events
            .iter()
            .map(|event| event.id.0)
            .max()
            .unwrap_or(0)
            + 1,
    )
}

fn toml_json_value_as_string(value: &serde_json::Value) -> Option<String> {
    value
        .as_str()
        .map(ToOwned::to_owned)
        .or_else(|| value.as_i64().map(|value| value.to_string()))
        .or_else(|| value.as_u64().map(|value| value.to_string()))
        .or_else(|| value.as_f64().map(|value| value.to_string()))
}

fn local_record_provenance(record: &LocalMarkdownRecord) -> Provenance {
    let mut provenance = Provenance::default();
    provenance
        .sources
        .push(SourceRef(format!("local:{}", record.id)));
    provenance.attributes.push(Attribute {
        key: "local_path".to_string(),
        value: record.path.clone(),
    });
    if !record.notes_markdown.is_empty() {
        provenance.attributes.push(Attribute {
            key: "notes_markdown".to_string(),
            value: record.notes_markdown.clone(),
        });
    }
    if let Some(date) = &record.date {
        provenance.attributes.push(Attribute {
            key: "date".to_string(),
            value: date.clone(),
        });
    }
    if let Some(summary) = &record.summary {
        provenance.attributes.push(Attribute {
            key: "summary".to_string(),
            value: summary.clone(),
        });
    }
    provenance
}

fn string_array_attribute(value: Option<&serde_json::Value>) -> Vec<String> {
    value
        .and_then(serde_json::Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn numeric_attribute(value: Option<&serde_json::Value>) -> Option<f32> {
    value
        .and_then(serde_json::Value::as_f64)
        .map(|value| value as f32)
}
