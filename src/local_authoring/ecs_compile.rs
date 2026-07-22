use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use super::{
    LocalAuthoringError, LocalDataBundle, LocalMarkdownRecord, compile_local_data,
    infer_local_kinship_relationships,
};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LocalEcsBundle {
    pub schema_version: u32,
    pub world: String,
    pub entities: Vec<LocalEcsEntity>,
    pub resources: LocalEcsResources,
}

impl LocalEcsBundle {
    pub const SCHEMA_VERSION: u32 = 1;
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LocalEcsEntity {
    pub id: String,
    pub components: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LocalEcsResources {
    #[serde(rename = "Views")]
    pub views: LocalEcsViews,
    #[serde(rename = "Relationships")]
    pub relationships: LocalEcsRelationships,
}

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LocalEcsRelationships {
    pub items: Vec<LocalEcsRelationship>,
    pub derived_kinship: Vec<super::LocalDerivedKinshipRelationship>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LocalEcsRelationship {
    pub id: String,
    pub relationship_kind: String,
    pub source: String,
    pub target: String,
    pub title: Option<String>,
    pub sources: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LocalEcsViews {
    pub timelines: Vec<String>,
    pub trees: Vec<String>,
    pub maps: Vec<String>,
    pub calendars: Vec<String>,
    pub visualizations: Vec<String>,
}

pub fn compile_local_ecs(
    world_root: impl AsRef<Path>,
) -> Result<LocalEcsBundle, LocalAuthoringError> {
    let world_root = world_root.as_ref();
    let bundle = compile_local_data(world_root)?;
    Ok(ecs_from_local_data_bundle(&bundle))
}

pub fn write_local_ecs_json(
    world_root: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
) -> Result<LocalEcsBundle, LocalAuthoringError> {
    let output_path = output_path.as_ref();
    let bundle = compile_local_ecs(world_root)?;
    let json =
        serde_json::to_string_pretty(&bundle).map_err(|source| LocalAuthoringError::Json {
            path: output_path.to_path_buf(),
            source,
        })?;

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).map_err(|source| LocalAuthoringError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    fs::write(output_path, format!("{json}\n")).map_err(|source| LocalAuthoringError::Io {
        path: output_path.to_path_buf(),
        source,
    })?;

    Ok(bundle)
}

fn ecs_from_local_data_bundle(bundle: &LocalDataBundle) -> LocalEcsBundle {
    let world = bundle
        .toml_documents
        .iter()
        .find(|document| document.path == "world.toml")
        .and_then(|document| document.id.clone())
        .unwrap_or_else(|| "world:default".to_string());

    let mut entities = bundle
        .markdown_records
        .iter()
        .filter(|record| is_entity_record(record) || is_event_record(record))
        .map(ecs_entity_from_markdown_record)
        .collect::<Vec<_>>();
    for record in bundle
        .markdown_records
        .iter()
        .filter(|record| is_event_record(record))
    {
        entities.extend(inline_assertion_entities(record));
    }

    entities.sort_by(|left, right| left.id.cmp(&right.id));

    let mut timelines = Vec::new();
    let mut trees = Vec::new();
    let mut maps = Vec::new();
    let mut calendars = Vec::new();
    let mut visualizations = Vec::new();
    for document in &bundle.toml_documents {
        match document.kind.as_deref() {
            Some("timeline-view") => {
                if let Some(id) = &document.id {
                    timelines.push(id.clone());
                }
            }
            Some("tree-view") => {
                if let Some(id) = &document.id {
                    trees.push(id.clone());
                }
            }
            Some("map-view") => {
                if let Some(id) = &document.id {
                    maps.push(id.clone());
                }
            }
            Some("calendar-view") => {
                if let Some(id) = &document.id {
                    calendars.push(id.clone());
                }
            }
            Some("visualization-view") => {
                if let Some(id) = &document.id {
                    visualizations.push(id.clone());
                }
            }
            _ => {}
        }
    }

    LocalEcsBundle {
        schema_version: LocalEcsBundle::SCHEMA_VERSION,
        world,
        entities,
        resources: LocalEcsResources {
            views: LocalEcsViews {
                timelines,
                trees,
                maps,
                calendars,
                visualizations,
            },
            relationships: LocalEcsRelationships {
                items: ecs_relationships_from_documents(&bundle.toml_documents),
                derived_kinship: infer_local_kinship_relationships(bundle),
            },
        },
    }
}

fn ecs_relationships_from_documents(
    documents: &[super::LocalTomlDocument],
) -> Vec<LocalEcsRelationship> {
    let mut relationships = documents
        .iter()
        .filter(|document| document.kind.as_deref() == Some("relationship"))
        .filter_map(|document| {
            let id = document.id.clone()?;
            let source = document.data.get("source")?.as_str()?.to_string();
            let target = document.data.get("target")?.as_str()?.to_string();
            let relationship_kind = document
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
                .unwrap_or("associate")
                .to_string();
            let sources = document
                .data
                .get("sources")
                .and_then(serde_json::Value::as_array)
                .map(|values| {
                    values
                        .iter()
                        .filter_map(serde_json::Value::as_str)
                        .map(ToOwned::to_owned)
                        .collect()
                })
                .unwrap_or_default();
            Some(LocalEcsRelationship {
                id,
                relationship_kind,
                source,
                target,
                title: document.title.clone(),
                sources,
            })
        })
        .collect::<Vec<_>>();
    relationships.sort_by(|left, right| left.id.cmp(&right.id));
    relationships
}

fn ecs_entity_from_markdown_record(record: &LocalMarkdownRecord) -> LocalEcsEntity {
    let mut components = BTreeMap::new();
    components.insert(
        "Identity".to_string(),
        serde_json::json!({
            "id": record.id,
            "kind": record.kind,
        }),
    );

    if matches!(
        record.kind.as_str(),
        "person" | "place" | "organization" | "object" | "concept"
    ) {
        let component = match record.kind.as_str() {
            "person" => "Person",
            "place" => "Place",
            "organization" => "Organization",
            "object" => "Object",
            "concept" => "Concept",
            _ => unreachable!("matched above"),
        };
        components.insert(component.to_string(), serde_json::json!({}));
        if let Some(full) = primary_name(record) {
            components.insert(
                "PrimaryName".to_string(),
                serde_json::json!({ "full": full }),
            );
        }
    }

    if record.path.starts_with("assertions/") {
        components.insert(
            "Assertion".to_string(),
            serde_json::json!({
                "assertion_kind": record.kind,
                "target": record.attributes.get("target"),
                "value": record.attributes.get("value"),
                "confidence": record.attributes.get("confidence"),
                "sources": record
                    .attributes
                    .get("sources")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!([])),
            }),
        );
    }

    if record.path.starts_with("sources/") {
        components.insert(
            "Source".to_string(),
            serde_json::json!({
                "source_kind": record.kind,
                "title": record.title,
                "media": record
                    .attributes
                    .get("media")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!([])),
            }),
        );
    }

    if is_event_record(record) {
        components.insert(
            "HistoricalEvent".to_string(),
            serde_json::json!({ "event_kind": record.kind }),
        );
        if let Some(value) = record
            .date
            .as_ref()
            .or_else(|| record.attributes.get("time").and_then(json_string_ref))
        {
            components.insert(
                "TimePoint".to_string(),
                serde_json::json!({ "value": value }),
            );
        }
        if let Some(participants) = record.attributes.get("participants") {
            components.insert(
                "Participants".to_string(),
                serde_json::json!({ "items": participants }),
            );
        }
    }

    if let Some(assertions) = record.attributes.get("assertions") {
        let external_assertions = external_assertion_ids(assertions);
        if !external_assertions.is_empty() {
            components.insert(
                "SourceLinks".to_string(),
                serde_json::json!({ "assertions": external_assertions }),
            );
        }
    }

    LocalEcsEntity {
        id: record.id.clone(),
        components,
    }
}

fn inline_assertion_entities(record: &LocalMarkdownRecord) -> Vec<LocalEcsEntity> {
    record
        .attributes
        .get("assertions")
        .and_then(serde_json::Value::as_array)
        .map(|assertions| {
            assertions
                .iter()
                .enumerate()
                .filter_map(|(index, assertion)| inline_assertion_entity(record, index, assertion))
                .collect()
        })
        .unwrap_or_default()
}

fn inline_assertion_entity(
    record: &LocalMarkdownRecord,
    index: usize,
    assertion: &serde_json::Value,
) -> Option<LocalEcsEntity> {
    let assertion = assertion.as_object()?;
    let target = assertion.get("target")?.as_str()?;
    let target = if target.starts_with('#') {
        format!("{}{}", record.id, target)
    } else {
        target.to_string()
    };
    let id = format!("{}#assertion:{}", record.id, index + 1);
    let mut components = BTreeMap::new();
    components.insert(
        "Assertion".to_string(),
        serde_json::json!({
            "assertion_kind": assertion
                .get("kind")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("event-support"),
            "target": target,
            "value": assertion.get("value"),
            "confidence": assertion.get("confidence"),
            "sources": assertion
                .get("sources")
                .cloned()
                .unwrap_or_else(|| serde_json::json!([])),
            "note": assertion.get("note"),
            "inline_in": record.id,
        }),
    );

    Some(LocalEcsEntity { id, components })
}

fn external_assertion_ids(assertions: &serde_json::Value) -> Vec<String> {
    assertions
        .as_array()
        .map(|assertions| {
            assertions
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn is_entity_record(record: &LocalMarkdownRecord) -> bool {
    record.path.starts_with("entities/")
        || record.path.starts_with("assertions/")
        || record.path.starts_with("sources/")
        || matches!(record.kind.as_str(), "person" | "place")
}

fn is_event_record(record: &LocalMarkdownRecord) -> bool {
    record.path.starts_with("events/") || record.attributes.contains_key("participants")
}

fn json_string_ref(value: &serde_json::Value) -> Option<&String> {
    match value {
        serde_json::Value::String(value) => Some(value),
        _ => None,
    }
}

fn primary_name(record: &LocalMarkdownRecord) -> Option<&str> {
    record.title.as_deref().or_else(|| {
        record
            .attributes
            .get("primary_name")
            .and_then(serde_json::Value::as_str)
    })
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::local_authoring::{LocalSkeletonOptions, create_workspace_skeleton};

    #[test]
    fn writes_minimal_ecs_json_for_world() {
        let temp_dir = std::env::temp_dir().join(format!(
            "kleio-ecs-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time")
                .as_nanos()
        ));
        create_workspace_skeleton(&temp_dir, &LocalSkeletonOptions::default()).expect("skeleton");
        let world_root = temp_dir.join("worlds/default");
        let out = world_root.join("build/kleio.ecs.json");

        let ecs = write_local_ecs_json(&world_root, &out).expect("write ecs");

        assert_eq!(ecs.world, "world:default");
        assert!(
            ecs.entities
                .iter()
                .any(|entity| entity.id == "person:example-person")
        );
        assert!(
            ecs.entities
                .iter()
                .any(|entity| entity.id == "source:personal-knowledge")
        );
        assert!(out.exists());

        fs::remove_dir_all(temp_dir).expect("remove temp dir");
    }

    #[test]
    fn includes_relationship_resources() {
        let temp_dir = std::env::temp_dir().join(format!(
            "kleio-ecs-relationships-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time")
                .as_nanos()
        ));
        create_workspace_skeleton(&temp_dir, &LocalSkeletonOptions::default()).expect("skeleton");
        let world_root = temp_dir.join("worlds/default");
        fs::create_dir_all(world_root.join("relationships")).expect("relationships dir");
        fs::write(
            world_root.join("relationships/example-association.toml"),
            "id = \"relationship:example-association\"\nkind = \"relationship\"\ntitle = \"Example association\"\nrelationship = \"associate\"\nsource = \"person:example-person\"\ntarget = \"person:example-person\"\nsources = [\"source:personal-knowledge\"]\n",
        )
        .expect("relationship");

        let ecs = compile_local_ecs(&world_root).expect("compile ecs");

        assert_eq!(ecs.resources.relationships.items.len(), 1);
        assert_eq!(
            ecs.resources.relationships.items[0].id,
            "relationship:example-association"
        );
        assert_eq!(
            ecs.resources.relationships.items[0].source,
            "person:example-person"
        );
        assert_eq!(
            ecs.resources.relationships.items[0].target,
            "person:example-person"
        );
        assert!(ecs.resources.relationships.derived_kinship.is_empty());

        fs::remove_dir_all(temp_dir).expect("remove temp dir");
    }

    #[test]
    fn includes_inferred_kinship_relationship_resources() {
        let temp_dir = std::env::temp_dir().join(format!(
            "kleio-ecs-derived-kinship-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time")
                .as_nanos()
        ));
        fs::create_dir_all(temp_dir.join("entities/people")).expect("people dir");
        fs::create_dir_all(temp_dir.join("relationships")).expect("relationships dir");
        for slug in ["alex", "morgan", "riley"] {
            fs::write(
                temp_dir.join(format!("entities/people/{slug}.md")),
                format!(
                    "+++\nid = \"person:{slug}\"\nkind = \"person\"\nprimary_name = \"{slug}\"\n+++\n"
                ),
            )
            .expect("person");
        }
        for (slug, source, target) in [
            ("alex-morgan", "person:alex", "person:morgan"),
            ("morgan-riley", "person:morgan", "person:riley"),
        ] {
            fs::write(
                temp_dir.join(format!("relationships/{slug}.toml")),
                format!(
                    "id = \"relationship:{slug}\"\nkind = \"relationship\"\nrelationship = \"biological-parent-child\"\nsource = \"{source}\"\ntarget = \"{target}\"\n"
                ),
            )
            .expect("relationship");
        }

        let ecs = compile_local_ecs(&temp_dir).expect("compile ecs");

        assert!(
            ecs.resources
                .relationships
                .derived_kinship
                .iter()
                .any(|relationship| {
                    relationship.relationship_kind == "grandparent-grandchild"
                        && relationship.source == "person:alex"
                        && relationship.target == "person:riley"
                })
        );

        fs::remove_dir_all(temp_dir).expect("remove temp dir");
    }
}
