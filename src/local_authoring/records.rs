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
    pub subject: Option<String>,
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
    pub title: Option<String>,
    pub relationship_kind: String,
    pub parent_role: Option<String>,
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
    pub confidence: Option<String>,
    pub note: Option<String>,
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
    if let Some(parent_role) = &options.parent_role {
        validate_slug(parent_role, "relationship parent role")?;
    }
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
title = "{}"
+++

# {}

{}"#,
        escape_toml_basic(&options.id()),
        options.kind.as_str(),
        escape_toml_basic(&options.title),
        options.title,
        entity_body(options.kind)
    )
}

fn entity_body(kind: LocalEntityKind) -> &'static str {
    match kind {
        LocalEntityKind::Person => {
            "## Notes

Write biographical notes, memories, research notes, or unresolved questions here.
Keep dated life facts in `events/` so timeline and tree views can project them.

## Known details

- Preferred name:
- Other names:
- Important places:
- Open questions:
"
        }
        LocalEntityKind::Place => {
            "## Notes

Describe this place, including memories, research notes, alternate names, or
uncertainty about its identity.

## Known details

- Alternate names:
- Larger area or jurisdiction:
- Coordinates or map reference:
- Open questions:
"
        }
        LocalEntityKind::Organization => {
            "## Notes

Describe this organization, its role, relevant dates, and open research questions.

## Known details

- Alternate names:
- Important people:
- Important places:
- Open questions:
"
        }
        LocalEntityKind::Object => {
            "## Notes

Describe this object, why it matters, where it came from, and where it is now.

## Known details

- Owner or custodian:
- Date or era:
- Related people or places:
- Open questions:
"
        }
        LocalEntityKind::Concept => {
            "## Notes

Describe this concept or topic and how it connects to people, events, places, or
sources in this world.

## Known details

- Related people:
- Related events:
- Related sources:
- Open questions:
"
        }
    }
}

fn event_markdown(options: &LocalEventOptions) -> String {
    let subject = options
        .subject
        .as_deref()
        .map(|subject| format!("subject = \"{}\"\n", escape_toml_basic(subject)))
        .unwrap_or_default();
    let participants = if options.participants.is_empty() {
        String::new()
    } else {
        format!(
            "participants = {}\n",
            toml_multiline_string_array(&options.participants)
        )
    };
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
{}{}{}{}{participants}places = {places}
assertions = []
sources = {sources}
{}+++

# {}

## Notes

Describe what happened, what is known, and what is uncertain.

## Evidence

List source notes, transcriptions, or follow-up tasks here. Use `sources` and
`assertions` in the frontmatter for structured evidence links.
"#,
        escape_toml_basic(&options.id()),
        escape_toml_basic(&options.event_type),
        title_line,
        time,
        date_precision,
        subject,
        inline_location,
        options.title.as_deref().unwrap_or(&options.event_type)
    )
}

fn relationship_toml(options: &LocalRelationshipOptions) -> String {
    let title = options
        .title
        .as_deref()
        .map(|title| format!("title = \"{}\"\n", escape_toml_basic(title)))
        .unwrap_or_default();
    let parent_role = options
        .parent_role
        .as_deref()
        .map(|parent_role| format!("parent_role = \"{}\"\n", escape_toml_basic(parent_role)))
        .unwrap_or_default();
    let sources = toml_string_array(&options.sources);
    format!(
        r#"schema_version = 1
relationship = "{}"
{parent_role}source = "{}"
target = "{}"
sources = {sources}
{title}"#,
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

# {}

## Citation

Add a concise citation or source description here.

## Notes

Record provenance, access notes, or context for this source.

## Transcript

Add transcribed text or image/PDF notes when useful.
"#,
        escape_toml_basic(&options.id()),
        escape_toml_basic(&options.source_kind),
        escape_toml_basic(&options.title),
        options.title
    )
}

fn assertion_markdown(options: &LocalAssertionOptions) -> String {
    let sources = toml_multiline_string_array(&options.sources);
    let confidence = options.confidence.as_deref().unwrap_or("medium");
    let note = options
        .note
        .as_deref()
        .map(|note| format!("note = \"{}\"\n", escape_toml_basic(note)))
        .unwrap_or_default();
    format!(
        r#"+++
schema_version = 1
id = "{}"
kind = "{}"
target = "{}"
{}sources = {sources}
confidence = "{}"
{}+++

## Reasoning

Explain why this claim is supported, uncertain, or in conflict.

## Source notes

Add transcription notes, quotations, or follow-up tasks here.
"#,
        escape_toml_basic(&options.id()),
        escape_toml_basic(&options.assertion_kind),
        escape_toml_basic(&options.target),
        options
            .value
            .as_deref()
            .map(|value| format!("value = \"{}\"\n", escape_toml_basic(value)))
            .unwrap_or_default(),
        escape_toml_basic(confidence),
        note,
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
                subject: None,
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
        assert!(!event_text.contains("participants ="));
        assert!(event_text.contains("places = []"));

        let event_with_details = create_local_event(
            &world_root,
            &LocalEventOptions {
                event_slug: "birth-example-person".to_string(),
                event_type: "birth".to_string(),
                title: None,
                subject: None,
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
                title: Some("Example association".to_string()),
                relationship_kind: "associate".to_string(),
                parent_role: None,
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
                confidence: None,
                note: None,
                force: false,
            },
        )
        .expect("assertion");

        let relationship_text = fs::read_to_string(&relationship).expect("relationship text");
        assert!(!relationship_text.contains("id ="));
        assert!(!relationship_text.contains("kind = \"relationship\""));
        assert!(relationship_text.contains("title = \"Example association\""));

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
                confidence: Some("low".to_string()),
                note: Some("Example source note.".to_string()),
                force: false,
            },
        )
        .expect("support assertion");
        let support_text = fs::read_to_string(&support_assertion).expect("support assertion text");
        assert!(!support_text.contains("value ="));
        assert!(support_text.contains("confidence = \"low\""));
        assert!(support_text.contains("note = \"Example source note.\""));

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
