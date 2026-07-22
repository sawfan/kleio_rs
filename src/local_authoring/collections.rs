use std::fs;
use std::path::{Path, PathBuf};

use super::{LocalAuthoringError, WorldPaths};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalCollectionKind {
    Set,
    Sequence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalCollectionOrder {
    Chronological,
    Manual,
    ManualThenChronological,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalCollectionOptions {
    pub collection_slug: String,
    pub title: String,
    pub kind: LocalCollectionKind,
    pub order: LocalCollectionOrder,
    pub members: Vec<String>,
    pub force: bool,
}

impl LocalCollectionOptions {
    pub fn id(&self) -> String {
        format!("collection:{}", self.collection_slug)
    }
}

impl LocalCollectionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Set => "set",
            Self::Sequence => "sequence",
        }
    }
}

impl LocalCollectionOrder {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Chronological => "chronological",
            Self::Manual => "manual",
            Self::ManualThenChronological => "manual_then_chronological",
        }
    }
}

pub fn create_local_collection(
    world_root: impl AsRef<Path>,
    options: &LocalCollectionOptions,
) -> Result<PathBuf, LocalAuthoringError> {
    validate_slug(&options.collection_slug, "collection slug")?;
    for member in &options.members {
        validate_record_id(member, "collection member")?;
    }

    let world_root = world_root.as_ref();
    let paths = WorldPaths::new(world_root);
    create_dir(world_root, &paths.collections_dir())?;
    let path = paths
        .collections_dir()
        .join(format!("{}.toml", options.collection_slug));
    write_new_file(world_root, &path, &collection_toml(options), options.force)?;
    Ok(path)
}

fn collection_toml(options: &LocalCollectionOptions) -> String {
    let mut text = format!(
        r#"schema_version = 1
id = "{}"
kind = "event-collection"
title = "{}"
collection_kind = "{}"
"#,
        escape_toml_basic(&options.id()),
        escape_toml_basic(&options.title),
        options.kind.as_str(),
    );

    if options.kind == LocalCollectionKind::Sequence {
        text.push_str(&format!("order = \"{}\"\n", options.order.as_str()));
    }

    if options.members.is_empty() {
        text.push_str(
            r#"
# Add events in editor-friendly TOML:
# [[members]]
# event = "event:example"
# label = "Example"
# role = "reference"
"#,
        );
    } else {
        for (index, member) in options.members.iter().enumerate() {
            text.push_str(&format!(
                r#"
[[members]]
event = "{}"
ordinal = {}
"#,
                escape_toml_basic(member),
                (index as i32 + 1) * 10,
            ));
        }
    }

    text
}

fn validate_slug(value: &str, label: &str) -> Result<(), LocalAuthoringError> {
    if value.trim().is_empty() {
        return Err(LocalAuthoringError::Validation {
            message: format!("{label} cannot be empty"),
        });
    }
    if !value
        .chars()
        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
    {
        return Err(LocalAuthoringError::Validation {
            message: format!("{label} must contain only lowercase letters, digits, and '-'"),
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
    if !value.contains(':') {
        return Err(LocalAuthoringError::Validation {
            message: format!("{label} `{value}` should be a stable id such as event:example"),
        });
    }
    Ok(())
}

fn create_dir(world_root: &Path, dir: &Path) -> Result<(), LocalAuthoringError> {
    fs::create_dir_all(dir).map_err(|source| LocalAuthoringError::Io {
        path: world_root.join(dir),
        source,
    })
}

fn write_new_file(
    world_root: &Path,
    path: &Path,
    content: &str,
    force: bool,
) -> Result<(), LocalAuthoringError> {
    if path.exists() && !force {
        return Err(LocalAuthoringError::Validation {
            message: format!(
                "{} already exists; pass force/--force to overwrite",
                path.strip_prefix(world_root).unwrap_or(path).display()
            ),
        });
    }

    fs::write(path, content).map_err(|source| LocalAuthoringError::Io {
        path: path.to_path_buf(),
        source,
    })
}

fn escape_toml_basic(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_collection_toml() {
        let temp_dir = std::env::temp_dir().join(format!(
            "kleio-collection-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time")
                .as_nanos()
        ));
        let path = create_local_collection(
            &temp_dir,
            &LocalCollectionOptions {
                collection_slug: "comparison".to_string(),
                title: "Comparison".to_string(),
                kind: LocalCollectionKind::Set,
                order: LocalCollectionOrder::ManualThenChronological,
                members: vec!["event:first".to_string(), "event:second".to_string()],
                force: false,
            },
        )
        .expect("create collection");
        let text = fs::read_to_string(&path).expect("read collection");

        assert!(text.contains("id = \"collection:comparison\""));
        assert!(text.contains("event = \"event:first\""));

        fs::remove_dir_all(temp_dir).expect("remove temp dir");
    }
}
