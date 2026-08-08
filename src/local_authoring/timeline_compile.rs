use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use super::{
    LocalAuthoringError, LocalDataBundle, LocalMarkdownRecord, LocalTomlDocument,
    compile_local_data, event_locations,
    event_profiles::{
        default_event_label, event_participant_entity_ids, event_participants, local_event_type_id,
    },
    refs::normalize_person_id,
    sources::source_items,
};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LocalTimelineProjection {
    pub schema_version: u32,
    pub world: String,
    pub view: Option<LocalTimelineViewSummary>,
    pub collections: Vec<LocalTimelineCollection>,
    pub events: Vec<LocalTimelineEvent>,
}

impl LocalTimelineProjection {
    pub const SCHEMA_VERSION: u32 = 1;
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LocalTimelineViewSummary {
    pub id: String,
    pub title: Option<String>,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LocalTimelineCollection {
    pub id: String,
    pub title: Option<String>,
    pub kind: String,
    pub order: Option<String>,
    pub members: Vec<LocalTimelineCollectionMember>,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LocalTimelineCollectionMember {
    pub event: String,
    pub label: Option<String>,
    pub role: Option<String>,
    pub ordinal: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LocalTimelineInlineAssertion {
    pub target: String,
    pub confidence: Option<String>,
    pub sources: Vec<String>,
    pub note: Option<String>,
    pub value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LocalTimelineEvent {
    pub id: String,
    pub event_type: String,
    pub title: Option<String>,
    pub time: Option<String>,
    pub participants: Vec<serde_json::Value>,
    pub places: Vec<serde_json::Value>,
    pub assertions: Vec<String>,
    pub inline_assertions: Vec<LocalTimelineInlineAssertion>,
    pub path: String,
    pub notes_markdown: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct TimelineFilter {
    subject: Option<String>,
    event_types: Vec<String>,
    related_entities: Vec<String>,
    include_context_events: bool,
}

pub fn compile_local_timeline(
    world_root: impl AsRef<Path>,
    view_slug: Option<&str>,
) -> Result<LocalTimelineProjection, LocalAuthoringError> {
    let bundle = compile_local_data(world_root)?;
    Ok(timeline_from_local_data_bundle(&bundle, view_slug))
}

pub fn write_local_timeline_json(
    world_root: impl AsRef<Path>,
    view_slug: Option<&str>,
    output_path: impl AsRef<Path>,
) -> Result<LocalTimelineProjection, LocalAuthoringError> {
    let output_path = output_path.as_ref();
    let timeline = compile_local_timeline(world_root, view_slug)?;
    let json =
        serde_json::to_string_pretty(&timeline).map_err(|source| LocalAuthoringError::Json {
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

    Ok(timeline)
}

fn timeline_from_local_data_bundle(
    bundle: &LocalDataBundle,
    view_slug: Option<&str>,
) -> LocalTimelineProjection {
    let world = bundle
        .toml_documents
        .iter()
        .find(|document| document.path == "world.toml")
        .and_then(|document| document.id.clone())
        .unwrap_or_else(|| "world:default".to_string());
    let view = select_timeline_view(&bundle.toml_documents, view_slug);
    let filter = view
        .map(|view| timeline_filter(view, &bundle.toml_documents))
        .unwrap_or_default();
    let entity_names = entity_name_index(&bundle.markdown_records);
    let mut events = bundle
        .markdown_records
        .iter()
        .filter(|record| is_event_record(record))
        .filter(|record| timeline_filter_includes_record(record, &filter))
        .map(|record| timeline_event_from_record(record, &entity_names))
        .collect::<Vec<_>>();
    let collections = timeline_collections_from_documents(&bundle.toml_documents, &events);

    events.sort_by(|left, right| {
        left.time
            .cmp(&right.time)
            .then_with(|| left.id.cmp(&right.id))
    });

    LocalTimelineProjection {
        schema_version: LocalTimelineProjection::SCHEMA_VERSION,
        world,
        view: view.map(|view| LocalTimelineViewSummary {
            id: view.id.clone().unwrap_or_else(|| view.path.clone()),
            title: view.title.clone(),
            path: view.path.clone(),
        }),
        collections,
        events,
    }
}

fn timeline_collections_from_documents(
    documents: &[LocalTomlDocument],
    events: &[LocalTimelineEvent],
) -> Vec<LocalTimelineCollection> {
    let event_ids = events
        .iter()
        .map(|event| event.id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let mut collections = documents
        .iter()
        .filter(|document| document.kind.as_deref() == Some("event-collection"))
        .filter_map(|document| timeline_collection_from_document(document, &event_ids))
        .collect::<Vec<_>>();
    collections.sort_by(|left, right| left.id.cmp(&right.id));
    collections
}

fn timeline_collection_from_document(
    document: &LocalTomlDocument,
    event_ids: &std::collections::BTreeSet<&str>,
) -> Option<LocalTimelineCollection> {
    let members = document
        .data
        .get("members")
        .and_then(serde_json::Value::as_array)
        .map(|members| {
            members
                .iter()
                .filter_map(|member| {
                    let event = member.get("event")?.as_str()?.to_string();
                    event_ids
                        .contains(event.as_str())
                        .then(|| LocalTimelineCollectionMember {
                            event,
                            label: member
                                .get("label")
                                .and_then(serde_json::Value::as_str)
                                .map(ToOwned::to_owned),
                            role: member
                                .get("role")
                                .and_then(serde_json::Value::as_str)
                                .map(ToOwned::to_owned),
                            ordinal: member.get("ordinal").and_then(serde_json::Value::as_i64),
                        })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    if members.is_empty() {
        return None;
    }

    Some(LocalTimelineCollection {
        id: document.id.clone().unwrap_or_else(|| document.path.clone()),
        title: document.title.clone(),
        kind: document
            .data
            .get("collection_kind")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("set")
            .to_string(),
        order: document
            .data
            .get("order")
            .and_then(serde_json::Value::as_str)
            .map(ToOwned::to_owned),
        members,
        path: document.path.clone(),
    })
}

fn select_timeline_view<'a>(
    documents: &'a [LocalTomlDocument],
    view_slug: Option<&str>,
) -> Option<&'a LocalTomlDocument> {
    let timelines = documents
        .iter()
        .filter(|document| document.kind.as_deref() == Some("timeline-view"));

    if let Some(view_slug) = view_slug {
        let view_id = format!("timeline:{view_slug}");
        return timelines.into_iter().find(|document| {
            document.id.as_deref() == Some(view_id.as_str())
                || document.path == format!("views/timelines/{view_slug}.toml")
        });
    }

    timelines.into_iter().next()
}

fn timeline_filter(view: &LocalTomlDocument, documents: &[LocalTomlDocument]) -> TimelineFilter {
    let subject = view
        .data
        .get("subject")
        .and_then(|subject| subject.get("entity"))
        .and_then(serde_json::Value::as_str)
        .map(normalize_person_id);
    let event_types = timeline_view_event_types(view).unwrap_or_default();
    let include_related_people = view
        .data
        .get("filter")
        .and_then(|filter| filter.get("include_related_people"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let include_context_events = view
        .data
        .get("filter")
        .and_then(|filter| filter.get("include_context_events"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let related_entities = subject
        .as_deref()
        .filter(|_| include_related_people)
        .map(|subject| related_entities_for_subject(subject, documents))
        .unwrap_or_default();

    TimelineFilter {
        subject,
        event_types,
        related_entities,
        include_context_events,
    }
}

fn related_entities_for_subject(subject: &str, documents: &[LocalTomlDocument]) -> Vec<String> {
    let mut related = Vec::new();
    for document in documents
        .iter()
        .filter(|document| document.kind.as_deref() == Some("relationship"))
    {
        let source = document
            .data
            .get("source")
            .and_then(serde_json::Value::as_str);
        let target = document
            .data
            .get("target")
            .and_then(serde_json::Value::as_str);
        match (source, target) {
            (Some(source), Some(target)) if source == subject => push_unique(&mut related, target),
            (Some(source), Some(target)) if target == subject => push_unique(&mut related, source),
            _ => {}
        }
    }
    related
}

fn timeline_filter_includes_record(record: &LocalMarkdownRecord, filter: &TimelineFilter) -> bool {
    let record_kind = local_event_type_id(record).unwrap_or_else(|| record.kind.clone());
    if !filter.event_types.is_empty() && !filter.event_types.iter().any(|kind| kind == &record_kind)
    {
        return false;
    }

    let Some(subject) = filter.subject.as_deref() else {
        return true;
    };

    let participant_ids = participant_entity_ids(record);
    if participant_ids.iter().any(|id| id == subject) {
        return true;
    }

    if participant_ids
        .iter()
        .any(|id| filter.related_entities.iter().any(|related| related == id))
    {
        return true;
    }

    filter.include_context_events && participant_ids.is_empty()
}

fn timeline_view_event_types(view: &LocalTomlDocument) -> Option<Vec<String>> {
    view.data
        .get("filter")
        .and_then(|filter| filter.get("event_types"))
        .and_then(serde_json::Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(ToOwned::to_owned)
                .collect()
        })
}

fn timeline_event_from_record(
    record: &LocalMarkdownRecord,
    entity_names: &BTreeMap<String, String>,
) -> LocalTimelineEvent {
    LocalTimelineEvent {
        id: record.id.clone(),
        event_type: local_event_type_id(record).unwrap_or_else(|| record.kind.clone()),
        title: event_title(record, entity_names),
        time: record
            .date
            .clone()
            .or_else(|| record.attributes.get("time").and_then(json_value_as_string)),
        participants: event_participants(record),
        places: event_locations(record),
        assertions: string_array(record.attributes.get("assertions")),
        inline_assertions: inline_assertions(record),
        path: record.path.clone(),
        notes_markdown: record.notes_markdown.clone(),
    }
}

fn event_title(
    record: &LocalMarkdownRecord,
    entity_names: &BTreeMap<String, String>,
) -> Option<String> {
    record
        .title
        .clone()
        .or_else(|| default_event_label(record, |entity| entity_names.get(entity).cloned()))
}

fn entity_name_index(records: &[LocalMarkdownRecord]) -> BTreeMap<String, String> {
    records
        .iter()
        .filter(|record| {
            matches!(
                record.kind.as_str(),
                "person" | "place" | "organization" | "object" | "concept"
            )
        })
        .filter_map(|record| markdown_record_name(record).map(|name| (record.id.clone(), name)))
        .collect()
}

fn markdown_record_name(record: &LocalMarkdownRecord) -> Option<String> {
    record
        .title
        .clone()
        .or_else(|| markdown_name_table(record, "preferred"))
        .or_else(|| markdown_name_table(record, "legal"))
}

fn markdown_name_table(record: &LocalMarkdownRecord, usage: &str) -> Option<String> {
    let table = record.attributes.get("names")?.get(usage)?;
    table
        .get("display")
        .and_then(serde_json::Value::as_str)
        .or_else(|| table.get("full").and_then(serde_json::Value::as_str))
        .map(ToOwned::to_owned)
        .or_else(|| {
            let given = table.get("given").and_then(serde_json::Value::as_str)?;
            let family = table.get("family").and_then(serde_json::Value::as_str)?;
            Some(format!("{given} {family}"))
        })
}

fn inline_assertions(record: &LocalMarkdownRecord) -> Vec<LocalTimelineInlineAssertion> {
    record
        .attributes
        .get("assertions")
        .and_then(serde_json::Value::as_array)
        .map(|assertions| {
            assertions
                .iter()
                .filter_map(|assertion| inline_assertion(record, assertion))
                .collect()
        })
        .unwrap_or_default()
}

fn inline_assertion(
    record: &LocalMarkdownRecord,
    assertion: &serde_json::Value,
) -> Option<LocalTimelineInlineAssertion> {
    let assertion = assertion.as_object()?;
    let target = assertion.get("target")?.as_str()?;
    Some(LocalTimelineInlineAssertion {
        target: if target.starts_with('#') {
            format!("{}{}", record.id, target)
        } else {
            target.to_string()
        },
        confidence: assertion
            .get("confidence")
            .and_then(serde_json::Value::as_str)
            .map(ToOwned::to_owned),
        sources: source_items(assertion.get("sources"))
            .into_iter()
            .filter_map(|source| match source {
                serde_json::Value::String(source) => Some(source),
                value => Some(value.to_string()),
            })
            .collect(),
        note: assertion
            .get("note")
            .and_then(serde_json::Value::as_str)
            .map(ToOwned::to_owned),
        value: assertion.get("value").and_then(json_value_as_string),
    })
}

fn is_event_record(record: &LocalMarkdownRecord) -> bool {
    record.kind == "event"
}

fn participant_entity_ids(record: &LocalMarkdownRecord) -> Vec<String> {
    event_participant_entity_ids(record)
}

fn push_unique(values: &mut Vec<String>, value: &str) {
    if !values.iter().any(|existing| existing == value) {
        values.push(value.to_string());
    }
}

fn string_array(value: Option<&serde_json::Value>) -> Vec<String> {
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

fn json_value_as_string(value: &serde_json::Value) -> Option<String> {
    value
        .as_str()
        .map(ToOwned::to_owned)
        .or_else(|| value.as_i64().map(|value| value.to_string()))
        .or_else(|| value.as_u64().map(|value| value.to_string()))
        .or_else(|| value.as_f64().map(|value| value.to_string()))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::local_authoring::{LocalSkeletonOptions, create_workspace_skeleton};

    #[test]
    fn writes_timeline_projection_for_world_view() {
        let temp_dir = std::env::temp_dir().join(format!(
            "kleio-timeline-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time")
                .as_nanos()
        ));
        create_workspace_skeleton(
            &temp_dir,
            &LocalSkeletonOptions {
                birth_date: Some("1900-01-01".to_string()),
                ..LocalSkeletonOptions::default()
            },
        )
        .expect("skeleton");
        let world_root = temp_dir.join("worlds/default");
        let out = world_root.join("build/example-life.timeline.json");

        let timeline = write_local_timeline_json(&world_root, Some("example-life"), &out)
            .expect("write timeline");

        assert_eq!(timeline.world, "world:default");
        assert_eq!(
            timeline.view.as_ref().map(|view| view.id.as_str()),
            Some("timeline:example-life")
        );
        assert!(
            timeline
                .events
                .iter()
                .any(|event| event.id == "event:birth-example-person")
        );
        assert!(out.exists());

        fs::remove_dir_all(temp_dir).expect("remove temp dir");
    }

    #[test]
    fn filters_timeline_by_subject_and_related_people() {
        let temp_dir = std::env::temp_dir().join(format!(
            "kleio-timeline-related-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time")
                .as_nanos()
        ));
        create_workspace_skeleton(
            &temp_dir,
            &LocalSkeletonOptions {
                birth_date: Some("1900-01-01".to_string()),
                ..LocalSkeletonOptions::default()
            },
        )
        .expect("skeleton");
        let world_root = temp_dir.join("worlds/default");
        fs::write(
            world_root.join("entities/people/morgan-example.md"),
            "+++\nschema_version = 1\nid = \"person:morgan-example\"\nkind = \"person\"\npreferred_name = \"Morgan Example\"\n+++\n\n# Morgan\n",
        )
        .expect("related person");
        fs::write(
            world_root.join("relationships/example-association.toml"),
            "schema_version = 1\nid = \"relationship:example-association\"\nkind = \"relationship\"\nrelationship = \"associate\"\nsource = \"person:example-person\"\ntarget = \"person:morgan-example\"\n",
        )
        .expect("relationship");
        fs::write(
            world_root.join("events/observations/related-observation.md"),
"+++\nschema_version = 1\nid = \"event:related-observation\"\nkind = \"event\"\ntype = \"residence\"\ntitle = \"Related observation\"\ntime = \"1905-01-01\"\nparticipants = [\"person:morgan-example\"]\nplaces = []\nassertions = []\n+++\n\n# Related\n"
        )
        .expect("related event");
        fs::write(
            world_root.join("events/observations/unrelated-observation.md"),
"+++\nschema_version = 1\nid = \"event:unrelated-observation\"\nkind = \"event\"\ntype = \"residence\"\ntitle = \"Unrelated observation\"\ntime = \"1906-01-01\"\nparticipants = []\nplaces = []\nassertions = []\n+++\n\n# Unrelated\n"
        )
        .expect("unrelated event");

        let timeline =
            compile_local_timeline(&world_root, Some("example-life")).expect("compile timeline");
        let ids = timeline
            .events
            .iter()
            .map(|event| event.id.as_str())
            .collect::<Vec<_>>();

        assert!(ids.contains(&"event:birth-example-person"));
        assert!(ids.contains(&"event:related-observation"));
        assert!(!ids.contains(&"event:unrelated-observation"));

        fs::remove_dir_all(temp_dir).expect("remove temp dir");
    }
}
