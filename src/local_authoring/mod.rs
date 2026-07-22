//! Local private data authoring helpers.
//!
//! This module supports a deliberately small authoring format for private
//! Kleio data in a user-chosen data root. `kleio-cli` defaults to the standard
//! XDG data location (`$XDG_DATA_HOME/kleio`, usually `~/.local/share/kleio`),
//! while tests and local development can pass an explicit scratch directory:
//! - Markdown records with TOML frontmatter, for human-authored notes/narrative.
//! - Plain TOML documents, for config, vocabularies, registries, and other
//!   structured data.
//! - Deterministic generated JSON files under `build/`.
//! - Raw import artifacts, such as versioned GEDCOM files, under `imports/`.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::TreeDocument;

mod build;
mod collections;
mod config;
mod data_validation;
mod ecs_compile;
mod imports;
mod kinship;
mod paths;
mod records;
mod schema;
mod skeleton;
mod timeline_compile;
mod tree_compile;
mod validation;
mod views;

pub use build::{
    LocalWorldBuildOptions, LocalWorldBuildOutput, build_local_world,
    build_local_world_with_options,
};
pub use collections::{
    LocalCollectionKind, LocalCollectionOptions, LocalCollectionOrder, create_local_collection,
};
pub use config::{
    GedcomImportConfig, GedcomImportsConfig, WorkspaceConfig, WorkspaceInfo, WorkspaceWorldEntry,
    WorldBuildConfig, WorldBuildPaths, WorldConfig, WorldImportsConfig, read_workspace_config,
    read_world_config, resolve_workspace_world_root, resolve_world_build_paths,
    write_workspace_config,
};
pub use ecs_compile::{
    LocalEcsBundle, LocalEcsEntity, LocalEcsResources, LocalEcsViews, compile_local_ecs,
    write_local_ecs_json,
};
pub use imports::{LocalImportKind, LocalImportReportOptions, create_local_import_report};
pub use kinship::{LocalDerivedKinshipRelationship, infer_local_kinship_relationships};
pub use paths::{
    DEFAULT_WORLD_SLUG, WORKSPACE_CONFIG_FILE, WORLD_CONFIG_FILE, WorkspacePaths, WorldPaths,
};
pub use records::{
    LocalAssertionOptions, LocalEntityKind, LocalEntityOptions, LocalEventOptions,
    LocalRelationshipOptions, LocalSourceOptions, create_local_assertion, create_local_entity,
    create_local_event, create_local_relationship, create_local_source,
};
pub use schema::{LocalSchemaKind, LocalSchemaOptions, create_local_schema};

pub use skeleton::{
    LocalBirthEventOptions, LocalPersonOptions, LocalSkeletonOptions, create_local_birth_event,
    create_local_person, create_local_skeleton, create_workspace_skeleton, create_world_layout,
    create_world_skeleton,
};
pub use timeline_compile::{
    LocalTimelineCollection, LocalTimelineCollectionMember, LocalTimelineEvent,
    LocalTimelineProjection, LocalTimelineViewSummary, compile_local_timeline,
    write_local_timeline_json,
};
pub use validation::{LocalWorldValidationReport, validate_local_world};
pub use views::{
    LocalViewKind, LocalViewOptions, LocalViewSummary, create_local_view, list_local_views,
};

use data_validation::validate_local_data;
use tree_compile::{tree_from_local_data_bundle, tree_from_local_data_bundle_with_view};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LocalDataBundle {
    pub schema_version: u32,
    pub compiler: String,
    pub source_root: String,
    pub markdown_records: Vec<LocalMarkdownRecord>,
    pub toml_documents: Vec<LocalTomlDocument>,
}

