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
mod event_profiles;
mod filename_hints;
mod imports;
mod kinship;
mod locations;
mod paths;
mod records;
mod refs;
mod schema;
mod skeleton;
mod sources;
mod summary;
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

pub use summary::{
    LocalMediaCheckReport, LocalMediaReferenceCheck, LocalWorldDiagnostic,
    LocalWorldDiagnosticKind, LocalWorldDiagnosticSeverity, LocalWorldDoctorReport,
    LocalWorldSummary, LocalWorldSummaryCounts, LocalWorldSummaryWarning,
    LocalWorldSummaryWarningKind, check_local_media, check_local_media_bundle,
    doctor_local_data_bundle, doctor_local_world, summarize_local_data_bundle,
    summarize_local_world,
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
use filename_hints::event_filename_hints;
use locations::event_locations;
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
    read_local_data(source_root, true)
}

pub fn read_local_data_unvalidated(
    source_root: impl AsRef<Path>,
) -> Result<LocalDataBundle, LocalAuthoringError> {
    read_local_data(source_root, false)
}

fn read_local_data(
    source_root: impl AsRef<Path>,
    validate: bool,
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

    if validate {
        validate_local_data(&markdown_records, &toml_documents)?;
    }

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
    apply_event_filename_hints(relative_path, &id, &kind, &mut table)?;
    apply_person_filename_hints(relative_path, &kind, &mut table, title.as_deref());
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

fn apply_person_filename_hints(
    relative_path: &Path,
    kind: &str,
    table: &mut toml::Table,
    title: Option<&str>,
) {
    if kind != "person" || !relative_path.starts_with("entities/people") {
        return;
    }

    let Some(legal_name) = inferred_name_from_person_filename(relative_path) else {
        return;
    };

    if !has_name_table(table, "legal") {
        insert_name_table(table, "legal", &legal_name);
    }

    if !has_name_table(table, "preferred") {
        let preferred = table
            .remove("preferred_name")
            .and_then(|value| value.as_str().map(ToOwned::to_owned))
            .or_else(|| title.map(ToOwned::to_owned));
        if let Some(preferred_name) = preferred
            && let Some(preferred) =
                preferred_name_parts(&preferred_name, legal_name.family.as_deref())
        {
            insert_name_table(table, "preferred", &preferred);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct InferredNameParts {
    full: String,
    given: Option<String>,
    middle: Option<String>,
    family: Option<String>,
}

fn inferred_name_from_person_filename(relative_path: &Path) -> Option<InferredNameParts> {
    let stem = relative_path.file_stem()?.to_str()?;
    name_parts_from_words(
        stem.split(['-', '_'])
            .filter(|part| !part.trim().is_empty())
            .map(title_case_slug_word)
            .collect::<Vec<_>>(),
    )
}

fn preferred_name_parts(value: &str, legal_family: Option<&str>) -> Option<InferredNameParts> {
    let mut words = value
        .split_whitespace()
        .filter(|part| !part.trim().is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    if words.is_empty() {
        return None;
    }

    if words.len() == 1
        && let Some(family) = legal_family
    {
        words.push(family.to_string());
    }

    name_parts_from_words(words)
}

fn name_parts_from_words(words: Vec<String>) -> Option<InferredNameParts> {
    if words.is_empty() {
        return None;
    }

    let full = words.join(" ");
    let given = words.first().cloned();
    let family = (words.len() > 1).then(|| words[words.len() - 1].clone());
    let middle = (words.len() > 2).then(|| words[1..words.len() - 1].join(" "));

    Some(InferredNameParts {
        full,
        given,
        middle,
        family,
    })
}

fn title_case_slug_word(value: &str) -> String {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    first.to_uppercase().chain(chars).collect()
}

fn has_name_table(table: &toml::Table, usage: &str) -> bool {
    table
        .get("names")
        .and_then(toml::Value::as_table)
        .and_then(|names| names.get(usage))
        .and_then(toml::Value::as_table)
        .is_some()
}

fn insert_name_table(table: &mut toml::Table, usage: &str, parts: &InferredNameParts) {
    let names = table
        .entry("names".to_string())
        .or_insert_with(|| toml::Value::Table(toml::Table::new()));
    let Some(names) = names.as_table_mut() else {
        return;
    };

    let mut name = toml::Table::new();
    name.insert("full".to_string(), toml::Value::String(parts.full.clone()));
    if let Some(given) = &parts.given {
        name.insert("given".to_string(), toml::Value::String(given.clone()));
    }
    if let Some(middle) = &parts.middle {
        name.insert("middle".to_string(), toml::Value::String(middle.clone()));
    }
    if let Some(family) = &parts.family {
        name.insert("family".to_string(), toml::Value::String(family.clone()));
    }
    names.insert(usage.to_string(), toml::Value::Table(name));
}

fn apply_event_filename_hints(
    relative_path: &Path,
    id: &str,
    kind: &str,
    table: &mut toml::Table,
) -> Result<(), LocalAuthoringError> {
    let hints = event_filename_hints(relative_path)?;

    if let Some(event_type) = hints.event_type {
        table
            .entry("type".to_string())
            .or_insert(toml::Value::String(event_type));
    }

    if let Some(time) = hints.time {
        table
            .entry("time".to_string())
            .or_insert(toml::Value::String(time));
    }

    if let Some(time_basis) = hints.time_basis {
        table
            .entry("time_basis".to_string())
            .or_insert(toml::Value::String(time_basis));
    }

    let participant = hints
        .participant
        .or_else(|| infer_birth_participant(relative_path, id, kind, table));
    if let Some(participant) = participant {
        replace_self_subject(table, &participant);
        if !table.contains_key("participants") && !table.contains_key("subject") {
            table.insert("subject".to_string(), toml::Value::String(participant));
        } else {
            replace_self_participants(table, &participant);
        }
    }

    if (hints.latitude.is_some() || hints.longitude.is_some()) && !table.contains_key("places") {
        merge_filename_location_coordinates(table, hints.latitude, hints.longitude);
    }

    Ok(())
}

fn infer_birth_participant(
    relative_path: &Path,
    id: &str,
    kind: &str,
    table: &toml::Table,
) -> Option<String> {
    if kind != "event" {
        return None;
    }
    let event_type = table.get("type").and_then(toml::Value::as_str)?;
    if event_type != "birth" || !relative_path.starts_with("events/births") {
        return None;
    }
    id.strip_prefix("event:birth-")
        .filter(|slug| !slug.trim().is_empty())
        .map(ToOwned::to_owned)
}

fn replace_self_subject(table: &mut toml::Table, participant: &str) {
    if table.get("subject").and_then(toml::Value::as_str) == Some("self") {
        table.insert(
            "subject".to_string(),
            toml::Value::String(participant.to_string()),
        );
    }
}

fn replace_self_participants(table: &mut toml::Table, participant: &str) {
    let Some(toml::Value::Array(participants)) = table.get_mut("participants") else {
        return;
    };

    for value in participants {
        match value {
            toml::Value::String(entity) if entity == "self" => {
                *entity = participant.to_string();
            }
            toml::Value::Table(participant_table) => {
                if participant_table
                    .get("entity")
                    .and_then(toml::Value::as_str)
                    == Some("self")
                {
                    participant_table.insert(
                        "entity".to_string(),
                        toml::Value::String(participant.to_string()),
                    );
                }
            }
            _ => {}
        }
    }
}

fn merge_filename_location_coordinates(
    table: &mut toml::Table,
    latitude: Option<f64>,
    longitude: Option<f64>,
) {
    let (Some(latitude), Some(longitude)) = (latitude, longitude) else {
        return;
    };

    match table.remove("location") {
        Some(toml::Value::Table(mut location)) => {
            location
                .entry("latitude".to_string())
                .or_insert(toml::Value::Float(latitude));
            location
                .entry("longitude".to_string())
                .or_insert(toml::Value::Float(longitude));
            table.insert("location".to_string(), toml::Value::Table(location));
        }
        Some(toml::Value::String(label)) => {
            let mut location = toml::Table::new();
            location.insert("label".to_string(), toml::Value::String(label));
            location.insert("latitude".to_string(), toml::Value::Float(latitude));
            location.insert("longitude".to_string(), toml::Value::Float(longitude));
            table.insert("location".to_string(), toml::Value::Table(location));
        }
        Some(value) => {
            table.insert("location".to_string(), value);
        }
        None => {
            let mut location = toml::Table::new();
            location.insert("latitude".to_string(), toml::Value::Float(latitude));
            location.insert("longitude".to_string(), toml::Value::Float(longitude));
            table.insert("location".to_string(), toml::Value::Table(location));
        }
    }
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
    let inferred_id = infer_toml_document_id(relative_path);
    let inferred_kind = infer_toml_document_kind(relative_path);
    let id = value
        .get("id")
        .and_then(toml_value_as_string)
        .or(inferred_id);
    let kind = value
        .get("kind")
        .and_then(toml_value_as_string)
        .or(inferred_kind);
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

fn infer_toml_document_id(relative_path: &Path) -> Option<String> {
    let stem = relative_path.file_stem()?.to_str()?;
    if relative_path.starts_with("relationships") {
        Some(format!("relationship:{stem}"))
    } else {
        None
    }
}

fn infer_toml_document_kind(relative_path: &Path) -> Option<String> {
    if relative_path.starts_with("relationships") {
        Some("relationship".to_string())
    } else {
        None
    }
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
