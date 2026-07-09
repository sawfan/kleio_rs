use std::fs;
use std::path::{Path, PathBuf};

use super::{LocalAuthoringError, WorldPaths};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalEntityKind {
    Person,
    Place,
    Organization,
    Object,
    Concept,
}

impl LocalEntityKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Person => "person",
            Self::Place => "place",
            Self::Organization => "organization",
            Self::Object => "object",
            Self::Concept => "concept",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalEntityOptions {
    pub slug: String,
    pub title: String,
    pub kind: LocalEntityKind,
    pub force: bool,
}

impl LocalEntityOptions {
    pub fn id(&self) -> String {
        format!("{}:{}", self.kind.as_str(), self.slug)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalEventOptions {
    pub event_slug: String,
    pub event_kind: String,
    pub title: String,
    pub force: bool,
}

impl LocalEventOptions {
    pub fn id(&self) -> String {
        format!("event:{}", self.event_slug)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalSourceOptions {
    pub source_slug: String,
    pub title: String,
    pub source_kind: String,
    pub force: bool,
}

impl LocalSourceOptions {
    pub fn id(&self) -> String {
        format!("source:{}", self.source_slug)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalAssertionOptions {
    pub assertion_slug: String,
    pub assertion_kind: String,
    pub subject: String,
    pub predicate: String,
    pub value: String,
    pub force: bool,
}

impl LocalAssertionOptions {
    pub fn id(&self) -> String {
        format!("assertion:{}", self.assertion_slug)
    }
}

pub fn create_local_entity(
    world_root: impl AsRef<Path>,
    options: &LocalEntityOptions,
) -> Result<PathBuf, LocalAuthoringError> {
    validate_slug(&options.slug, "entity slug")?;
    let world_root = world_root.as_ref();
    let paths = WorldPaths::new(world_root);
    let dir = match options.kind {
        LocalEntityKind::Person => paths.people_dir(),
        LocalEntityKind::Place => paths.places_dir(),
        LocalEntityKind::Organization => paths.organizations_dir(),
        LocalEntityKind::Object => paths.objects_dir(),
        LocalEntityKind::Concept => paths.concepts_dir(),
    };
    create_dir(world_root, &dir)?;
    let path = dir.join(format!("{}.md", options.slug));
    write_new_file(world_root, &path, &entity_markdown(options), options.force)?;
    Ok(path)
}

pub fn create_local_event(
    world_root: impl AsRef<Path>,
    options: &LocalEventOptions,
) -> Result<PathBuf, LocalAuthoringError> {
    validate_slug(&options.event_slug, "event slug")?;
    validate_slug(&options.event_kind, "event kind")?;
    let world_root = world_root.as_ref();
    let paths = WorldPaths::new(world_root);
    let dir = paths.event_kind_dir(event_kind_dir_name(&options.event_kind));
    create_dir(world_root, &dir)?;
    let path = dir.join(format!("{}.md", options.event_slug));
    write_new_file(world_root, &path, &event_markdown(options), options.force)?;
    Ok(path)
}

pub fn create_local_source(
    world_root: impl AsRef<Path>,
    options: &LocalSourceOptions,
) -> Result<PathBuf, LocalAuthoringError> {
    validate_slug(&options.source_slug, "source slug")?;
    validate_slug(&options.source_kind, "source kind")?;
    let world_root = world_root.as_ref();
    let paths = WorldPaths::new(world_root);
    create_dir(world_root, &paths.sources_dir())?;
    let path = paths
        .sources_dir()
        .join(format!("{}.md", options.source_slug));
    write_new_file(world_root, &path, &source_markdown(options), options.force)?;
    Ok(path)
}

pub fn create_local_assertion(
    world_root: impl AsRef<Path>,
    options: &LocalAssertionOptions,
) -> Result<PathBuf, LocalAuthoringError> {
    validate_slug(&options.assertion_slug, "assertion slug")?;
    validate_slug(&options.assertion_kind, "assertion kind")?;
    if options.subject.trim().is_empty() || options.predicate.trim().is_empty() {
        return Err(LocalAuthoringError::Validation {
            message: "assertion subject and predicate cannot be empty".to_string(),
        });
    }

    let world_root = world_root.as_ref();
    let paths = WorldPaths::new(world_root);
    create_dir(world_root, &paths.assertions_dir())?;
    let path = paths
        .assertions_dir()
        .join(format!("{}.md", options.assertion_slug));
    write_new_file(
        world_root,
        &path,
        &assertion_markdown(options),
        options.force,
    )?;
    Ok(path)
}

fn entity_markdown(options: &LocalEntityOptions) -> String {
    format!(
        r#"+++
schema_version = 1
id = "{}"
kind = "{}"
primary_name = "{}"
+++

# {}

Add notes about this {} here.
"#,
        escape_toml_basic(&options.id()),
        options.kind.as_str(),
        escape_toml_basic(&options.title),
        options.title,
        options.kind.as_str()
    )
}

fn event_markdown(options: &LocalEventOptions) -> String {
    format!(
        r#"+++
schema_version = 1
id = "{}"
kind = "{}"
title = "{}"
participants = []
places = []
assertions = []
+++

# {}

Add event notes here. Connect entities through `participants`, places through `places`, and source-backed claims through `assertions`.
"#,
        escape_toml_basic(&options.id()),
        escape_toml_basic(&options.event_kind),
        escape_toml_basic(&options.title),
        options.title
    )
}

fn source_markdown(options: &LocalSourceOptions) -> String {
    format!(
        r#"+++
schema_version = 1
id = "{}"
kind = "{}"
title = "{}"
media = []
+++

Optional citation, transcription, provenance, or notes.
"#,
        escape_toml_basic(&options.id()),
        escape_toml_basic(&options.source_kind),
        escape_toml_basic(&options.title)
    )
}

fn assertion_markdown(options: &LocalAssertionOptions) -> String {
    format!(
        r#"+++
schema_version = 1
id = "{}"
kind = "{}"
subject = "{}"
predicate = "{}"
value = "{}"
sources = []
confidence = "medium"
+++

Optional reasoning, transcription notes, uncertainty notes, or conflict notes.
"#,
        escape_toml_basic(&options.id()),
        escape_toml_basic(&options.assertion_kind),
        escape_toml_basic(&options.subject),
        escape_toml_basic(&options.predicate),
        escape_toml_basic(&options.value)
    )
}

fn event_kind_dir_name(event_kind: &str) -> &str {
    match event_kind {
        "birth" => "births",
        "death" => "deaths",
        "residence" => "residences",
        "marriage" => "marriages",
        "migration" => "migrations",
        "observation" => "observations",
        "moment" => "moments",
        _ => "other",
    }
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
    fn creates_world_owned_records() {
        let temp_dir = std::env::temp_dir().join(format!(
            "kleio-records-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time")
                .as_nanos()
        ));
        create_workspace_skeleton(&temp_dir, &LocalSkeletonOptions::default()).expect("skeleton");
        let world_root = temp_dir.join("worlds/default");

        let place = create_local_entity(
            &world_root,
            &LocalEntityOptions {
                slug: "example-place".to_string(),
                title: "Example Place".to_string(),
                kind: LocalEntityKind::Place,
                force: false,
            },
        )
        .expect("place");
        let event = create_local_event(
            &world_root,
            &LocalEventOptions {
                event_slug: "example-observation".to_string(),
                event_kind: "observation".to_string(),
                title: "Example Observation".to_string(),
                force: false,
            },
        )
        .expect("event");
        let source = create_local_source(
            &world_root,
            &LocalSourceOptions {
                source_slug: "example-source".to_string(),
                title: "Example Source".to_string(),
                source_kind: "note".to_string(),
                force: false,
            },
        )
        .expect("source");
        let assertion = create_local_assertion(
            &world_root,
            &LocalAssertionOptions {
                assertion_slug: "example-claim".to_string(),
                assertion_kind: "identity".to_string(),
                subject: "person:example-person".to_string(),
                predicate: "has_name".to_string(),
                value: "Example Person".to_string(),
                force: false,
            },
        )
        .expect("assertion");

        assert_eq!(
            place.strip_prefix(&world_root).unwrap(),
            Path::new("entities/places/example-place.md")
        );
        assert_eq!(
            event.strip_prefix(&world_root).unwrap(),
            Path::new("events/observations/example-observation.md")
        );
        assert_eq!(
            source.strip_prefix(&world_root).unwrap(),
            Path::new("sources/example-source.md")
        );
        assert_eq!(
            assertion.strip_prefix(&world_root).unwrap(),
            Path::new("assertions/example-claim.md")
        );

        fs::remove_dir_all(temp_dir).expect("remove temp dir");
    }
}
