use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use super::LocalAuthoringError;

pub const DEFAULT_PROJECT_ID: &str = "private-timeline";
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
            title: "Private timeline".to_string(),
            person_slug: DEFAULT_PERSON_SLUG.to_string(),
            person_name: "Example Person".to_string(),
            birth_date: None,
            force: false,
        }
    }
}

impl LocalSkeletonOptions {
    pub fn person_id(&self) -> String {
        format!("person:{}", self.person_slug)
    }

    pub fn birth_event_id(&self) -> String {
        format!("event:birth-{}", self.person_slug)
    }
}

pub fn create_local_person(
    root: impl AsRef<Path>,
    options: &LocalPersonOptions,
) -> Result<(), LocalAuthoringError> {
    validate_slug(&options.person_slug, "person slug")?;
    let root = root.as_ref();
    create_dir(root, root)?;
    create_dir(root, &root.join("people"))?;
    write_new_file(
        root,
        &root
            .join("people")
            .join(format!("{}.md", options.person_slug)),
        &person_markdown(&PersonTemplate {
            person_id: options.person_id(),
            person_name: options.person_name.clone(),
        }),
        options.force,
    )?;

    if options.create_birth_event {
        create_local_birth_event(
            root,
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
    root: impl AsRef<Path>,
    options: &LocalBirthEventOptions,
) -> Result<(), LocalAuthoringError> {
    validate_slug(&options.person_slug, "person slug")?;
    let root = root.as_ref();
    create_dir(root, root)?;
    create_dir(root, &root.join("events"))?;
    create_dir(root, &root.join("places"))?;
    create_dir(root, &root.join("sources"))?;

    write_new_file(
        root,
        &root.join("places/unknown-birth-place.toml"),
        &unknown_place_toml(),
        false,
    )?;
    write_new_file(
        root,
        &root.join("sources/personal-knowledge.md"),
        &personal_knowledge_source_markdown(),
        false,
    )?;
    write_new_file(
        root,
        &root.join("events").join(format!(
            "{}-birth-{}.md",
            options.birth_date.as_deref().unwrap_or("unknown-date"),
            options.person_slug
        )),
        &birth_event_markdown(&BirthEventTemplate {
            event_id: options.birth_event_id(),
            person_id: options.person_id(),
            person_name: options.person_name.clone(),
            birth_date: options.birth_date.clone(),
        }),
        options.force,
    )?;

    Ok(())
}

pub fn create_local_skeleton(
    root: impl AsRef<Path>,
    options: &LocalSkeletonOptions,
) -> Result<(), LocalAuthoringError> {
    validate_slug(&options.person_slug, "person slug")?;
    validate_slug(&options.project_id, "project id")?;
    let root = root.as_ref();
    create_dir(root, root)?;

    for dir in [
        "people",
        "events",
        "places",
        "sources",
        "media/people",
        "media/places",
        "imports/gedcom",
        "vocab",
        "build",
    ] {
        create_dir(root, &root.join(dir))?;
    }

    write_new_file(
        root,
        &root.join("kleio.toml"),
        &project_toml(options),
        options.force,
    )?;
    write_new_file(
        root,
        &root
            .join("people")
            .join(format!("{}.md", options.person_slug)),
        &person_markdown(&PersonTemplate {
            person_id: options.person_id(),
            person_name: options.person_name.clone(),
        }),
        options.force,
    )?;
    write_new_file(
        root,
        &root.join("events").join(format!(
            "{}-birth-{}.md",
            options.birth_date.as_deref().unwrap_or("unknown-date"),
            options.person_slug
        )),
        &birth_event_markdown(&BirthEventTemplate {
            event_id: options.birth_event_id(),
            person_id: options.person_id(),
            person_name: options.person_name.clone(),
            birth_date: options.birth_date.clone(),
        }),
        options.force,
    )?;
    write_new_file(
        root,
        &root.join("places/unknown-birth-place.toml"),
        &unknown_place_toml(),
        options.force,
    )?;
    write_new_file(
        root,
        &root.join("sources/personal-knowledge.md"),
        &personal_knowledge_source_markdown(),
        options.force,
    )?;
    write_new_file(
        root,
        &root.join("vocab/event-kinds.toml"),
        &event_kinds_toml(),
        options.force,
    )?;
    write_new_file(
        root,
        &root.join("vocab/participant-roles.toml"),
        &participant_roles_toml(),
        options.force,
    )?;
    write_new_file(
        root,
        &root.join("imports/gedcom/README.md"),
        &gedcom_imports_readme(),
        options.force,
    )?;

    Ok(())
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

fn project_toml(options: &LocalSkeletonOptions) -> String {
    format!(
        r#"schema_version = 1
id = "{}"
kind = "registry"
title = "{}"

[tree]
id = "{}"
title = "{}"
main_person = "{}"
description = "Private timeline source files. Plain files are canonical; build outputs are generated."

[build]
compiled_json = "build/kleio.compiled.json"
sqlite = "build/kleio.sqlite"

[imports.gedcom.primary]
# Put versioned GEDCOM files under imports/gedcom/ and point this at the active one.
# path = "imports/gedcom/family-2026-07-08.ged"
strategy = "link"
"#,
        escape_toml_basic(&options.project_id),
        escape_toml_basic(&options.title),
        escape_toml_basic(&options.project_id),
        escape_toml_basic(&options.title),
        escape_toml_basic(&options.person_id())
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
    let (given, surname) = split_display_name(&options.person_name);
    format!(
        r#"+++
id = "{}"
kind = "person"
title = "{}"
given = "{}"
surname = "{}"
sex = "unknown"
tags = ["starter"]
related = []
+++

# {}

Add biographical notes here. Keep concrete life facts in `events/` so the timeline can include sources, places, and multiple participants.
"#,
        escape_toml_basic(&options.person_id),
        escape_toml_basic(&options.person_name),
        escape_toml_basic(&given),
        escape_toml_basic(&surname),
        options.person_name
    )
}

fn birth_event_markdown(options: &BirthEventTemplate) -> String {
    let mut fields = BTreeMap::new();
    fields.insert("id", escape_toml_basic(&options.event_id));
    fields.insert(
        "title",
        escape_toml_basic(&format!("Birth of {}", options.person_name)),
    );

    let date_line = options
        .birth_date
        .as_ref()
        .map(|date| {
            format!(
                "date = \"{}\"\ndate_precision = \"day\"\n",
                escape_toml_basic(date)
            )
        })
        .unwrap_or_else(|| "date_precision = \"unknown\"\n".to_string());

    format!(
        r#"+++
id = "{}"
kind = "birth"
class = "life"
title = "{}"
{}place = "{}"

participants = [
  {{ entity = "{}", role = "child" }},
]

sources = [
  "{}",
]
+++

# Birth of {}

Replace the placeholder place/source with the best available evidence when you have it.
"#,
        fields["id"],
        fields["title"],
        date_line,
        DEFAULT_PLACE_ID,
        escape_toml_basic(&options.person_id),
        DEFAULT_SOURCE_ID,
        options.person_name
    )
}

fn unknown_place_toml() -> String {
    r#"id = "place:unknown-birth-place"
kind = "place"
title = "Unknown birth place"

[names]
preferred = "Unknown birth place"
"#
    .to_string()
}

fn personal_knowledge_source_markdown() -> String {
    r#"+++
id = "source:personal-knowledge"
kind = "source"
title = "Personal knowledge"
source_type = "note"
tags = ["starter"]
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

- `family-2026-07-08.ged`
- `family-ancestry-export-2026-07-08.ged`

Select the active GEDCOM in `../../kleio.toml`:

```toml
[imports.gedcom.primary]
path = "imports/gedcom/family-2026-07-08.ged"
strategy = "link"
```

Raw imports should be treated as source artifacts. Generated JSON/SQLite belongs under `build/`.
"#
    .to_string()
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
    fn creates_starter_timeline_skeleton() {
        let temp_dir = std::env::temp_dir().join(format!(
            "kleio-local-skeleton-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time")
                .as_nanos()
        ));
        let options = LocalSkeletonOptions {
            project_id: "test-timeline".to_string(),
            title: "Test timeline".to_string(),
            person_slug: "alex-example".to_string(),
            person_name: "Alex Example".to_string(),
            birth_date: Some("1900-01-01".to_string()),
            force: false,
        };

        create_local_skeleton(&temp_dir, &options).expect("create skeleton");

        assert!(temp_dir.join("imports/gedcom/README.md").exists());
        assert!(
            temp_dir
                .join("events/1900-01-01-birth-alex-example.md")
                .exists()
        );
        let bundle = compile_local_data(&temp_dir).expect("skeleton compiles");
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

        let tree = compile_local_tree(&temp_dir).expect("skeleton tree compiles");
        assert_eq!(tree.people.len(), 1);
        assert_eq!(tree.events.len(), 1);

        fs::remove_dir_all(temp_dir).expect("remove temp dir");
    }
}
