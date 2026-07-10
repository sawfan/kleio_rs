use std::fs;
use std::path::{Path, PathBuf};

use super::{LocalAuthoringError, WorldPaths, compile_local_data};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalViewKind {
    Timeline,
    Tree,
    Map,
    Calendar,
    Visualization,
}

impl LocalViewKind {
    pub fn id_prefix(self) -> &'static str {
        match self {
            Self::Timeline => "timeline",
            Self::Tree => "tree",
            Self::Map => "map",
            Self::Calendar => "calendar",
            Self::Visualization => "visualization",
        }
    }

    pub fn kind_value(self) -> &'static str {
        match self {
            Self::Timeline => "timeline-view",
            Self::Tree => "tree-view",
            Self::Map => "map-view",
            Self::Calendar => "calendar-view",
            Self::Visualization => "visualization-view",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LocalViewSummary {
    pub id: Option<String>,
    pub kind: String,
    pub title: Option<String>,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalViewOptions {
    pub view_slug: String,
    pub title: String,
    pub kind: LocalViewKind,
    pub subject: Option<String>,
    pub force: bool,
}

impl LocalViewOptions {
    pub fn id(&self) -> String {
        format!("{}:{}", self.kind.id_prefix(), self.view_slug)
    }
}

pub fn create_local_view(
    world_root: impl AsRef<Path>,
    options: &LocalViewOptions,
) -> Result<PathBuf, LocalAuthoringError> {
    validate_slug(&options.view_slug, "view slug")?;
    let world_root = world_root.as_ref();
    let paths = WorldPaths::new(world_root);
    let dir = match options.kind {
        LocalViewKind::Timeline => paths.timeline_views_dir(),
        LocalViewKind::Tree => paths.tree_views_dir(),
        LocalViewKind::Map => paths.map_views_dir(),
        LocalViewKind::Calendar => paths.calendar_views_dir(),
        LocalViewKind::Visualization => paths.visualization_views_dir(),
    };
    create_dir(world_root, &dir)?;
    let path = dir.join(format!("{}.toml", options.view_slug));
    write_new_file(world_root, &path, &view_toml(options), options.force)?;
    Ok(path)
}

pub fn list_local_views(
    world_root: impl AsRef<Path>,
    kind: Option<LocalViewKind>,
) -> Result<Vec<LocalViewSummary>, LocalAuthoringError> {
    let bundle = compile_local_data(world_root)?;
    let mut views = bundle
        .toml_documents
        .iter()
        .filter(|document| {
            kind.map(|kind| document.kind.as_deref() == Some(kind.kind_value()))
                .unwrap_or_else(|| {
                    matches!(
                        document.kind.as_deref(),
                        Some(
                            "timeline-view"
                                | "tree-view"
                                | "map-view"
                                | "calendar-view"
                                | "visualization-view"
                        )
                    )
                })
        })
        .map(|document| LocalViewSummary {
            id: document.id.clone(),
            kind: document.kind.clone().unwrap_or_default(),
            title: document.title.clone(),
            path: document.path.clone(),
        })
        .collect::<Vec<_>>();
    views.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(views)
}

fn view_toml(options: &LocalViewOptions) -> String {
    match options.kind {
        LocalViewKind::Timeline => timeline_view_toml(options),
        LocalViewKind::Tree => tree_view_toml(options),
        LocalViewKind::Map => map_view_toml(options),
        LocalViewKind::Calendar => calendar_view_toml(options),
        LocalViewKind::Visualization => visualization_view_toml(options),
    }
}

fn timeline_view_toml(options: &LocalViewOptions) -> String {
    let subject = options
        .subject
        .as_deref()
        .map(|entity| format!("\n[subject]\nentity = \"{}\"\n", escape_toml_basic(entity)))
        .unwrap_or_default();

    format!(
        r#"schema_version = 1
id = "{}"
kind = "{}"
title = "{}"
{}
[filter]
event_kinds = ["birth", "residence", "marriage", "death"]
include_related_people = true
include_context_events = false

[sort]
by = "date"
direction = "ascending"

[display]
group_by = "year"
show_sources = true
show_uncertainty = true
"#,
        escape_toml_basic(&options.id()),
        options.kind.kind_value(),
        escape_toml_basic(&options.title),
        subject.trim_end()
    )
}

fn tree_view_toml(options: &LocalViewOptions) -> String {
    let root = options
        .subject
        .as_deref()
        .map(|entity| format!("\n[root]\nentity = \"{}\"\n", escape_toml_basic(entity)))
        .unwrap_or_default();

    format!(
        r#"schema_version = 1
id = "{}"
kind = "{}"
title = "{}"
{}
[filter]
relationship_kinds = ["biological-parent-child", "adoptive-parent-child", "foster-parent-child", "step-parent-child", "guardian-child", "spouse", "partner", "former-spouse", "sibling"]
generations_up = 5
generations_down = 3

[display]
show_life_dates = true
show_places = true
show_sources = false
"#,
        escape_toml_basic(&options.id()),
        options.kind.kind_value(),
        escape_toml_basic(&options.title),
        root.trim_end()
    )
}

fn map_view_toml(options: &LocalViewOptions) -> String {
    generic_view_toml(
        options,
        "[filter]\nplace_kinds = []\nevent_kinds = []\n\n[display]\nshow_labels = true\nshow_event_markers = true\n",
    )
}

fn calendar_view_toml(options: &LocalViewOptions) -> String {
    generic_view_toml(
        options,
        "[filter]\nevent_kinds = []\n\n[display]\ngroup_by = \"month\"\nshow_sources = true\n",
    )
}

fn visualization_view_toml(options: &LocalViewOptions) -> String {
    generic_view_toml(
        options,
        "[filter]\nevent_kinds = []\nentity_kinds = []\n\n[display]\nrenderer = \"default\"\nshow_metadata = true\n",
    )
}

fn generic_view_toml(options: &LocalViewOptions, body: &str) -> String {
    format!(
        r#"schema_version = 1
id = "{}"
kind = "{}"
title = "{}"

{}"#,
        escape_toml_basic(&options.id()),
        options.kind.kind_value(),
        escape_toml_basic(&options.title),
        body
    )
}

fn create_dir(root: &Path, path: &Path) -> Result<(), LocalAuthoringError> {
    fs::create_dir_all(path).map_err(|source| LocalAuthoringError::Io {
        path: display_path(root, path),
        source,
    })
}

fn write_new_file(
    root: &Path,
    path: &Path,
    content: &str,
    force: bool,
) -> Result<(), LocalAuthoringError> {
    if path.exists() && !force {
        return Ok(());
    }

    fs::write(path, content).map_err(|source| LocalAuthoringError::Io {
        path: display_path(root, path),
        source,
    })
}

fn display_path(root: &Path, path: &Path) -> PathBuf {
    path.strip_prefix(root).unwrap_or(path).to_path_buf()
}

fn validate_slug(value: &str, label: &str) -> Result<(), LocalAuthoringError> {
    if value.trim().is_empty() {
        return Err(LocalAuthoringError::Validation {
            message: format!("{label} cannot be empty"),
        });
    }

    if value
        .chars()
        .any(|ch| ch.is_whitespace() || matches!(ch, '/' | '\\' | ':'))
    {
        return Err(LocalAuthoringError::Validation {
            message: format!("{label} `{value}` may not contain whitespace, slashes, or colons"),
        });
    }

    Ok(())
}

fn escape_toml_basic(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::local_authoring::{LocalSkeletonOptions, create_workspace_skeleton};

    #[test]
    fn creates_world_owned_views() {
        let temp_dir = std::env::temp_dir().join(format!(
            "kleio-views-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time")
                .as_nanos()
        ));
        create_workspace_skeleton(&temp_dir, &LocalSkeletonOptions::default()).expect("skeleton");
        let world_root = temp_dir.join("worlds/default");

        let path = create_local_view(
            &world_root,
            &LocalViewOptions {
                view_slug: "example-map".to_string(),
                title: "Example Map".to_string(),
                kind: LocalViewKind::Map,
                subject: None,
                force: false,
            },
        )
        .expect("map view");

        assert_eq!(
            path.strip_prefix(&world_root).unwrap(),
            Path::new("views/maps/example-map.toml")
        );

        let views = list_local_views(&world_root, None).expect("list views");
        assert!(views.iter().any(|view| view.kind == "map-view"));
        let maps = list_local_views(&world_root, Some(LocalViewKind::Map)).expect("list maps");
        assert_eq!(maps.len(), 1);
        assert_eq!(maps[0].id.as_deref(), Some("map:example-map"));

        fs::remove_dir_all(temp_dir).expect("remove temp dir");
    }
}
