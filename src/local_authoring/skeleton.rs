use std::fs;
use std::path::Path;

use super::{
    DEFAULT_WORLD_SLUG, LocalAuthoringError, WorkspaceConfig, WorkspacePaths, WorldConfig,
    WorldPaths,
};

pub const DEFAULT_PROJECT_ID: &str = DEFAULT_WORLD_SLUG;
pub const DEFAULT_PERSON_SLUG: &str = "example-person";
pub const DEFAULT_PLACE_ID: &str = "place:unknown-birth-place";
pub const DEFAULT_SOURCE_ID: &str = "source:personal-knowledge";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalPersonOptions {
    pub person_slug: String,
    pub person_name: String,
    pub birth_date: Option<String>,
    pub create_birth_event: bool,
    pub force: bool,
}

impl LocalPersonOptions {
    pub fn person_id(&self) -> String {
        format!("person:{}", self.person_slug)
    }

    pub fn birth_event_id(&self) -> String {
        format!("event:birth-{}", self.person_slug)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalBirthEventOptions {
    pub person_slug: String,
    pub person_name: String,
    pub birth_date: Option<String>,
    pub force: bool,
}

impl LocalBirthEventOptions {
    pub fn person_id(&self) -> String {
        format!("person:{}", self.person_slug)
    }

    pub fn birth_event_id(&self) -> String {
        format!("event:birth-{}", self.person_slug)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalSkeletonOptions {
    pub project_id: String,
    pub title: String,
    pub person_slug: String,
    pub person_name: String,
    pub birth_date: Option<String>,
    pub force: bool,
}

impl Default for LocalSkeletonOptions {
    fn default() -> Self {
        Self {
            project_id: DEFAULT_PROJECT_ID.to_string(),
            title: "Default world".to_string(),
            person_slug: DEFAULT_PERSON_SLUG.to_string(),
            person_name: "Example Person".to_string(),
            birth_date: None,
            force: false,
        }
    }
}

impl LocalSkeletonOptions {
    pub fn world_slug(&self) -> &str {
        &self.project_id
    }

    pub fn world_id(&self) -> String {
        format!("world:{}", self.world_slug())
    }

    pub fn person_id(&self) -> String {
        format!("person:{}", self.person_slug)
    }

    pub fn birth_event_id(&self) -> String {
        format!("event:birth-{}", self.person_slug)
    }
}

pub fn create_local_person(
    world_root: impl AsRef<Path>,
    options: &LocalPersonOptions,
) -> Result<(), LocalAuthoringError> {
    validate_slug(&options.person_slug, "person slug")?;
    let world_root = world_root.as_ref();
    let paths = WorldPaths::new(world_root);
    create_dir(world_root, world_root)?;
    create_dir(world_root, &paths.people_dir())?;
    write_new_file(
        world_root,
        &paths
            .people_dir()
            .join(format!("{}.md", options.person_slug)),
        &person_markdown(&PersonTemplate {
            person_id: options.person_id(),
            person_name: options.person_name.clone(),
        }),
        options.force,
    )?;

    if options.create_birth_event {
        create_local_birth_event(
            world_root,
            &LocalBirthEventOptions {
                person_slug: options.person_slug.clone(),
                person_name: options.person_name.clone(),
                birth_date: options.birth_date.clone(),
                force: options.force,
            },
        )?;
    }

    Ok(())
}

pub fn create_local_birth_event(
    world_root: impl AsRef<Path>,
    options: &LocalBirthEventOptions,
) -> Result<(), LocalAuthoringError> {
    validate_slug(&options.person_slug, "person slug")?;
    let world_root = world_root.as_ref();
    let paths = WorldPaths::new(world_root);
    create_dir(world_root, world_root)?;
    create_dir(world_root, &paths.births_dir())?;
    create_dir(world_root, &paths.places_dir())?;
    create_dir(world_root, &paths.sources_dir())?;

    write_new_file(
        world_root,
        &paths.places_dir().join("unknown-birth-place.toml"),
        &unknown_place_toml(),
        false,
    )?;
    write_new_file(
        world_root,
        &paths.sources_dir().join("personal-knowledge.md"),
        &personal_knowledge_source_markdown(),
        false,
    )?;
    let birth_template = BirthEventTemplate {
        event_id: options.birth_event_id(),
        person_id: options.person_id(),
        person_name: options.person_name.clone(),
        birth_date: options.birth_date.clone(),
    };
    write_new_file(
        world_root,
        &paths.births_dir().join(format!(
            "{}-birth-{}.md",
            options.birth_date.as_deref().unwrap_or("unknown-date"),
            options.person_slug
        )),
        &birth_event_markdown(&birth_template),
        options.force,
    )?;

    Ok(())
}

pub fn create_workspace_skeleton(
    workspace_root: impl AsRef<Path>,
    options: &LocalSkeletonOptions,
) -> Result<(), LocalAuthoringError> {
    validate_slug(options.world_slug(), "world slug")?;
    let workspace_root = workspace_root.as_ref();
    let workspace_paths = WorkspacePaths::new(workspace_root);
    create_dir(workspace_root, workspace_root)?;
    create_dir(workspace_root, &workspace_paths.worlds_dir())?;

    write_new_file(
        workspace_root,
        &workspace_paths.config(),
        &workspace_toml(options)?,
        options.force,
    )?;
    create_world_skeleton(workspace_paths.world(options.world_slug()).root(), options)
}

pub fn create_world_layout(
    world_root: impl AsRef<Path>,
    options: &LocalSkeletonOptions,
) -> Result<(), LocalAuthoringError> {
    validate_slug(options.world_slug(), "world slug")?;
    let world_root = world_root.as_ref();
    let paths = WorldPaths::new(world_root);
    create_dir(world_root, world_root)?;

    for dir in [
        paths.people_dir(),
        paths.places_dir(),
        paths.organizations_dir(),
        paths.objects_dir(),
        paths.concepts_dir(),
        paths.births_dir(),
        paths.deaths_dir(),
        paths.residences_dir(),
        paths.marriages_dir(),
        paths.migrations_dir(),
        paths.observations_dir(),
        paths.collections_dir(),
        paths.moments_dir(),
        paths.other_events_dir(),
        paths.assertions_dir(),
        paths.relationships_dir(),
        paths.sources_dir(),
        paths.media_people_dir(),
        paths.media_places_dir(),
        paths.media_sources_dir(),
        paths.media_events_dir(),
        paths.gedcom_imports_dir(),
        paths.wikidata_imports_dir(),
        paths.csv_imports_dir(),
        paths.timeline_views_dir(),
        paths.tree_views_dir(),
        paths.map_views_dir(),
        paths.calendar_views_dir(),
        paths.visualization_views_dir(),
        paths.component_schemas_dir(),
        paths.bundle_schemas_dir(),
        paths.event_schemas_dir(),
        paths.view_schemas_dir(),
        paths.vocab_schemas_dir(),
        paths.importer_systems_dir(),
        paths.compiler_systems_dir(),
        paths.validator_systems_dir(),
        paths.renderer_systems_dir(),
        paths.build_dir(),
    ] {
        create_dir(world_root, &dir)?;
    }

    write_new_file(
        world_root,
        &paths.config(),
        &world_toml(options)?,
        options.force,
    )?;
    write_new_file(
        world_root,
        &paths.vocab_schemas_dir().join("event-kinds.toml"),
        &event_kinds_toml(),
        options.force,
    )?;
    write_new_file(
        world_root,
        &paths.vocab_schemas_dir().join("participant-roles.toml"),
        &participant_roles_toml(),
        options.force,
    )?;
    write_new_file(
        world_root,
        &paths.gedcom_imports_dir().join("README.md"),
        &gedcom_imports_readme(),
        options.force,
    )?;
    write_schema_seed_files(world_root, &paths, options.force)?;

    Ok(())
}

pub fn create_world_skeleton(
    world_root: impl AsRef<Path>,
    options: &LocalSkeletonOptions,
) -> Result<(), LocalAuthoringError> {
    validate_slug(&options.person_slug, "person slug")?;
    let world_root = world_root.as_ref();
    let paths = WorldPaths::new(world_root);
    create_world_layout(world_root, options)?;

    write_new_file(
        world_root,
        &paths
            .people_dir()
            .join(format!("{}.md", options.person_slug)),
        &person_markdown(&PersonTemplate {
            person_id: options.person_id(),
            person_name: options.person_name.clone(),
        }),
        options.force,
    )?;
    create_local_birth_event(
        world_root,
        &LocalBirthEventOptions {
            person_slug: options.person_slug.clone(),
            person_name: options.person_name.clone(),
            birth_date: options.birth_date.clone(),
            force: options.force,
        },
    )?;
    write_new_file(
        world_root,
        &paths.collections_dir().join("example-life.toml"),
        &life_collection_toml(options),
        options.force,
    )?;
    write_new_file(
        world_root,
        &paths.timeline_views_dir().join("example-life.toml"),
        &timeline_view_toml(options),
        options.force,
    )?;
    write_new_file(
        world_root,
        &paths.tree_views_dir().join("main-family-tree.toml"),
        &tree_view_toml(options),
        options.force,
    )?;

    Ok(())
}

pub fn create_local_skeleton(
    root: impl AsRef<Path>,
    options: &LocalSkeletonOptions,
) -> Result<(), LocalAuthoringError> {
    create_workspace_skeleton(root, options)
}

fn write_schema_seed_files(
    world_root: &Path,
    paths: &WorldPaths,
    force: bool,
) -> Result<(), LocalAuthoringError> {
    for (file_name, id, component_name, description) in [
        (
            "identity.toml",
            "component:identity",
            "Identity",
            "Stable world-scoped identity for an entity, event, or view.",
        ),
        (
            "primary-name.toml",
            "component:primary-name",
            "PrimaryName",
            "Human-readable primary display name.",
        ),
        (
            "participants.toml",
            "component:participants",
            "Participants",
            "Entity references and roles attached to an event.",
        ),
        (
            "source-links.toml",
            "component:source-links",
            "SourceLinks",
            "Assertion and source references attached to compiled records.",
        ),
    ] {
        write_new_file(
            world_root,
            &paths.component_schemas_dir().join(file_name),
            &component_schema_toml(id, component_name, description),
            force,
        )?;
    }

    write_new_file(
        world_root,
        &paths.bundle_schemas_dir().join("person.toml"),
        &bundle_schema_toml(
            "bundle:person",
            &[
                "component:identity",
                "component:primary-name",
                "component:source-links",
            ],
        ),
        force,
    )?;
    write_new_file(
        world_root,
        &paths.bundle_schemas_dir().join("birth-event.toml"),
        &bundle_schema_toml(
            "bundle:birth-event",
            &[
                "component:identity",
                "component:participants",
                "component:source-links",
            ],
        ),
        force,
    )?;

    Ok(())
}

fn component_schema_toml(id: &str, component_name: &str, description: &str) -> String {
    format!(
        r#"schema_version = 1
id = "{}"
kind = "ecs-component"
name = "{}"
description = "{}"
"#,
        escape_toml_basic(id),
        escape_toml_basic(component_name),
        escape_toml_basic(description)
    )
}

fn bundle_schema_toml(id: &str, components: &[&str]) -> String {
    let components = components
        .iter()
        .map(|component| format!("  \"{}\"", escape_toml_basic(component)))
        .collect::<Vec<_>>()
        .join(",\n");
    format!(
        r#"schema_version = 1
id = "{}"
kind = "ecs-bundle"

components = [
{}
]
"#,
        escape_toml_basic(id),
        components
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

fn display_path(root: &Path, path: &Path) -> std::path::PathBuf {
    path.strip_prefix(root).unwrap_or(path).to_path_buf()
}

fn workspace_toml(options: &LocalSkeletonOptions) -> Result<String, LocalAuthoringError> {
    toml::to_string_pretty(&WorkspaceConfig::with_default_world(
        options.world_slug(),
        &options.title,
    ))
    .map_err(|source| LocalAuthoringError::TomlSerialize {
        path: WorkspacePaths::new(".").config(),
        source,
    })
}

fn world_toml(options: &LocalSkeletonOptions) -> Result<String, LocalAuthoringError> {
    toml::to_string_pretty(&WorldConfig::new(options.world_slug(), &options.title)).map_err(
        |source| LocalAuthoringError::TomlSerialize {
            path: WorldPaths::new(".").config(),
            source,
        },
    )
}

struct PersonTemplate {
    person_id: String,
    person_name: String,
}

struct BirthEventTemplate {
    event_id: String,
    person_id: String,
    person_name: String,
    birth_date: Option<String>,
}

fn person_markdown(options: &PersonTemplate) -> String {
    let (given, family) = split_display_name(&options.person_name);
    format!(
        r#"+++
schema_version = 1
id = "{}"
kind = "person"
primary_name = "{}"
tags = ["starter"]
related = []

[names.primary]
full = "{}"
given = "{}"
family = "{}"
+++

# {}

Add biographical notes here. Keep concrete life facts in `events/` so timeline and tree views can project source-backed world data.
"#,
        escape_toml_basic(&options.person_id),
        escape_toml_basic(&options.person_name),
        escape_toml_basic(&options.person_name),
        escape_toml_basic(&given),
        escape_toml_basic(&family),
        options.person_name
    )
}

enum BirthSupportKind {
    Date,
    Time,
}

impl BirthSupportKind {
    fn target_fragment(&self) -> &'static str {
        match self {
            Self::Date => "date",
            Self::Time => "time",
        }
    }

    fn confidence(&self) -> &'static str {
        match self {
            Self::Date => "high",
            Self::Time => "low",
        }
    }

    fn note(&self) -> &'static str {
        match self {
            Self::Date => {
                "Optional reasoning, transcription notes, uncertainty notes, or conflict notes about the birth date. Keep the date value on the linked event so it has one editable source of truth."
            }
            Self::Time => {
                "Optional reasoning, transcription notes, uncertainty notes, or conflict notes about the birth time. Keep the time value on the linked event so it has one editable source of truth."
            }
        }
    }
}

fn birth_event_markdown(options: &BirthEventTemplate) -> String {
    let date_line = options
        .birth_date
        .as_ref()
        .map(|date| {
            let precision = if birth_input_has_time(date) {
                "minute"
            } else {
                "day"
            };
            format!(
                "time = \"{}\"\ndate_precision = \"{}\"\n",
                escape_toml_basic(date),
                precision,
            )
        })
        .unwrap_or_else(|| "date_precision = \"unknown\"\n".to_string());
    let assertions = birth_assertions_toml(options);

    format!(
        r#"+++
schema_version = 1
id = "{}"
kind = "birth"
title = "{}"
{}participants = [
  {{ entity = "{}", role = "subject" }},
]
places = [
  {{ entity = "{}", role = "birthplace" }},
]
{}
+++

# Birth of {}

Replace the placeholder place/source with the best available evidence when you have it.
"#,
        escape_toml_basic(&options.event_id),
        escape_toml_basic(&format!("Birth of {}", options.person_name)),
        date_line,
        escape_toml_basic(&options.person_id),
        DEFAULT_PLACE_ID,
        assertions,
        options.person_name
    )
}

fn birth_assertions_toml(options: &BirthEventTemplate) -> String {
    birth_support_kinds(options)
        .into_iter()
        .map(|support_kind| {
            format!(
                r##"
[[assertions]]
target = "#{}"
sources = ["{}"]
confidence = "{}"
note = "{}"
"##,
                support_kind.target_fragment(),
                DEFAULT_SOURCE_ID,
                support_kind.confidence(),
                escape_toml_basic(support_kind.note()),
            )
        })
        .collect::<Vec<_>>()
        .join("")
}

fn birth_support_kinds(options: &BirthEventTemplate) -> Vec<BirthSupportKind> {
    let Some(birth_date) = options.birth_date.as_deref() else {
        return Vec::new();
    };

    let mut kinds = vec![BirthSupportKind::Date];
    if birth_input_has_time(birth_date) {
        kinds.push(BirthSupportKind::Time);
    }
    kinds
}

fn birth_input_has_time(value: &str) -> bool {
    value.trim().contains(':')
}

fn unknown_place_toml() -> String {
    r#"schema_version = 1
id = "place:unknown-birth-place"
kind = "place"
title = "Unknown birth place"

[names]
preferred = "Unknown birth place"
"#
    .to_string()
}

fn personal_knowledge_source_markdown() -> String {
    r#"+++
schema_version = 1
id = "source:personal-knowledge"
kind = "birth-record"
title = "Personal knowledge placeholder"
date_accessed = "2026-07-09"
media = []
+++

Use this placeholder only until you have a more specific source.
"#
    .to_string()
}

fn event_kinds_toml() -> String {
    r#"id = "vocab:event-kinds"
kind = "vocabulary"
title = "Event kinds"

[[terms]]
id = "birth"
label = "Birth"
class = "life"

[[terms]]
id = "death"
label = "Death"
class = "life"

[[terms]]
id = "census"
label = "Census"
class = "recorded-event"
"#
    .to_string()
}

fn participant_roles_toml() -> String {
    r#"id = "vocab:participant-roles"
kind = "vocabulary"
title = "Participant roles"

[[terms]]
id = "child"
label = "Child"

[[terms]]
id = "resident"
label = "Resident"

[[terms]]
id = "subject"
label = "Subject"
"#
    .to_string()
}

fn gedcom_imports_readme() -> String {
    r#"# GEDCOM imports

Keep raw, versioned GEDCOM exports here, for example:

- `family-neutral-example.ged`
- `family-export-neutral-example.ged`

Select the active GEDCOM in this world's `world.toml`:

```toml
[imports.gedcom.primary]
path = "imports/gedcom/family-neutral-example.ged"
strategy = "link"
```

Raw imports are world-owned source artifacts. Generated JSON/SQLite belongs under `build/`.
"#
    .to_string()
}

fn life_collection_toml(options: &LocalSkeletonOptions) -> String {
    format!(
        r#"schema_version = 1
id = "collection:{}-life"
kind = "event-collection"
title = "{} life events"
collection_kind = "sequence"
order = "manual_then_chronological"

[[members]]
event = "event:birth-{}"
label = "Birth"
role = "start"
ordinal = 10
"#,
        escape_toml_basic(&options.person_slug),
        escape_toml_basic(&options.person_name),
        escape_toml_basic(&options.person_slug),
    )
}

fn timeline_view_toml(options: &LocalSkeletonOptions) -> String {
    format!(
        r#"schema_version = 1
id = "timeline:example-life"
kind = "timeline-view"
title = "Example Life Timeline"

[subject]
entity = "{}"

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
        escape_toml_basic(&options.person_id())
    )
}

fn tree_view_toml(options: &LocalSkeletonOptions) -> String {
    format!(
        r#"schema_version = 1
id = "tree:main-family-tree"
kind = "tree-view"
title = "Main Family Tree"

[root]
entity = "{}"

[filter]
relationship_kinds = ["biological-parent-child", "adoptive-parent-child", "foster-parent-child", "step-parent-child", "guardian-child", "spouse", "partner", "former-spouse", "sibling"]
generations_up = 5
generations_down = 3

[display]
show_life_dates = true
show_places = true
show_sources = false
"#,
        escape_toml_basic(&options.person_id())
    )
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

fn split_display_name(name: &str) -> (String, String) {
    let mut parts = name.split_whitespace().collect::<Vec<_>>();
    match parts.len() {
        0 => (String::new(), String::new()),
        1 => (parts[0].to_string(), String::new()),
        _ => {
            let surname = parts.pop().unwrap_or_default().to_string();
            (parts.join(" "), surname)
        }
    }
}

fn escape_toml_basic(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::local_authoring::{compile_local_data, compile_local_tree};

    #[test]
    fn birth_event_with_time_gets_separate_time_support_assertion() {
        let temp_dir = std::env::temp_dir().join(format!(
            "kleio-birth-time-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time")
                .as_nanos()
        ));
        create_local_birth_event(
            &temp_dir,
            &LocalBirthEventOptions {
                person_slug: "alex-example".to_string(),
                person_name: "Alex Example".to_string(),
                birth_date: Some("1900-01-01 07:18".to_string()),
                force: false,
            },
        )
        .expect("birth event");

        let event_text = fs::read_to_string(
            temp_dir.join("events/births/1900-01-01 07:18-birth-alex-example.md"),
        )
        .expect("event text");
        assert!(event_text.contains("date_precision = \"minute\""));
        assert!(event_text.contains("target = \"#date\""));
        assert!(event_text.contains("target = \"#time\""));
        assert!(event_text.contains("confidence = \"high\""));
        assert!(event_text.contains("confidence = \"low\""));
        assert!(!event_text.contains("predicate ="));
        assert!(!event_text.contains("value ="));
        assert!(
            !temp_dir
                .join("assertions/birth-date-alex-example.md")
                .exists()
        );
        assert!(
            !temp_dir
                .join("assertions/birth-time-alex-example.md")
                .exists()
        );

        fs::remove_dir_all(temp_dir).expect("remove temp dir");
    }

    #[test]
    fn creates_starter_world_skeleton() {
        let temp_dir = std::env::temp_dir().join(format!(
            "kleio-local-skeleton-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time")
                .as_nanos()
        ));
        let options = LocalSkeletonOptions {
            project_id: "test-world".to_string(),
            title: "Test world".to_string(),
            person_slug: "alex-example".to_string(),
            person_name: "Alex Example".to_string(),
            birth_date: Some("1900-01-01".to_string()),
            force: false,
        };

        create_local_skeleton(&temp_dir, &options).expect("create skeleton");

        let world_root = temp_dir.join("worlds/test-world");
        assert!(temp_dir.join("kleio.toml").exists());
        assert!(world_root.join("world.toml").exists());
        assert!(world_root.join("imports/gedcom/README.md").exists());
        assert!(
            world_root
                .join("events/births/1900-01-01-birth-alex-example.md")
                .exists()
        );
        assert!(world_root.join("schemas/components/identity.toml").exists());
        assert!(world_root.join("schemas/bundles/person.toml").exists());
        let bundle = compile_local_data(&world_root).expect("world skeleton compiles");
        assert!(
            bundle
                .markdown_records
                .iter()
                .any(|record| record.id == "person:alex-example")
        );
        assert!(
            bundle
                .markdown_records
                .iter()
                .any(|record| record.id == "event:birth-alex-example")
        );

        let birth_event_text =
            fs::read_to_string(world_root.join("events/births/1900-01-01-birth-alex-example.md"))
                .expect("birth event text");
        assert!(birth_event_text.contains("target = \"#date\""));
        assert!(!birth_event_text.contains("target = \"#time\""));
        assert!(birth_event_text.contains("confidence = \"high\""));
        assert!(!birth_event_text.contains("predicate ="));
        assert!(!birth_event_text.contains("value ="));
        assert!(
            !world_root
                .join("assertions/birth-date-alex-example.md")
                .exists()
        );
        assert!(
            !world_root
                .join("assertions/birth-time-alex-example.md")
                .exists()
        );

        let tree = compile_local_tree(&world_root).expect("skeleton tree compiles");
        assert_eq!(tree.people.len(), 1);
        assert_eq!(tree.events.len(), 1);

        fs::remove_dir_all(temp_dir).expect("remove temp dir");
    }
}
