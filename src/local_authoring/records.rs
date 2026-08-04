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

#[derive(Debug, Clone, PartialEq)]
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

#[derive(Debug, Clone, PartialEq)]
pub struct LocalEventOptions {
    pub event_slug: String,
    pub event_type: String,
    pub title: Option<String>,
    pub participants: Vec<String>,
    pub places: Vec<String>,
    pub location: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub time: Option<String>,
    pub date_precision: Option<String>,
    pub sources: Vec<String>,
    pub force: bool,
}

impl LocalEventOptions {
    pub fn id(&self) -> String {
        format!("event:{}", self.event_slug)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalRelationshipOptions {
    pub relationship_slug: String,
    pub title: String,
    pub relationship_kind: String,
    pub source: String,
    pub target: String,
    pub sources: Vec<String>,
    pub force: bool,
}

impl LocalRelationshipOptions {
    pub fn id(&self) -> String {
        format!("relationship:{}", self.relationship_slug)
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
    pub target: String,
    pub value: Option<String>,
    pub sources: Vec<String>,
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
    validate_slug(&options.event_type, "event type")?;
    for participant in &options.participants {
        validate_contextual_record_id(participant, "event participant")?;
    }
    for place in &options.places {
        validate_contextual_record_id(place, "event place")?;
    }
    for source in &options.sources {
        validate_contextual_record_id(source, "event source")?;
    }
    if options.latitude.is_some() != options.longitude.is_some() {
        return Err(LocalAuthoringError::Validation {
            message: "event latitude and longitude must be provided together".to_string(),
        });
    }
    let world_root = world_root.as_ref();
    let paths = WorldPaths::new(world_root);
    let dir = paths.event_type_dir(event_type_dir_name(&options.event_type));
    create_dir(world_root, &dir)?;
    let path = dir.join(format!("{}.md", options.event_slug));
    write_new_file(world_root, &path, &event_markdown(options), options.force)?;
    Ok(path)
}

pub fn create_local_relationship(
    world_root: impl AsRef<Path>,
    options: &LocalRelationshipOptions,
) -> Result<PathBuf, LocalAuthoringError> {
    validate_slug(&options.relationship_slug, "relationship slug")?;
    validate_slug(&options.relationship_kind, "relationship kind")?;
    validate_contextual_record_id(&options.source, "relationship source")?;
    validate_contextual_record_id(&options.target, "relationship target")?;
    for source_id in &options.sources {
        validate_contextual_record_id(source_id, "relationship source reference")?;
    }

    let world_root = world_root.as_ref();
    let paths = WorldPaths::new(world_root);
    create_dir(world_root, &paths.relationships_dir())?;
    let path = paths
        .relationships_dir()
        .join(format!("{}.toml", options.relationship_slug));
    write_new_file(
        world_root,
        &path,
        &relationship_toml(options),
        options.force,
    )?;
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
    validate_record_id(target_base_id(&options.target), "assertion target")?;
    for source in &options.sources {
        validate_contextual_record_id(source, "assertion source")?;
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
    let participants = toml_multiline_string_array(&options.participants);
    let places = toml_multiline_string_array(&options.places);
    let time = options
        .time
        .as_deref()
        .map(|time| format!("time = \"{}\"\n", escape_toml_basic(time)))
        .unwrap_or_default();
    let date_precision = options
        .date_precision
        .as_deref()
        .map(|precision| format!("date_precision = \"{}\"\n", escape_toml_basic(precision)))
        .unwrap_or_else(|| {
            options
                .time
                .as_deref()
                .map(infer_date_precision)
                .map(|precision| format!("date_precision = \"{precision}\"\n"))
                .unwrap_or_default()
        });
    let sources = toml_multiline_string_array(&options.sources);
    let inline_location = inline_location_toml(options);
    let title_line = options
        .title
        .as_deref()
        .filter(|title| !title.trim().is_empty())
        .map(|title| format!("title = \"{}\"\n", escape_toml_basic(title)))
        .unwrap_or_default();
    format!(
        r#"+++
schema_version = 1
id = "{}"
kind = "event"
type = "{}"
{}{}{}{}participants = {participants}
places = {places}
assertions = []
sources = {sources}
+++

# {}

Add event notes here. Connect entities through `participants`, places through `places`, and source-backed claims through `assertions`.
"#,
        escape_toml_basic(&options.id()),
        escape_toml_basic(&options.event_type),
        title_line,
        time,
        date_precision,
        inline_location,
        options.title.as_deref().unwrap_or(&options.event_type)
    )
}

fn relationship_toml(options: &LocalRelationshipOptions) -> String {
    let sources = toml_string_array(&options.sources);
    format!(
        r#"schema_version = 1
id = "{}"
kind = "relationship"
title = "{}"
relationship = "{}"
source = "{}"
target = "{}"
sources = {sources}
"#,
        escape_toml_basic(&options.id()),
        escape_toml_basic(&options.title),
        escape_toml_basic(&options.relationship_kind),
        escape_toml_basic(&options.source),
        escape_toml_basic(&options.target),
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
    let sources = toml_multiline_string_array(&options.sources);
    format!(
        r#"+++
schema_version = 1
id = "{}"
kind = "{}"
target = "{}"
{}sources = {sources}
confidence = "medium"
+++

Optional reasoning, transcription notes, uncertainty notes, or conflict notes.
"#,
        escape_toml_basic(&options.id()),
        escape_toml_basic(&options.assertion_kind),
        escape_toml_basic(&options.target),
        options
            .value
            .as_deref()
            .map(|value| format!("value = \"{}\"\n", escape_toml_basic(value)))
            .unwrap_or_default(),
    )
}

fn target_base_id(target: &str) -> &str {
    target
        .split_once('#')
        .map(|(base, _)| base)
        .unwrap_or(target)
}

fn event_type_dir_name(event_type: &str) -> &str {
    match event_type {
        "birth" => "births",
        "death" => "deaths",
        "residence" => "residences",
        "marriage" => "marriages",
        "divorce" => "divorces",
        "adoption" => "adoptions",
        "education" => "education",
        "military-service" => "military-service",
        "immigration" => "migrations",
        "emigration" => "migrations",
        "migration" => "migrations",
        "naturalization" => "naturalizations",
        "census" => "census",
        "name-change" => "name-changes",
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

fn validate_record_id(value: &str, label: &str) -> Result<(), LocalAuthoringError> {
    if value.trim().is_empty() {
        return Err(LocalAuthoringError::Validation {
            message: format!("{label} cannot be empty"),
        });
    }

    if value.chars().any(char::is_whitespace) {
        return Err(LocalAuthoringError::Validation {
            message: format!("{label} `{value}` may not contain whitespace"),
        });
    }

    Ok(())
}

fn inline_location_toml(options: &LocalEventOptions) -> String {
    match (
        options.location.as_deref(),
        options.latitude,
        options.longitude,
    ) {
        (Some(label), Some(latitude), Some(longitude)) => format!(
            "[location]\nlabel = \"{}\"\nlatitude = {}\nlongitude = {}\n",
            escape_toml_basic(label),
            latitude,
            longitude
        ),
        (Some(label), None, None) => format!("location = \"{}\"\n", escape_toml_basic(label)),
        (None, Some(latitude), Some(longitude)) => {
            format!(
                "[location]\nlatitude = {}\nlongitude = {}\n",
                latitude, longitude
            )
        }
        _ => String::new(),
    }
}

fn infer_date_precision(value: &str) -> &'static str {
    if value.contains(':') {
        "minute"
    } else if value.matches('-').count() >= 2 {
        "day"
    } else if value.matches('-').count() == 1 {
        "month"
    } else if !value.trim().is_empty() {
        "year"
    } else {
        "unknown"
    }
}

fn validate_contextual_record_id(value: &str, label: &str) -> Result<(), LocalAuthoringError> {
    validate_slug(value, label).or_else(|_| validate_record_id(value, label))
}

fn toml_multiline_string_array(values: &[String]) -> String {
    if values.is_empty() {
        return "[]".to_string();
    }

    let mut text = String::from("[\n");
    for value in values {
        text.push_str(&format!("  \"{}\",\n", escape_toml_basic(value)));
    }
    text.push(']');
    text
}

fn toml_string_array(values: &[String]) -> String {
    let values = values
        .iter()
        .map(|value| format!("\"{}\"", escape_toml_basic(value)))
        .collect::<Vec<_>>()
        .join(", ");
    format!("[{values}]")
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
                event_type: "observation".to_string(),
                title: Some("Example Observation".to_string()),
                participants: Vec::new(),
                places: Vec::new(),
                location: None,
                latitude: None,
                longitude: None,
                time: None,
                date_precision: None,
                sources: Vec::new(),
                force: false,
            },
        )
        .expect("event");
        let event_text = fs::read_to_string(&event).expect("event text");
        assert!(event_text.contains("kind = \"event\""));
        assert!(event_text.contains("type = \"observation\""));
        assert!(event_text.contains("title = \"Example Observation\""));
        assert!(event_text.contains("participants = []"));
        assert!(event_text.contains("places = []"));

        let event_with_details = create_local_event(
            &world_root,
            &LocalEventOptions {
                event_slug: "birth-example-person".to_string(),
                event_type: "birth".to_string(),
                title: None,
                participants: vec!["example-person".to_string()],
                places: vec!["example-place".to_string()],
                location: Some("Example Town".to_string()),
                latitude: Some(12.345),
                longitude: Some(-67.89),
                time: Some("1900-01-01 07:18".to_string()),
                date_precision: None,
                sources: vec!["example-source".to_string()],
                force: false,
            },
        )
        .expect("detailed event");
        let event_with_details_text =
            fs::read_to_string(&event_with_details).expect("detailed event text");
        assert!(event_with_details_text.contains("type = \"birth\""));
        assert!(event_with_details_text.contains("time = \"1900-01-01 07:18\""));
        assert!(event_with_details_text.contains("date_precision = \"minute\""));
        assert!(event_with_details_text.contains("participants = ["));
        assert!(event_with_details_text.contains("\"example-person\""));
        assert!(event_with_details_text.contains("places = ["));
        assert!(event_with_details_text.contains("\"example-place\""));
        assert!(event_with_details_text.contains("sources = ["));
        assert!(event_with_details_text.contains("\"example-source\""));
        assert!(event_with_details_text.contains("[location]"));
        assert!(event_with_details_text.contains("label = \"Example Town\""));
        assert!(event_with_details_text.contains("latitude = 12.345"));
        assert!(event_with_details_text.contains("longitude = -67.89"));
        assert!(!event_with_details_text.contains("title ="));

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
        let relationship = create_local_relationship(
            &world_root,
            &LocalRelationshipOptions {
                relationship_slug: "example-association".to_string(),
                title: "Example association".to_string(),
                relationship_kind: "associate".to_string(),
                source: "person:example-person".to_string(),
                target: "person:example-person".to_string(),
                sources: vec!["source:example-source".to_string()],
                force: false,
            },
        )
        .expect("relationship");
        let assertion = create_local_assertion(
            &world_root,
            &LocalAssertionOptions {
                assertion_slug: "example-claim".to_string(),
                assertion_kind: "identity".to_string(),
                target: "person:example-person#name".to_string(),
                value: Some("Example Person".to_string()),
                sources: vec!["example-source".to_string()],
                force: false,
            },
        )
        .expect("assertion");

        let assertion_text = fs::read_to_string(&assertion).expect("assertion text");
        assert!(assertion_text.contains("sources = ["));
        assert!(assertion_text.contains("\"example-source\""));

        let support_assertion = create_local_assertion(
            &world_root,
            &LocalAssertionOptions {
                assertion_slug: "example-event-support".to_string(),
                assertion_kind: "event-support".to_string(),
                target: "event:example-observation#date".to_string(),
                value: None,
                sources: Vec::new(),
                force: false,
            },
        )
        .expect("support assertion");
        let support_text = fs::read_to_string(&support_assertion).expect("support assertion text");
        assert!(!support_text.contains("value ="));

        assert_eq!(
            relationship.strip_prefix(&world_root).unwrap(),
            Path::new("relationships/example-association.toml")
        );
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