impl LocalDataBundle {
    pub const SCHEMA_VERSION: u32 = 1;
    pub const COMPILER: &'static str = "kleio-local-authoring/0.1.0";
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LocalMarkdownRecord {
    pub path: String,
    pub id: String,
    pub kind: String,
    pub title: Option<String>,
    pub date: Option<String>,
    pub summary: Option<String>,
    pub tags: Vec<String>,
    pub related: Vec<String>,
    pub place: Option<String>,
    pub attributes: BTreeMap<String, serde_json::Value>,
    pub notes_markdown: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LocalTomlDocument {
    pub path: String,
    pub id: Option<String>,
    pub kind: Option<String>,
    pub title: Option<String>,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LocalTreesDocument {
    pub version: u32,
    pub main_tree_id: String,
    pub trees: Vec<TreeDocument>,
}

impl LocalTreesDocument {
    pub const VERSION: u32 = 1;

    pub fn from_tree(tree: TreeDocument) -> Self {
        Self {
            version: Self::VERSION,
            main_tree_id: tree.metadata.id.0.clone(),
            trees: vec![tree],
        }
    }
}

#[derive(Debug)]
pub enum LocalAuthoringError {
    Io {
        path: PathBuf,
        source: io::Error,
    },
    Toml {
        path: PathBuf,
        source: toml::de::Error,
    },
    TomlSerialize {
        path: PathBuf,
        source: toml::ser::Error,
    },
    Json {
        path: PathBuf,
        source: serde_json::Error,
    },
    InvalidMarkdown {
        path: PathBuf,
        message: String,
    },
    Validation {
        message: String,
    },
}

impl fmt::Display for LocalAuthoringError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => write!(f, "{}: {source}", path.display()),
            Self::Toml { path, source } => write!(f, "{}: invalid TOML: {source}", path.display()),
            Self::TomlSerialize { path, source } => {
                write!(f, "{}: TOML serialization failed: {source}", path.display())
            }
            Self::Json { path, source } => {
                write!(f, "{}: JSON serialization failed: {source}", path.display())
            }
            Self::InvalidMarkdown { path, message } => {
                write!(f, "{}: invalid Markdown record: {message}", path.display())
            }
            Self::Validation { message } => write!(f, "local data validation failed: {message}"),
        }
    }
}

impl Error for LocalAuthoringError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Toml { source, .. } => Some(source),
            Self::TomlSerialize { source, .. } => Some(source),
            Self::Json { source, .. } => Some(source),
            Self::InvalidMarkdown { .. } | Self::Validation { .. } => None,
        }
    }
}

pub fn compile_local_data(
    source_root: impl AsRef<Path>,
) -> Result<LocalDataBundle, LocalAuthoringError> {
    let source_root = source_root.as_ref();
    let mut markdown_records = Vec::new();
    let mut toml_documents = Vec::new();
    let mut files = Vec::new();

    collect_local_data_files(source_root, source_root, &mut files)?;
    files.sort();

    for path in files {
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("md") => markdown_records.push(read_markdown_record(source_root, &path)?),
            Some("toml") => toml_documents.push(read_toml_document(source_root, &path)?),
            _ => {}
        }
    }

    validate_local_data(&markdown_records, &toml_documents)?;

    Ok(LocalDataBundle {
        schema_version: LocalDataBundle::SCHEMA_VERSION,
        compiler: LocalDataBundle::COMPILER.to_string(),
        source_root: source_root.display().to_string(),
        markdown_records,
        toml_documents,
    })
}

