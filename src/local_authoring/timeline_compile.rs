use std::fs;
use std::path::Path;

use super::{
    LocalAuthoringError, LocalDataBundle, LocalMarkdownRecord, LocalTomlDocument,
    compile_local_data,
};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LocalTimelineProjection {
    pub schema_version: u32,
    pub world: String,
    pub view: Option<LocalTimelineViewSummary>,
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
pub struct LocalTimelineEvent {
    pub id: String,
    pub kind: String,
    pub title: Option<String>,
    pub time: Option<String>,
    pub participants: Vec<serde_json::Value>,
    pub places: Vec<serde_json::Value>,
    pub assertions: Vec<String>,
    pub path: String,
    pub notes_markdown: String,
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
    let event_kinds = view.and_then(timeline_view_event_kinds).unwrap_or_default();
    let mut events = bundle
        .markdown_records
        .iter()
        .filter(|record| is_event_record(record))
        .filter(|record| {
            event_kinds.is_empty() || event_kinds.iter().any(|kind| kind == &record.kind)
        })
        .map(timeline_event_from_record)
        .collect::<Vec<_>>();

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
        events,
    }
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

fn timeline_view_event_kinds(view: &LocalTomlDocument) -> Option<Vec<String>> {
    view.data
        .get("filter")
        .and_then(|filter| filter.get("event_kinds"))
        .and_then(serde_json::Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(ToOwned::to_owned)
                .collect()
        })
}

fn timeline_event_from_record(record: &LocalMarkdownRecord) -> LocalTimelineEvent {
    LocalTimelineEvent {
        id: record.id.clone(),
        kind: record.kind.clone(),
        title: record.title.clone().or_else(|| {
            record
                .attributes
                .get("primary_name")
                .and_then(serde_json::Value::as_str)
                .map(ToOwned::to_owned)
        }),
        time: record
            .date
            .clone()
            .or_else(|| record.attributes.get("time").and_then(json_value_as_string)),
        participants: json_array(record.attributes.get("participants")),
        places: json_array(record.attributes.get("places")),
        assertions: string_array(record.attributes.get("assertions")),
        path: record.path.clone(),
        notes_markdown: record.notes_markdown.clone(),
    }
}

fn is_event_record(record: &LocalMarkdownRecord) -> bool {
    record.path.starts_with("events/") || record.attributes.contains_key("participants")
}

fn json_array(value: Option<&serde_json::Value>) -> Vec<serde_json::Value> {
    value
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default()
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
}
