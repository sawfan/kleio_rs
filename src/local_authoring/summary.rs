use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::{
    LocalAuthoringError, LocalDataBundle, LocalMarkdownRecord, LocalTomlDocument,
    compile_local_data,
    refs::{normalize_person_id, normalize_source_id},
};

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LocalWorldSummary {
    pub world_id: Option<String>,
    pub world_title: Option<String>,
    pub counts: LocalWorldSummaryCounts,
    pub warnings: Vec<LocalWorldSummaryWarning>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LocalMediaCheckReport {
    pub references: Vec<LocalMediaReferenceCheck>,
}

impl LocalMediaCheckReport {
    pub fn referenced_files(&self) -> usize {
        self.references.len()
    }

    pub fn present_files(&self) -> usize {
        self.references
            .iter()
            .filter(|reference| reference.exists)
            .count()
    }

    pub fn missing_files(&self) -> usize {
        self.references
            .iter()
            .filter(|reference| !reference.exists)
            .count()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LocalMediaReferenceCheck {
    pub path: String,
    pub exists: bool,
    pub referenced_by: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LocalWorldDoctorReport {
    pub summary: LocalWorldSummary,
    pub diagnostics: Vec<LocalWorldDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LocalWorldDiagnostic {
    pub severity: LocalWorldDiagnosticSeverity,
    pub kind: LocalWorldDiagnosticKind,
    pub record_id: String,
    pub path: String,
    pub message: String,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "kebab-case")]
pub enum LocalWorldDiagnosticSeverity {
    Warning,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "kebab-case")]
pub enum LocalWorldDiagnosticKind {
    PersonMissingName,
    PersonMissingBirthEvent,
    EventMissingParticipant,
    EventMissingTime,
    EventMissingSource,
    RelationshipMissingSource,
    ReferencedFileMissing,
    RecordUnexpectedPath,
    PossibleDuplicatePerson,
    SuspiciousParentChildDirection,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LocalWorldSummaryCounts {
    pub people: usize,
    pub places: usize,
    pub organizations: usize,
    pub objects: usize,
    pub concepts: usize,
    pub events: usize,
    pub events_by_type: BTreeMap<String, usize>,
    pub sources: usize,
    pub assertions: usize,
    pub relationships: usize,
    pub collections: usize,
    pub timeline_views: usize,
    pub tree_views: usize,
    pub map_views: usize,
    pub calendar_views: usize,
    pub visualization_views: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LocalWorldSummaryWarning {
    pub kind: LocalWorldSummaryWarningKind,
    pub record_id: String,
    pub path: String,
    pub message: String,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "kebab-case")]
pub enum LocalWorldSummaryWarningKind {
    PersonMissingBirthEvent,
    EventMissingTime,
    EventMissingSource,
    RelationshipMissingSource,
    ReferencedFileMissing,
    RecordUnexpectedPath,
    PossibleDuplicatePerson,
    SuspiciousParentChildDirection,
}

impl From<LocalWorldDiagnosticKind> for LocalWorldSummaryWarningKind {
    fn from(value: LocalWorldDiagnosticKind) -> Self {
        match value {
            LocalWorldDiagnosticKind::PersonMissingName => Self::PersonMissingBirthEvent,
            LocalWorldDiagnosticKind::PersonMissingBirthEvent => Self::PersonMissingBirthEvent,
            LocalWorldDiagnosticKind::EventMissingParticipant => Self::EventMissingTime,
            LocalWorldDiagnosticKind::EventMissingTime => Self::EventMissingTime,
            LocalWorldDiagnosticKind::EventMissingSource => Self::EventMissingSource,
            LocalWorldDiagnosticKind::RelationshipMissingSource => Self::RelationshipMissingSource,
            LocalWorldDiagnosticKind::ReferencedFileMissing => Self::ReferencedFileMissing,
            LocalWorldDiagnosticKind::RecordUnexpectedPath => Self::RecordUnexpectedPath,
            LocalWorldDiagnosticKind::PossibleDuplicatePerson => Self::PossibleDuplicatePerson,
            LocalWorldDiagnosticKind::SuspiciousParentChildDirection => {
                Self::SuspiciousParentChildDirection
            }
        }
    }
}

pub fn check_local_media(
    world_root: impl AsRef<Path>,
) -> Result<LocalMediaCheckReport, LocalAuthoringError> {
    let world_root = world_root.as_ref();
    let bundle = compile_local_data(world_root)?;
    Ok(check_local_media_bundle(world_root, &bundle))
}

pub fn check_local_media_bundle(
    world_root: &Path,
    bundle: &LocalDataBundle,
) -> LocalMediaCheckReport {
    LocalMediaCheckReport {
        references: collect_file_reference_checks(world_root, bundle),
    }
}

pub fn summarize_local_world(
    world_root: impl AsRef<Path>,
) -> Result<LocalWorldSummary, LocalAuthoringError> {
    let report = doctor_local_world(world_root)?;
    Ok(report.summary)
}

pub fn doctor_local_world(
    world_root: impl AsRef<Path>,
) -> Result<LocalWorldDoctorReport, LocalAuthoringError> {
    let world_root = world_root.as_ref();
    let bundle = compile_local_data(world_root)?;
    Ok(doctor_local_data_bundle(world_root, &bundle))
}

pub fn doctor_local_data_bundle(
    world_root: &Path,
    bundle: &LocalDataBundle,
) -> LocalWorldDoctorReport {
    let diagnostics = local_world_diagnostics(world_root, bundle);
    LocalWorldDoctorReport {
        summary: summarize_local_data_bundle_with_diagnostics(bundle, &diagnostics),
        diagnostics,
    }
}

pub fn summarize_local_data_bundle(
    world_root: &Path,
    bundle: &LocalDataBundle,
) -> LocalWorldSummary {
    let diagnostics = local_world_diagnostics(world_root, bundle);
    summarize_local_data_bundle_with_diagnostics(bundle, &diagnostics)
}

fn summarize_local_data_bundle_with_diagnostics(
    bundle: &LocalDataBundle,
    diagnostics: &[LocalWorldDiagnostic],
) -> LocalWorldSummary {
    LocalWorldSummary {
        world_id: world_document(bundle).and_then(|document| document.id.clone()),
        world_title: world_document(bundle).and_then(|document| document.title.clone()),
        counts: summary_counts(bundle),
        warnings: diagnostics
            .iter()
            .map(summary_warning_from_diagnostic)
            .collect(),
    }
}

fn summary_warning_from_diagnostic(diagnostic: &LocalWorldDiagnostic) -> LocalWorldSummaryWarning {
    LocalWorldSummaryWarning {
        kind: diagnostic.kind.into(),
        record_id: diagnostic.record_id.clone(),
        path: diagnostic.path.clone(),
        message: diagnostic.message.clone(),
    }
}

fn summary_counts(bundle: &LocalDataBundle) -> LocalWorldSummaryCounts {
    let mut counts = LocalWorldSummaryCounts::default();

    for record in &bundle.markdown_records {
        match record.kind.as_str() {
            "person" => counts.people += 1,
            "place" => counts.places += 1,
            "organization" => counts.organizations += 1,
            "object" => counts.objects += 1,
            "concept" => counts.concepts += 1,
            "event" => {
                counts.events += 1;
                let event_type = event_type(record).unwrap_or("unknown");
                *counts
                    .events_by_type
                    .entry(event_type.to_string())
                    .or_default() += 1;
            }
            "note" => counts.sources += record.path.starts_with("sources/") as usize,
            _ => {}
        }

        if record.path.starts_with("sources/") {
            counts.sources += usize::from(record.kind != "note");
        } else if record.path.starts_with("assertions/") {
            counts.assertions += 1;
        }
    }

    for document in &bundle.toml_documents {
        match document.kind.as_deref() {
            Some("relationship") => counts.relationships += 1,
            Some("event-collection") => counts.collections += 1,
            Some("timeline-view") => counts.timeline_views += 1,
            Some("tree-view") => counts.tree_views += 1,
            Some("map-view") => counts.map_views += 1,
            Some("calendar-view") => counts.calendar_views += 1,
            Some("visualization-view") => counts.visualization_views += 1,
            Some("source") => counts.sources += 1,
            Some("assertion") => counts.assertions += 1,
            Some("place") => counts.places += 1,
            _ => {}
        }
    }

    counts
}

fn local_world_diagnostics(
    world_root: &Path,
    bundle: &LocalDataBundle,
) -> Vec<LocalWorldDiagnostic> {
    let mut diagnostics = Vec::new();
    add_unexpected_path_diagnostics(bundle, &mut diagnostics);
    add_duplicate_person_diagnostics(bundle, &mut diagnostics);
    add_parent_child_direction_diagnostics(bundle, &mut diagnostics);
    let birth_participants = birth_participants(bundle);

    for person in bundle
        .markdown_records
        .iter()
        .filter(|record| record.kind == "person")
    {
        if person_name(person).is_none() {
            diagnostics.push(LocalWorldDiagnostic {
                severity: LocalWorldDiagnosticSeverity::Warning,
                kind: LocalWorldDiagnosticKind::PersonMissingName,
                record_id: person.id.clone(),
                path: person.path.clone(),
                message: format!("{} does not have a primary name or title", person.id),
            });
        }

        if !birth_participants.contains(&person.id) {
            diagnostics.push(LocalWorldDiagnostic {
                severity: LocalWorldDiagnosticSeverity::Warning,
                kind: LocalWorldDiagnosticKind::PersonMissingBirthEvent,
                record_id: person.id.clone(),
                path: person.path.clone(),
                message: format!(
                    "{} does not have a birth event",
                    display_record_name(person)
                ),
            });
        }
    }

    for event in bundle
        .markdown_records
        .iter()
        .filter(|record| record.kind == "event")
    {
        if event_participants(event).is_empty() {
            diagnostics.push(LocalWorldDiagnostic {
                severity: LocalWorldDiagnosticSeverity::Warning,
                kind: LocalWorldDiagnosticKind::EventMissingParticipant,
                record_id: event.id.clone(),
                path: event.path.clone(),
                message: format!("{} has no participants", display_record_name(event)),
            });
        }

        if event_time(event).is_none() {
            diagnostics.push(LocalWorldDiagnostic {
                severity: LocalWorldDiagnosticSeverity::Warning,
                kind: LocalWorldDiagnosticKind::EventMissingTime,
                record_id: event.id.clone(),
                path: event.path.clone(),
                message: format!("{} has no time/date", display_record_name(event)),
            });
        }

        if event_source_items(event).is_empty() {
            diagnostics.push(LocalWorldDiagnostic {
                severity: LocalWorldDiagnosticSeverity::Warning,
                kind: LocalWorldDiagnosticKind::EventMissingSource,
                record_id: event.id.clone(),
                path: event.path.clone(),
                message: format!("{} does not reference a source", display_record_name(event)),
            });
        }
    }

    for relationship in bundle
        .toml_documents
        .iter()
        .filter(|document| document.kind.as_deref() == Some("relationship"))
    {
        if source_strings(relationship.data.get("sources")).is_empty() {
            diagnostics.push(LocalWorldDiagnostic {
                severity: LocalWorldDiagnosticSeverity::Warning,
                kind: LocalWorldDiagnosticKind::RelationshipMissingSource,
                record_id: relationship
                    .id
                    .clone()
                    .unwrap_or_else(|| relationship.path.clone()),
                path: relationship.path.clone(),
                message: format!(
                    "{} does not reference a source",
                    relationship.title.as_deref().unwrap_or_else(|| relationship
                        .id
                        .as_deref()
                        .unwrap_or(&relationship.path))
                ),
            });
        }
    }

    let mut missing_files = BTreeSet::new();
    for reference in collect_file_reference_checks(world_root, bundle) {
        if !reference.exists {
            missing_files.insert(reference.path);
        }
    }
    for path in missing_files {
        diagnostics.push(LocalWorldDiagnostic {
            severity: LocalWorldDiagnosticSeverity::Warning,
            kind: LocalWorldDiagnosticKind::ReferencedFileMissing,
            record_id: path.clone(),
            path: path.clone(),
            message: format!("referenced file `{path}` does not exist"),
        });
    }

    diagnostics
}

const MIN_PARENT_CHILD_AGE_GAP_YEARS: i32 = 12;

fn add_parent_child_direction_diagnostics(
    bundle: &LocalDataBundle,
    diagnostics: &mut Vec<LocalWorldDiagnostic>,
) {
    let birth_years = person_birth_years(bundle);
    for relationship in bundle
        .toml_documents
        .iter()
        .filter(|document| document.kind.as_deref() == Some("relationship"))
    {
        let Some(kind) = relationship_kind(relationship) else {
            continue;
        };
        if !is_parent_child_relationship_kind(kind) {
            continue;
        }
        let Some(source) = relationship
            .data
            .get("source")
            .and_then(serde_json::Value::as_str)
            .map(normalize_person_id)
        else {
            continue;
        };
        let Some(target) = relationship
            .data
            .get("target")
            .and_then(serde_json::Value::as_str)
            .map(normalize_person_id)
        else {
            continue;
        };
        let (Some(parent_year), Some(child_year)) =
            (birth_years.get(&source), birth_years.get(&target))
        else {
            continue;
        };
        let age_gap = child_year - parent_year;
        if age_gap < MIN_PARENT_CHILD_AGE_GAP_YEARS {
            diagnostics.push(LocalWorldDiagnostic {
                severity: LocalWorldDiagnosticSeverity::Warning,
                kind: LocalWorldDiagnosticKind::SuspiciousParentChildDirection,
                record_id: relationship
                    .id
                    .clone()
                    .unwrap_or_else(|| relationship.path.clone()),
                path: relationship.path.clone(),
                message: format!(
                    "{} has parent-child timing that looks suspicious: source `{source}` was born in {parent_year}, target `{target}` was born in {child_year}",
                    relationship.id.as_deref().unwrap_or(&relationship.path)
                ),
            });
        }
    }
}

fn person_birth_years(bundle: &LocalDataBundle) -> BTreeMap<String, i32> {
    let mut birth_years = BTreeMap::new();
    for record in bundle
        .markdown_records
        .iter()
        .filter(|record| record.kind == "person")
    {
        if let Some(year) = record
            .attributes
            .get("birth_date")
            .and_then(serde_json::Value::as_str)
            .and_then(extract_leading_year)
            .or_else(|| {
                record
                    .attributes
                    .get("birth_year")
                    .and_then(serde_json::Value::as_i64)
                    .and_then(|year| i32::try_from(year).ok())
            })
        {
            birth_years.insert(record.id.clone(), year);
        }
    }

    for event in bundle
        .markdown_records
        .iter()
        .filter(|record| record.kind == "event" && event_type(record) == Some("birth"))
    {
        let Some(year) = event_time(event).and_then(extract_leading_year) else {
            continue;
        };
        for participant in event_participants(event) {
            birth_years.entry(participant).or_insert(year);
        }
    }

    birth_years
}

fn relationship_kind(document: &LocalTomlDocument) -> Option<&str> {
    document
        .data
        .get("relationship")
        .and_then(serde_json::Value::as_str)
        .or_else(|| {
            document
                .data
                .get("relationship_kind")
                .and_then(serde_json::Value::as_str)
        })
        .or_else(|| {
            document
                .data
                .get("relation")
                .and_then(serde_json::Value::as_str)
        })
}

fn is_parent_child_relationship_kind(kind: &str) -> bool {
    matches!(
        kind,
        "biological-parent-child"
            | "adoptive-parent-child"
            | "foster-parent-child"
            | "step-parent-child"
            | "guardian-child"
    )
}

fn extract_leading_year(value: &str) -> Option<i32> {
    let value = value.trim();
    let end = value
        .char_indices()
        .take_while(|(_, ch)| ch.is_ascii_digit() || *ch == '-')
        .map(|(index, ch)| index + ch.len_utf8())
        .last()?;
    value[..end]
        .split('-')
        .next()
        .and_then(|year| year.parse().ok())
}

fn add_duplicate_person_diagnostics(
    bundle: &LocalDataBundle,
    diagnostics: &mut Vec<LocalWorldDiagnostic>,
) {
    let mut people_by_name = BTreeMap::<String, Vec<&LocalMarkdownRecord>>::new();
    for person in bundle
        .markdown_records
        .iter()
        .filter(|record| record.kind == "person")
    {
        let Some(name) = person_name(person) else {
            continue;
        };
        let normalized = normalize_person_name(&name);
        if !normalized.is_empty() {
            people_by_name.entry(normalized).or_default().push(person);
        }
    }

    for people in people_by_name.values().filter(|people| people.len() > 1) {
        let duplicate_ids = people
            .iter()
            .map(|person| person.id.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        for person in people {
            diagnostics.push(LocalWorldDiagnostic {
                severity: LocalWorldDiagnosticSeverity::Warning,
                kind: LocalWorldDiagnosticKind::PossibleDuplicatePerson,
                record_id: person.id.clone(),
                path: person.path.clone(),
                message: format!(
                    "{} may duplicate another person with the same name: {duplicate_ids}",
                    person.id
                ),
            });
        }
    }
}

fn normalize_person_name(name: &str) -> String {
    name.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn add_unexpected_path_diagnostics(
    bundle: &LocalDataBundle,
    diagnostics: &mut Vec<LocalWorldDiagnostic>,
) {
    for record in &bundle.markdown_records {
        let expected_prefix = match record.kind.as_str() {
            "person" => Some("entities/people/"),
            "place" => Some("entities/places/"),
            "organization" => Some("entities/organizations/"),
            "object" => Some("entities/objects/"),
            "concept" => Some("entities/concepts/"),
            "event" => Some("events/"),
            _ if record.path.starts_with("sources/") => Some("sources/"),
            _ if record.path.starts_with("assertions/") => Some("assertions/"),
            _ => None,
        };
        if let Some(expected_prefix) = expected_prefix
            && !record.path.starts_with(expected_prefix)
        {
            diagnostics.push(unexpected_path_diagnostic(
                &record.id,
                &record.path,
                record.kind.as_str(),
                expected_prefix,
            ));
        }
    }

    for document in &bundle.toml_documents {
        let Some(kind) = document.kind.as_deref() else {
            continue;
        };
        let expected_prefix = match kind {
            "relationship" => Some("relationships/"),
            "event-collection" => Some("collections/"),
            "timeline-view" => Some("views/timelines/"),
            "tree-view" => Some("views/trees/"),
            "map-view" => Some("views/maps/"),
            "calendar-view" => Some("views/calendars/"),
            "visualization-view" => Some("views/visualizations/"),
            "source" => Some("sources/"),
            "assertion" => Some("assertions/"),
            "place" => Some("entities/places/"),
            _ => None,
        };
        if let Some(expected_prefix) = expected_prefix
            && !document.path.starts_with(expected_prefix)
        {
            diagnostics.push(unexpected_path_diagnostic(
                document.id.as_deref().unwrap_or(&document.path),
                &document.path,
                kind,
                expected_prefix,
            ));
        }
    }
}

fn unexpected_path_diagnostic(
    record_id: &str,
    path: &str,
    kind: &str,
    expected_prefix: &str,
) -> LocalWorldDiagnostic {
    LocalWorldDiagnostic {
        severity: LocalWorldDiagnosticSeverity::Warning,
        kind: LocalWorldDiagnosticKind::RecordUnexpectedPath,
        record_id: record_id.to_string(),
        path: path.to_string(),
        message: format!("{record_id} has kind `{kind}` but is outside `{expected_prefix}`"),
    }
}

fn world_document(bundle: &LocalDataBundle) -> Option<&LocalTomlDocument> {
    bundle
        .toml_documents
        .iter()
        .find(|document| document.path == "world.toml")
}

fn birth_participants(bundle: &LocalDataBundle) -> BTreeSet<String> {
    bundle
        .markdown_records
        .iter()
        .filter(|record| record.kind == "event" && event_type(record) == Some("birth"))
        .flat_map(event_participants)
        .collect()
}

fn event_participants(record: &LocalMarkdownRecord) -> Vec<String> {
    if let Some(subject) = record
        .attributes
        .get("subject")
        .and_then(serde_json::Value::as_str)
        .filter(|subject| !subject.trim().is_empty())
    {
        return vec![normalize_person_id(subject)];
    }

    let Some(participants) = record
        .attributes
        .get("participants")
        .and_then(serde_json::Value::as_array)
    else {
        return Vec::new();
    };

    participants
        .iter()
        .filter_map(|participant| match participant {
            serde_json::Value::String(id) => Some(normalize_person_id(id)),
            serde_json::Value::Object(value) => value
                .get("entity")
                .and_then(serde_json::Value::as_str)
                .map(normalize_person_id),
            _ => None,
        })
        .collect()
}

fn event_type(record: &LocalMarkdownRecord) -> Option<&str> {
    record
        .attributes
        .get("type")
        .and_then(serde_json::Value::as_str)
}

fn event_time(record: &LocalMarkdownRecord) -> Option<&str> {
    record.date.as_deref().or_else(|| {
        record
            .attributes
            .get("time")
            .and_then(serde_json::Value::as_str)
    })
}

fn display_record_name(record: &LocalMarkdownRecord) -> String {
    person_name(record).unwrap_or_else(|| record.id.clone())
}

fn person_name(record: &LocalMarkdownRecord) -> Option<String> {
    record
        .title
        .clone()
        .or_else(|| record_name_table(record, "preferred"))
        .or_else(|| record_name_table(record, "legal"))
}

fn record_name_table(record: &LocalMarkdownRecord, usage: &str) -> Option<String> {
    let table = record.attributes.get("names")?.get(usage)?;
    table
        .get("display")
        .and_then(serde_json::Value::as_str)
        .or_else(|| table.get("full").and_then(serde_json::Value::as_str))
        .map(ToOwned::to_owned)
        .or_else(|| {
            let given = table.get("given").and_then(serde_json::Value::as_str)?;
            let family = table.get("family").and_then(serde_json::Value::as_str)?;
            Some(format!("{given} {family}"))
        })
}

fn event_source_items(record: &LocalMarkdownRecord) -> Vec<String> {
    let mut sources = source_items(record.attributes.get("sources"));
    if let Some(assertions) = record
        .attributes
        .get("assertions")
        .and_then(serde_json::Value::as_array)
    {
        for assertion in assertions {
            sources.extend(source_items(assertion.get("sources")));
        }
    }
    sources
}

fn source_items(value: Option<&serde_json::Value>) -> Vec<String> {
    match value {
        Some(serde_json::Value::Array(values)) => values
            .iter()
            .filter_map(|value| match value {
                serde_json::Value::String(id) if !id.trim().is_empty() => {
                    Some(normalize_source_id(id))
                }
                serde_json::Value::Object(source) if inline_source_has_identity(source) => {
                    Some("inline-source".to_string())
                }
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn source_strings(value: Option<&serde_json::Value>) -> Vec<String> {
    match value {
        Some(serde_json::Value::Array(values)) => values
            .iter()
            .filter_map(serde_json::Value::as_str)
            .map(normalize_source_id)
            .collect(),
        _ => Vec::new(),
    }
}

fn inline_source_has_identity(source: &serde_json::Map<String, serde_json::Value>) -> bool {
    [
        "label", "title", "file", "path", "uri", "url", "hash", "sha256",
    ]
    .into_iter()
    .any(|key| {
        source
            .get(key)
            .and_then(serde_json::Value::as_str)
            .is_some_and(|value| !value.trim().is_empty())
    })
}

fn collect_file_reference_checks(
    world_root: &Path,
    bundle: &LocalDataBundle,
) -> Vec<LocalMediaReferenceCheck> {
    let mut references = BTreeMap::<String, BTreeSet<String>>::new();
    for record in &bundle.markdown_records {
        collect_file_references(&record.attributes, &record.path, &mut references);
    }
    for document in &bundle.toml_documents {
        collect_file_references(&document.data, &document.path, &mut references);
    }

    references
        .into_iter()
        .map(|(path, referenced_by)| LocalMediaReferenceCheck {
            exists: world_root.join(&path).exists(),
            path,
            referenced_by: referenced_by.into_iter().collect(),
        })
        .collect()
}

fn collect_file_references(
    value: &impl serde::Serialize,
    referenced_by: &str,
    references: &mut BTreeMap<String, BTreeSet<String>>,
) {
    let Ok(value) = serde_json::to_value(value) else {
        return;
    };
    collect_file_references_from_json(&value, referenced_by, references);
}

fn collect_file_references_from_json(
    value: &serde_json::Value,
    referenced_by: &str,
    references: &mut BTreeMap<String, BTreeSet<String>>,
) {
    match value {
        serde_json::Value::Object(object) => {
            if let Some(file) = object
                .get("file")
                .or_else(|| object.get("path"))
                .and_then(serde_json::Value::as_str)
                .filter(|path| should_check_file_reference(path))
            {
                references
                    .entry(file.trim().to_string())
                    .or_default()
                    .insert(referenced_by.to_string());
            }

            for value in object.values() {
                collect_file_references_from_json(value, referenced_by, references);
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                collect_file_references_from_json(value, referenced_by, references);
            }
        }
        _ => {}
    }
}

fn should_check_file_reference(path: &str) -> bool {
    let path = path.trim();
    !path.is_empty()
        && !path.contains("://")
        && !path.starts_with('/')
        && !path.starts_with('#')
        && !path.starts_with("worlds/")
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::local_authoring::{LocalSkeletonOptions, create_workspace_skeleton};

    #[test]
    fn summarizes_standard_world() {
        let temp_dir = test_temp_dir("summary-standard");
        create_workspace_skeleton(
            &temp_dir,
            &LocalSkeletonOptions {
                birth_date: Some("1900-01-01".to_string()),
                ..LocalSkeletonOptions::default()
            },
        )
        .expect("skeleton");
        let world_root = temp_dir.join("worlds/default");

        let summary = summarize_local_world(&world_root).expect("summary");

        assert_eq!(summary.world_id.as_deref(), Some("world:default"));
        assert_eq!(summary.counts.people, 1);
        assert_eq!(summary.counts.events, 1);
        assert_eq!(summary.counts.events_by_type.get("birth"), Some(&1));
        assert_eq!(summary.counts.sources, 1);
        assert_eq!(summary.counts.timeline_views, 1);
        assert_eq!(summary.counts.tree_views, 1);
        assert!(
            !summary.warnings.iter().any(
                |warning| warning.kind == LocalWorldSummaryWarningKind::PersonMissingBirthEvent
            )
        );

        fs::remove_dir_all(temp_dir).expect("remove temp dir");
    }

    #[test]
    fn inline_assertion_sources_count_as_event_sources() {
        let temp_dir = test_temp_dir("inline-assertion-event-source");
        fs::create_dir_all(temp_dir.join("entities/people")).expect("people dir");
        fs::create_dir_all(temp_dir.join("events/births")).expect("events dir");
        fs::create_dir_all(temp_dir.join("sources")).expect("sources dir");
        fs::write(
            temp_dir.join("entities/people/alex-example.md"),
            "+++\nid = \"person:alex-example\"\nkind = \"person\"\npreferred_name = \"Alex Example\"\n+++\n\n# Alex\n",
        )
        .expect("person");
        fs::write(
            temp_dir.join("sources/personal-knowledge.md"),
            "+++\nid = \"source:personal-knowledge\"\nkind = \"note\"\ntitle = \"Personal knowledge\"\n+++\n\n# Source\n",
        )
        .expect("source");
        fs::write(
            temp_dir.join("events/births/birth-alex-example.md"),
            "+++\nid = \"event:birth-alex-example\"\nkind = \"event\"\ntype = \"birth\"\nsubject = \"alex-example\"\ntime = \"1900-01-01\"\n\n[[assertions]]\ntarget = \"#date\"\nsources = [\"personal-knowledge\"]\nconfidence = \"high\"\n+++\n\n# Birth\n",
        )
        .expect("event");

        let report = doctor_local_world(&temp_dir).expect("doctor");

        assert!(
            !report.diagnostics.iter().any(|diagnostic| {
                diagnostic.kind == LocalWorldDiagnosticKind::EventMissingSource
            })
        );

        fs::remove_dir_all(temp_dir).expect("remove temp dir");
    }

    #[test]
    fn reports_authoring_attention_items() {
        let temp_dir = test_temp_dir("summary-warnings");
        fs::create_dir_all(temp_dir.join("entities/people")).expect("people dir");
        fs::create_dir_all(temp_dir.join("events/observations")).expect("events dir");
        fs::create_dir_all(temp_dir.join("relationships")).expect("relationships dir");
        fs::write(
            temp_dir.join("entities/people/alex-example.md"),
            "+++\nid = \"person:alex-example\"\nkind = \"person\"\npreferred_name = \"Alex Example\"\n+++\n\n# Alex\n",
        )
        .expect("person");
        fs::write(
            temp_dir.join("events/observations/example.md"),
            "+++\nid = \"event:example\"\nkind = \"event\"\ntype = \"observation\"\nparticipants = [\"person:alex-example\"]\nsources = [{ label = \"Missing scan\", file = \"media/sources/missing.jpg\" }]\n+++\n\n# Example\n",
        )
        .expect("event");
        fs::write(
            temp_dir.join("relationships/example.toml"),
            "schema_version = 1\nid = \"relationship:example\"\nkind = \"relationship\"\ntitle = \"Example\"\nrelationship = \"associate\"\nsource = \"person:alex-example\"\ntarget = \"person:alex-example\"\n",
        )
        .expect("relationship");

        let summary = summarize_local_world(&temp_dir).expect("summary");
        let warning_kinds = summary
            .warnings
            .iter()
            .map(|warning| warning.kind)
            .collect::<BTreeSet<_>>();

        assert!(warning_kinds.contains(&LocalWorldSummaryWarningKind::PersonMissingBirthEvent));
        assert!(warning_kinds.contains(&LocalWorldSummaryWarningKind::EventMissingTime));
        assert!(warning_kinds.contains(&LocalWorldSummaryWarningKind::RelationshipMissingSource));
        assert!(warning_kinds.contains(&LocalWorldSummaryWarningKind::ReferencedFileMissing));
        assert!(
            !warning_kinds.contains(&LocalWorldSummaryWarningKind::EventMissingSource),
            "inline source should count as event evidence"
        );

        fs::remove_dir_all(temp_dir).expect("remove temp dir");
    }

    #[test]
    fn checks_local_media_file_references() {
        let temp_dir = test_temp_dir("check-media");
        fs::create_dir_all(temp_dir.join("entities/people")).expect("people dir");
        fs::create_dir_all(temp_dir.join("events/observations")).expect("events dir");
        fs::create_dir_all(temp_dir.join("media/sources")).expect("media dir");
        fs::write(temp_dir.join("media/sources/present.jpg"), "present").expect("media");
        fs::write(
            temp_dir.join("entities/people/alex-example.md"),
            "+++\nid = \"person:alex-example\"\nkind = \"person\"\npreferred_name = \"Alex Example\"\n+++\n\n# Alex\n",
        )
        .expect("person");
        fs::write(
            temp_dir.join("events/observations/example.md"),
            "+++\nid = \"event:example\"\nkind = \"event\"\ntype = \"observation\"\nparticipants = [\"person:alex-example\"]\nsources = [{ label = \"Present scan\", file = \"media/sources/present.jpg\" }, { label = \"Missing scan\", file = \"media/sources/missing.jpg\" }]\n+++\n\n# Event\n",
        )
        .expect("event");

        let report = check_local_media(&temp_dir).expect("media check");

        assert_eq!(report.referenced_files(), 2);
        assert_eq!(report.present_files(), 1);
        assert_eq!(report.missing_files(), 1);
        assert!(report.references.iter().any(|reference| {
            reference.path == "media/sources/present.jpg" && reference.exists
        }));
        assert!(report.references.iter().any(|reference| {
            reference.path == "media/sources/missing.jpg"
                && !reference.exists
                && reference
                    .referenced_by
                    .contains(&"events/observations/example.md".to_string())
        }));

        fs::remove_dir_all(temp_dir).expect("remove temp dir");
    }

    #[test]
    fn doctor_reports_additional_diagnostics() {
        let temp_dir = test_temp_dir("doctor-diagnostics");
        fs::create_dir_all(temp_dir.join("entities/people")).expect("people dir");
        fs::create_dir_all(temp_dir.join("events/observations")).expect("events dir");
        fs::create_dir_all(temp_dir.join("records")).expect("records dir");
        fs::write(
            temp_dir.join("records/unnamed.md"),
            "+++\nid = \"person:unnamed\"\nkind = \"person\"\n+++\n\n# Unnamed\n",
        )
        .expect("person");
        fs::write(
            temp_dir.join("events/observations/no-participant.md"),
            "+++\nid = \"event:no-participant\"\nkind = \"event\"\ntype = \"observation\"\ntime = \"1900-01-01\"\nsources = [{ label = \"Personal note\" }]\n+++\n\n# Event\n",
        )
        .expect("event");

        let report = doctor_local_world(&temp_dir).expect("doctor");
        let diagnostic_kinds = report
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.kind)
            .collect::<BTreeSet<_>>();

        assert!(diagnostic_kinds.contains(&LocalWorldDiagnosticKind::PersonMissingName));
        assert!(diagnostic_kinds.contains(&LocalWorldDiagnosticKind::PersonMissingBirthEvent));
        assert!(diagnostic_kinds.contains(&LocalWorldDiagnosticKind::EventMissingParticipant));

        fs::remove_dir_all(temp_dir).expect("remove temp dir");
    }

    #[test]
    fn doctor_reports_implausibly_small_parent_child_age_gap() {
        let temp_dir = test_temp_dir("doctor-parent-child-age-gap");
        fs::create_dir_all(temp_dir.join("entities/people")).expect("people dir");
        fs::create_dir_all(temp_dir.join("relationships")).expect("relationships dir");
        fs::write(
            temp_dir.join("entities/people/parent.md"),
            "+++\nid = \"person:parent\"\nkind = \"person\"\npreferred_name = \"Parent Example\"\nbirth_date = 1900-01-01\n+++\n\n# Parent\n",
        )
        .expect("parent");
        fs::write(
            temp_dir.join("entities/people/child.md"),
            "+++\nid = \"person:child\"\nkind = \"person\"\npreferred_name = \"Child Example\"\nbirth_date = 1908-01-01\n+++\n\n# Child\n",
        )
        .expect("child");
        fs::write(
            temp_dir.join("relationships/too-close.toml"),
            "schema_version = 1\nid = \"relationship:too-close\"\nkind = \"relationship\"\ntitle = \"Too close example\"\nrelationship = \"biological-parent-child\"\nsource = \"person:parent\"\ntarget = \"person:child\"\n",
        )
        .expect("relationship");

        let report = doctor_local_world(&temp_dir).expect("doctor");

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.kind == LocalWorldDiagnosticKind::SuspiciousParentChildDirection
                && diagnostic.record_id == "relationship:too-close"
        }));

        fs::remove_dir_all(temp_dir).expect("remove temp dir");
    }

    #[test]
    fn doctor_reports_suspicious_parent_child_direction() {
        let temp_dir = test_temp_dir("doctor-parent-child-direction");
        fs::create_dir_all(temp_dir.join("entities/people")).expect("people dir");
        fs::create_dir_all(temp_dir.join("relationships")).expect("relationships dir");
        fs::write(
            temp_dir.join("entities/people/older.md"),
            "+++\nid = \"person:older\"\nkind = \"person\"\npreferred_name = \"Older Example\"\nbirth_date = 1900-01-01\n+++\n\n# Older\n",
        )
        .expect("older");
        fs::write(
            temp_dir.join("entities/people/younger.md"),
            "+++\nid = \"person:younger\"\nkind = \"person\"\npreferred_name = \"Younger Example\"\nbirth_date = 1930-01-01\n+++\n\n# Younger\n",
        )
        .expect("younger");
        fs::write(
            temp_dir.join("relationships/reversed.toml"),
            "schema_version = 1\nid = \"relationship:reversed\"\nkind = \"relationship\"\ntitle = \"Reversed example\"\nrelationship = \"biological-parent-child\"\nsource = \"person:younger\"\ntarget = \"person:older\"\n",
        )
        .expect("relationship");

        let report = doctor_local_world(&temp_dir).expect("doctor");

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.kind == LocalWorldDiagnosticKind::SuspiciousParentChildDirection
                && diagnostic.record_id == "relationship:reversed"
        }));

        fs::remove_dir_all(temp_dir).expect("remove temp dir");
    }

    #[test]
    fn doctor_reports_possible_duplicate_people_by_name() {
        let temp_dir = test_temp_dir("doctor-duplicate-people");
        fs::create_dir_all(temp_dir.join("entities/people")).expect("people dir");
        for slug in ["alex-one", "alex-two"] {
            fs::write(
                temp_dir.join(format!("entities/people/{slug}.md")),
                format!(
                    "+++\nid = \"person:{slug}\"\nkind = \"person\"\npreferred_name = \"Alex Example\"\n+++\n\n# Alex\n"
                ),
            )
            .expect("person");
        }

        let report = doctor_local_world(&temp_dir).expect("doctor");
        let duplicate_ids = report
            .diagnostics
            .iter()
            .filter(|diagnostic| {
                diagnostic.kind == LocalWorldDiagnosticKind::PossibleDuplicatePerson
            })
            .map(|diagnostic| diagnostic.record_id.as_str())
            .collect::<BTreeSet<_>>();

        assert!(duplicate_ids.contains("person:alex-one"));
        assert!(duplicate_ids.contains("person:alex-two"));

        fs::remove_dir_all(temp_dir).expect("remove temp dir");
    }

    #[test]
    fn doctor_reports_records_outside_expected_directories() {
        let temp_dir = test_temp_dir("doctor-unexpected-path");
        fs::create_dir_all(temp_dir.join("records")).expect("records dir");
        fs::create_dir_all(temp_dir.join("entities/people")).expect("people dir");
        fs::create_dir_all(temp_dir.join("misc")).expect("misc dir");
        fs::write(
            temp_dir.join("records/alex-example.md"),
            "+++\nid = \"person:alex-example\"\nkind = \"person\"\npreferred_name = \"Alex Example\"\n+++\n\n# Alex\n",
        )
        .expect("person");
        fs::write(
            temp_dir.join("entities/people/morgan-example.md"),
            "+++\nid = \"person:morgan-example\"\nkind = \"person\"\npreferred_name = \"Morgan Example\"\n+++\n\n# Morgan\n",
        )
        .expect("person");
        fs::write(
            temp_dir.join("misc/alex-morgan.toml"),
            "schema_version = 1\nid = \"relationship:alex-morgan\"\nkind = \"relationship\"\ntitle = \"Example relationship\"\nrelationship = \"associate\"\nsource = \"person:alex-example\"\ntarget = \"person:morgan-example\"\n",
        )
        .expect("relationship");

        let report = doctor_local_world(&temp_dir).expect("doctor");
        let unexpected_paths = report
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.kind == LocalWorldDiagnosticKind::RecordUnexpectedPath)
            .map(|diagnostic| diagnostic.path.as_str())
            .collect::<BTreeSet<_>>();

        assert!(unexpected_paths.contains("records/alex-example.md"));
        assert!(unexpected_paths.contains("misc/alex-morgan.toml"));

        fs::remove_dir_all(temp_dir).expect("remove temp dir");
    }

    fn test_temp_dir(label: &str) -> std::path::PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "kleio-local-authoring-{label}-{}-{unique}",
            std::process::id()
        ))
    }
}