pub fn write_local_data_json(
    source_root: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
) -> Result<LocalDataBundle, LocalAuthoringError> {
    let output_path = output_path.as_ref();
    let bundle = compile_local_data(source_root)?;
    let json =
        serde_json::to_string_pretty(&bundle).map_err(|source| LocalAuthoringError::Json {
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

    Ok(bundle)
}

pub fn compile_local_tree(
    source_root: impl AsRef<Path>,
) -> Result<TreeDocument, LocalAuthoringError> {
    tree_from_local_data_bundle(&compile_local_data(source_root)?)
}

pub fn write_local_tree_json(
    source_root: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
) -> Result<TreeDocument, LocalAuthoringError> {
    let output_path = output_path.as_ref();
    let tree = compile_local_tree(source_root)?;
    let json = serde_json::to_string_pretty(&tree).map_err(|source| LocalAuthoringError::Json {
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

    Ok(tree)
}

pub fn compile_local_tree_with_view(
    source_root: impl AsRef<Path>,
    view_slug: Option<&str>,
) -> Result<TreeDocument, LocalAuthoringError> {
    tree_from_local_data_bundle_with_view(&compile_local_data(source_root)?, view_slug)
}

pub fn write_local_tree_json_with_view(
    source_root: impl AsRef<Path>,
    view_slug: Option<&str>,
    output_path: impl AsRef<Path>,
) -> Result<TreeDocument, LocalAuthoringError> {
    let output_path = output_path.as_ref();
    let tree = compile_local_tree_with_view(source_root, view_slug)?;
    let json = serde_json::to_string_pretty(&tree).map_err(|source| LocalAuthoringError::Json {
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

    Ok(tree)
}

pub fn compile_local_trees_document(
    source_root: impl AsRef<Path>,
) -> Result<LocalTreesDocument, LocalAuthoringError> {
    compile_local_tree(source_root).map(LocalTreesDocument::from_tree)
}

pub fn write_local_trees_document_json(
    source_root: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
) -> Result<LocalTreesDocument, LocalAuthoringError> {
    let output_path = output_path.as_ref();
    let document = compile_local_trees_document(source_root)?;
    let json =
        serde_json::to_string_pretty(&document).map_err(|source| LocalAuthoringError::Json {
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

    Ok(document)
}

fn collect_local_data_files(
    source_root: &Path,
    dir: &Path,
    files: &mut Vec<PathBuf>,
) -> Result<(), LocalAuthoringError> {
    let entries = fs::read_dir(dir).map_err(|source| LocalAuthoringError::Io {
        path: dir.to_path_buf(),
        source,
    })?;

    for entry in entries {
        let entry = entry.map_err(|source| LocalAuthoringError::Io {
            path: dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();

        if matches!(file_name, "README.md") {
            continue;
        }

        let file_type = entry
            .file_type()
            .map_err(|source| LocalAuthoringError::Io {
                path: path.clone(),
                source,
            })?;

        if file_name.starts_with('.')
            || (file_type.is_dir() && matches!(file_name, "build" | "compiled"))
        {
            continue;
        }

        if file_type.is_dir() {
            collect_local_data_files(source_root, &path, files)?;
        } else if file_type.is_file() && is_local_data_file(&path) {
            files.push(
                path.strip_prefix(source_root)
                    .unwrap_or(&path)
                    .to_path_buf(),
            );
        }
    }

    Ok(())
}

fn is_local_data_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("md" | "toml")
    )
}

fn read_markdown_record(
    source_root: &Path,
    relative_path: &Path,
) -> Result<LocalMarkdownRecord, LocalAuthoringError> {
    let full_path = source_root.join(relative_path);
    let text = fs::read_to_string(&full_path).map_err(|source| LocalAuthoringError::Io {
        path: full_path.clone(),
        source,
    })?;
    let (frontmatter, notes_markdown) = split_toml_frontmatter(&full_path, &text)?;
    let mut table =
        frontmatter
            .parse::<toml::Table>()
            .map_err(|source| LocalAuthoringError::Toml {
                path: full_path.clone(),
                source,
            })?;

    let id = take_required_string(&mut table, "id", &full_path)?;
    let kind = take_required_string(&mut table, "kind", &full_path)?;
    let title = take_optional_string(&mut table, "title", &full_path)?;
    let date = take_optional_string(&mut table, "date", &full_path)?;
    let summary = take_optional_string(&mut table, "summary", &full_path)?;
    let tags = take_string_array(&mut table, "tags", &full_path)?;
    let related = take_string_array(&mut table, "related", &full_path)?;
    let place = take_optional_string(&mut table, "place", &full_path)?;
    let attributes = toml_table_to_json_map(table, &full_path)?;

    Ok(LocalMarkdownRecord {
        path: relative_path_to_string(relative_path),
        id,
        kind,
        title,
        date,
        summary,
        tags,
        related,
        place,
        attributes,
        notes_markdown: notes_markdown.trim().to_string(),
    })
}

fn read_toml_document(
    source_root: &Path,
    relative_path: &Path,
) -> Result<LocalTomlDocument, LocalAuthoringError> {
    let full_path = source_root.join(relative_path);
    let text = fs::read_to_string(&full_path).map_err(|source| LocalAuthoringError::Io {
        path: full_path.clone(),
        source,
    })?;
    let value = text
        .parse::<toml::Value>()
        .map_err(|source| LocalAuthoringError::Toml {
            path: full_path.clone(),
            source,
        })?;
    let id = value.get("id").and_then(toml_value_as_string);
    let kind = value.get("kind").and_then(toml_value_as_string);
    let title = value.get("title").and_then(toml_value_as_string);
    let data = serde_json::to_value(value).map_err(|source| LocalAuthoringError::Json {
        path: full_path,
        source,
    })?;

    Ok(LocalTomlDocument {
        path: relative_path_to_string(relative_path),
        id,
        kind,
        title,
        data,
    })
}

fn split_toml_frontmatter<'a>(
    path: &Path,
    text: &'a str,
) -> Result<(&'a str, &'a str), LocalAuthoringError> {
    let Some(rest) = text
        .strip_prefix("+++\n")
        .or_else(|| text.strip_prefix("+++\r\n"))
    else {
        return Err(LocalAuthoringError::InvalidMarkdown {
            path: path.to_path_buf(),
            message: "expected TOML frontmatter delimited by +++".to_string(),
        });
    };

    if let Some(index) = rest.find("\n+++\n") {
        let frontmatter = &rest[..index];
        let body = &rest[index + "\n+++\n".len()..];
        return Ok((frontmatter, body));
    }

    if let Some(index) = rest.find("\n+++\r\n") {
        let frontmatter = &rest[..index];
        let body = &rest[index + "\n+++\r\n".len()..];
        return Ok((frontmatter, body));
    }

    Err(LocalAuthoringError::InvalidMarkdown {
        path: path.to_path_buf(),
        message: "missing closing +++ frontmatter delimiter".to_string(),
    })
}

fn take_required_string(
    table: &mut toml::Table,
    key: &str,
    path: &Path,
) -> Result<String, LocalAuthoringError> {
    take_optional_string(table, key, path)?.ok_or_else(|| LocalAuthoringError::InvalidMarkdown {
        path: path.to_path_buf(),
        message: format!("missing required `{key}` frontmatter field"),
    })
}

fn take_optional_string(
    table: &mut toml::Table,
    key: &str,
    path: &Path,
) -> Result<Option<String>, LocalAuthoringError> {
    table
        .remove(key)
        .map(|value| {
            toml_value_as_string(&value).ok_or_else(|| LocalAuthoringError::InvalidMarkdown {
                path: path.to_path_buf(),
                message: format!("`{key}` must be a string or TOML date/time value"),
            })
        })
        .transpose()
}

fn take_string_array(
    table: &mut toml::Table,
    key: &str,
    path: &Path,
) -> Result<Vec<String>, LocalAuthoringError> {
    let Some(value) = table.remove(key) else {
        return Ok(Vec::new());
    };

    let Some(values) = value.as_array() else {
        return Err(LocalAuthoringError::InvalidMarkdown {
            path: path.to_path_buf(),
            message: format!("`{key}` must be an array of strings"),
        });
    };

    values
        .iter()
        .map(|value| {
            value.as_str().map(ToOwned::to_owned).ok_or_else(|| {
                LocalAuthoringError::InvalidMarkdown {
                    path: path.to_path_buf(),
                    message: format!("`{key}` must contain only strings"),
                }
            })
        })
        .collect()
}

fn toml_value_as_string(value: &toml::Value) -> Option<String> {
    value
        .as_str()
        .map(ToOwned::to_owned)
        .or_else(|| value.as_datetime().map(ToString::to_string))
}

fn toml_table_to_json_map(
    table: toml::Table,
    path: &Path,
) -> Result<BTreeMap<String, serde_json::Value>, LocalAuthoringError> {
    table
        .into_iter()
        .map(|(key, value)| {
            if let Some(value) = toml_value_as_string(&value) {
                return Ok((key, serde_json::Value::String(value)));
            }

            serde_json::to_value(value)
                .map(|value| (key, value))
                .map_err(|source| LocalAuthoringError::Json {
                    path: path.to_path_buf(),
                    source,
                })
        })
        .collect()
}

fn relative_path_to_string(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests;
