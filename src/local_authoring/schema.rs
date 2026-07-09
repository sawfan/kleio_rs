use std::fs;
use std::path::{Path, PathBuf};

use super::{LocalAuthoringError, WorldPaths};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalSchemaKind {
    Component,
    Bundle,
    Event,
    View,
    Vocab,
}

impl LocalSchemaKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Component => "ecs-component",
            Self::Bundle => "ecs-bundle",
            Self::Event => "event-schema",
            Self::View => "view-schema",
            Self::Vocab => "vocabulary",
        }
    }

    pub fn id_prefix(self) -> &'static str {
        match self {
            Self::Component => "component",
            Self::Bundle => "bundle",
            Self::Event => "event-schema",
            Self::View => "view-schema",
            Self::Vocab => "vocab",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalSchemaOptions {
    pub schema_slug: String,
    pub title: String,
    pub kind: LocalSchemaKind,
    pub force: bool,
}

impl LocalSchemaOptions {
    pub fn id(&self) -> String {
        format!("{}:{}", self.kind.id_prefix(), self.schema_slug)
    }
}

pub fn create_local_schema(
    world_root: impl AsRef<Path>,
    options: &LocalSchemaOptions,
) -> Result<PathBuf, LocalAuthoringError> {
    validate_slug(&options.schema_slug, "schema slug")?;
    let world_root = world_root.as_ref();
    let paths = WorldPaths::new(world_root);
    let dir = match options.kind {
        LocalSchemaKind::Component => paths.component_schemas_dir(),
        LocalSchemaKind::Bundle => paths.bundle_schemas_dir(),
        LocalSchemaKind::Event => paths.event_schemas_dir(),
        LocalSchemaKind::View => paths.view_schemas_dir(),
        LocalSchemaKind::Vocab => paths.vocab_schemas_dir(),
    };
    create_dir(world_root, &dir)?;
    let path = dir.join(format!("{}.toml", options.schema_slug));
    write_new_file(world_root, &path, &schema_toml(options), options.force)?;
    Ok(path)
}

fn schema_toml(options: &LocalSchemaOptions) -> String {
    match options.kind {
        LocalSchemaKind::Bundle => format!(
            r#"schema_version = 1
id = "{}"
kind = "{}"
title = "{}"

components = []
"#,
            escape_toml_basic(&options.id()),
            options.kind.as_str(),
            escape_toml_basic(&options.title)
        ),
        LocalSchemaKind::Vocab => format!(
            r#"schema_version = 1
id = "{}"
kind = "{}"
title = "{}"

terms = []
"#,
            escape_toml_basic(&options.id()),
            options.kind.as_str(),
            escape_toml_basic(&options.title)
        ),
        _ => format!(
            r#"schema_version = 1
id = "{}"
kind = "{}"
title = "{}"
description = ""
"#,
            escape_toml_basic(&options.id()),
            options.kind.as_str(),
            escape_toml_basic(&options.title)
        ),
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
    fn creates_world_schema_record() {
        let temp_dir = std::env::temp_dir().join(format!(
            "kleio-schema-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time")
                .as_nanos()
        ));
        create_workspace_skeleton(&temp_dir, &LocalSkeletonOptions::default()).expect("skeleton");
        let world_root = temp_dir.join("worlds/default");

        let path = create_local_schema(
            &world_root,
            &LocalSchemaOptions {
                schema_slug: "example-component".to_string(),
                title: "Example Component".to_string(),
                kind: LocalSchemaKind::Component,
                force: false,
            },
        )
        .expect("schema");

        assert_eq!(
            path.strip_prefix(&world_root).unwrap(),
            Path::new("schemas/components/example-component.toml")
        );

        fs::remove_dir_all(temp_dir).expect("remove temp dir");
    }
}
