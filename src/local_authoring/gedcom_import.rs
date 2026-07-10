use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use super::LocalAuthoringError;
use super::gedcom_parse::{
    MinimalGedcomDocument, MinimalGedcomEvent, MinimalGedcomIndividual, parse_gedcom_document,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrimaryGedcomImportOptions {
    pub path: String,
    pub strategy: String,
    pub allow_missing: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalGedcomIngestOptions {
    pub path: String,
    pub force: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LocalGedcomIngestReport {
    pub people: usize,
    pub places: usize,
    pub events: usize,
    pub assertions: usize,
    pub relationships: usize,
    pub sources: usize,
    pub skipped_existing: usize,
    pub warnings: Vec<String>,
    pub parser: String,
    pub import_report_path: Option<PathBuf>,
}

impl Default for PrimaryGedcomImportOptions {
    fn default() -> Self {
        Self {
            path: "imports/gedcom/family.ged".to_string(),
            strategy: "link".to_string(),
            allow_missing: false,
        }
    }
}

pub fn ingest_primary_gedcom_to_world(
    world_root: impl AsRef<Path>,
    options: &LocalGedcomIngestOptions,
) -> Result<LocalGedcomIngestReport, LocalAuthoringError> {
    let world_root = world_root.as_ref();
    let normalized_path = normalize_import_path(world_root, &options.path)?;
    ingest_gedcom_path_to_world(world_root, &normalized_path, options.force)
}

fn ingest_gedcom_path_to_world(
    world_root: &Path,
    relative_path: &Path,
    force: bool,
) -> Result<LocalGedcomIngestReport, LocalAuthoringError> {
    let path = world_root.join(relative_path);
    let text = fs::read_to_string(&path).map_err(|source| LocalAuthoringError::Io {
        path: path.clone(),
        source,
    })?;
    let parsed = parse_gedcom_document(&text);
    write_gedcom_document_to_world(
        world_root,
        &parsed.document,
        relative_path,
        force,
        &parsed.parser,
        parsed.warning,
    )
}

fn write_gedcom_document_to_world(
    world_root: &Path,
    document: &MinimalGedcomDocument,
    import_path: &Path,
    force: bool,
    parser: &str,
    parser_warning: Option<String>,
) -> Result<LocalGedcomIngestReport, LocalAuthoringError> {
    let mut report = LocalGedcomIngestReport {
        parser: parser.to_string(),
        ..Default::default()
    };
    if let Some(warning) = parser_warning {
        push_warning_once(&mut report, warning);
    }
    let source_slug = import_source_slug(import_path);
    let source_id = format!("source:{source_slug}");
    if write_new_file(
        world_root,
        &world_root.join("sources").join(format!("{source_slug}.md")),
        &source_markdown(&source_id, import_path),
        force,
    )? {
        report.sources += 1;
    } else {
        report.skipped_existing += 1;
    }

    let mut people = BTreeSet::new();
    for individual in document.individuals.values() {
        let slug = gedcom_xref_slug(&individual.xref);
        let id = format!("person:{slug}");
        if write_new_file(
            world_root,
            &world_root
                .join("entities/people")
                .join(format!("{slug}.md")),
            &person_markdown(&id, individual, &source_id),
            force,
        )? {
            report.people += 1;
        } else {
            report.skipped_existing += 1;
        }
        people.insert(individual.xref.clone());
    }

    let mut places = BTreeMap::<String, String>::new();
    for individual in document.individuals.values() {
        for event in &individual.events {
            if let Some(place) = event
                .place
                .as_deref()
                .filter(|place| !place.trim().is_empty())
            {
                let slug = place_slug(place);
                places.entry(place.to_string()).or_insert(slug);
            }
        }
    }
    for (name, slug) in &places {
        if write_new_file(
            world_root,
            &world_root
                .join("entities/places")
                .join(format!("{slug}.md")),
            &place_markdown(&format!("place:{slug}"), name, &source_id),
            force,
        )? {
            report.places += 1;
        } else {
            report.skipped_existing += 1;
        }
    }

    for individual in document.individuals.values() {
        let person_id = format!("person:{}", gedcom_xref_slug(&individual.xref));
        for (index, event) in individual.events.iter().enumerate() {
            let event_slug = safe_slug(&format!(
                "{}-{}-{}",
                event.date.as_deref().unwrap_or("unknown-date"),
                event.kind,
                gedcom_xref_slug(&individual.xref)
            ));
            let event_id = format!("event:{event_slug}");
            let assertion_write = write_event_assertions(
                world_root,
                &event_slug,
                event,
                &event_id,
                &person_id,
                &source_id,
                force,
            )?;
            report.assertions += assertion_write.created;
            report.skipped_existing += assertion_write.skipped_existing;
            if write_new_file(
                world_root,
                &world_root
                    .join("events")
                    .join(event_kind_dir(&event.kind))
                    .join(format!("{event_slug}-{index}.md")),
                &event_markdown(
                    &event_slug,
                    event,
                    &person_id,
                    &places,
                    &source_id,
                    &assertion_write.ids,
                ),
                force,
            )? {
                report.events += 1;
            } else {
                report.skipped_existing += 1;
            }
        }
    }

    for family in document.families.values() {
        for child in &family.children {
            if !people.contains(child) {
                push_warning_once(
                    &mut report,
                    format!(
                        "GEDCOM family references missing child `{child}`; parent-child relationships for that child were skipped"
                    ),
                );
                continue;
            }
            for parent in family.husband.iter().chain(family.wife.iter()) {
                if people.contains(parent) {
                    if write_relationship(
                        world_root,
                        parent,
                        child,
                        "biological-parent-child",
                        &source_id,
                        force,
                    )? {
                        report.relationships += 1;
                    } else {
                        report.skipped_existing += 1;
                    }
                } else {
                    push_warning_once(
                        &mut report,
                        format!(
                            "GEDCOM family references missing parent `{parent}` for child `{child}`; parent-child relationship was skipped"
                        ),
                    );
                }
            }
        }
        if let (Some(left), Some(right)) = (&family.husband, &family.wife) {
            if people.contains(left) && people.contains(right) {
                if write_relationship(world_root, left, right, "spouse", &source_id, force)? {
                    report.relationships += 1;
                } else {
                    report.skipped_existing += 1;
                }
            } else {
                push_warning_once(
                    &mut report,
                    format!(
                        "GEDCOM family spouse link `{left}`/`{right}` references a missing person; spouse relationship was skipped"
                    ),
                );
            }
        }
    }

    write_gedcom_import_report(world_root, import_path, &source_id, &mut report)?;

    Ok(report)
}

pub fn set_primary_gedcom_import(
    world_root: impl AsRef<Path>,
    options: &PrimaryGedcomImportOptions,
) -> Result<(), LocalAuthoringError> {
    let world_root = world_root.as_ref();
    let config_path = world_root.join("world.toml");
    let normalized_path = normalize_import_path(world_root, &options.path)?;

    if options.strategy.trim().is_empty() {
        return Err(LocalAuthoringError::Validation {
            message: "GEDCOM import strategy cannot be empty".to_string(),
        });
    }

    if !options.allow_missing && !world_root.join(&normalized_path).exists() {
        return Err(LocalAuthoringError::Validation {
            message: format!(
                "GEDCOM import `{}` does not exist; pass --allow-missing to link it anyway",
                normalized_path.display()
            ),
        });
    }

    let text = fs::read_to_string(&config_path).map_err(|source| LocalAuthoringError::Io {
        path: config_path.clone(),
        source,
    })?;
    let mut table = text
        .parse::<toml::Table>()
        .map_err(|source| LocalAuthoringError::Toml {
            path: config_path.clone(),
            source,
        })?;

    set_nested_string(
        &mut table,
        &["imports", "gedcom", "primary", "path"],
        &relative_path_to_string(&normalized_path),
    );
    set_nested_string(
        &mut table,
        &["imports", "gedcom", "primary", "strategy"],
        options.strategy.trim(),
    );

    let toml =
        toml::to_string_pretty(&table).map_err(|source| LocalAuthoringError::TomlSerialize {
            path: config_path.clone(),
            source,
        })?;
    fs::write(&config_path, format!("{toml}\n")).map_err(|source| LocalAuthoringError::Io {
        path: config_path,
        source,
    })?;

    Ok(())
}

fn push_warning_once(report: &mut LocalGedcomIngestReport, warning: String) {
    if !report.warnings.iter().any(|existing| existing == &warning) {
        report.warnings.push(warning);
    }
}

fn write_gedcom_import_report(
    world_root: &Path,
    import_path: &Path,
    source_id: &str,
    report: &mut LocalGedcomIngestReport,
) -> Result<(), LocalAuthoringError> {
    let slug = safe_slug(
        import_path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("gedcom-import"),
    );
    let path = world_root
        .join("imports/gedcom")
        .join(format!("{slug}-report.toml"));
    let content = gedcom_import_report_toml(import_path, source_id, report);
    write_new_file(world_root, &path, &content, true)?;
    report.import_report_path = Some(path.strip_prefix(world_root).unwrap_or(&path).to_path_buf());
    Ok(())
}

fn gedcom_import_report_toml(
    import_path: &Path,
    source_id: &str,
    report: &LocalGedcomIngestReport,
) -> String {
    let warnings = toml_string_array(&report.warnings);
    format!(
        "schema_version = 1\nid = \"import:gedcom:{}\"\nkind = \"gedcom-import-report\"\ntitle = \"GEDCOM import report for {}\"\nsource_path = \"{}\"\nsource = \"{}\"\nstrategy = \"ingest\"\nstatus = \"completed\"\nparser = \"{}\"\nskipped_existing = {}\nwarnings = {warnings}\n\n[created]\npeople = {}\nplaces = {}\nevents = {}\nassertions = {}\nrelationships = {}\nsources = {}\n\n[updated]\npeople = 0\nplaces = 0\nevents = 0\nassertions = 0\nrelationships = 0\nsources = 0\n\n[notes]\nsummary = \"Generated by Kleio GEDCOM ingestion. Reconciliation and merge/update behavior remain future work.\"\n",
        escape_toml_basic(&import_source_slug(import_path)),
        escape_toml_basic(&relative_path_to_string(import_path)),
        escape_toml_basic(&relative_path_to_string(import_path)),
        escape_toml_basic(source_id),
        escape_toml_basic(&report.parser),
        report.skipped_existing,
        report.people,
        report.places,
        report.events,
        report.assertions,
        report.relationships,
        report.sources,
    )
}

fn write_relationship(
    world_root: &Path,
    source: &str,
    target: &str,
    relationship: &str,
    source_id: &str,
    force: bool,
) -> Result<bool, LocalAuthoringError> {
    let source_slug = gedcom_xref_slug(source);
    let target_slug = gedcom_xref_slug(target);
    let slug = safe_slug(&format!("{relationship}-{source_slug}-{target_slug}"));
    write_new_file(
        world_root,
        &world_root
            .join("relationships")
            .join(format!("{slug}.toml")),
        &format!(
            "schema_version = 1\nid = \"relationship:{slug}\"\nkind = \"relationship\"\ntitle = \"{} relationship\"\nrelationship = \"{relationship}\"\nsource = \"person:{source_slug}\"\ntarget = \"person:{target_slug}\"\nsources = [\"{}\"]\n",
            relationship.replace('-', " "),
            escape_toml_basic(source_id),
        ),
        force,
    )
}

fn write_new_file(
    root: &Path,
    path: &Path,
    content: &str,
    force: bool,
) -> Result<bool, LocalAuthoringError> {
    if path.exists() && !force {
        return Ok(false);
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| LocalAuthoringError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    fs::write(path, content)
        .map(|_| true)
        .map_err(|source| LocalAuthoringError::Io {
            path: path.strip_prefix(root).unwrap_or(path).to_path_buf(),
            source,
        })
}

fn source_markdown(id: &str, import_path: &Path) -> String {
    format!(
        "+++\nschema_version = 1\nid = \"{}\"\nkind = \"gedcom-import\"\ntitle = \"GEDCOM import {}\"\nmedia = []\n+++\n\nGenerated from `{}`.\n",
        escape_toml_basic(id),
        escape_toml_basic(&relative_path_to_string(import_path)),
        escape_toml_basic(&relative_path_to_string(import_path)),
    )
}

fn person_markdown(id: &str, individual: &MinimalGedcomIndividual, source_id: &str) -> String {
    let name = individual
        .name
        .as_deref()
        .map(clean_gedcom_name)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| individual.xref.clone());
    let mut fields = format!(
        "schema_version = 1\nid = \"{}\"\nkind = \"person\"\nprimary_name = \"{}\"\nsources = [\"{}\"]\n\n[metadata]\ngedcom_xref = \"{}\"\n",
        escape_toml_basic(id),
        escape_toml_basic(&name),
        escape_toml_basic(source_id),
        escape_toml_basic(&individual.xref),
    );
    if let Some(sex) = &individual.sex {
        fields.push_str(&format!("sex = \"{}\"\n", escape_toml_basic(sex)));
    }
    format!(
        "+++\n{fields}+++\n\nImported from GEDCOM. Review and enrich this person record.{}\n",
        notes_markdown_block(&individual.notes)
    )
}

fn place_markdown(id: &str, name: &str, source_id: &str) -> String {
    format!(
        "+++\nschema_version = 1\nid = \"{}\"\nkind = \"place\"\nprimary_name = \"{}\"\nsources = [\"{}\"]\n+++\n\nImported from GEDCOM place text.\n",
        escape_toml_basic(id),
        escape_toml_basic(name),
        escape_toml_basic(source_id),
    )
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct AssertionWriteResult {
    ids: Vec<String>,
    created: usize,
    skipped_existing: usize,
}

fn write_event_assertions(
    world_root: &Path,
    event_slug: &str,
    event: &MinimalGedcomEvent,
    event_id: &str,
    person_id: &str,
    source_id: &str,
    force: bool,
) -> Result<AssertionWriteResult, LocalAuthoringError> {
    let mut result = AssertionWriteResult::default();
    if let Some(date) = event.date.as_deref().filter(|date| !date.trim().is_empty()) {
        let slug = safe_slug(&format!("gedcom-{event_slug}-date"));
        let assertion_id = format!("assertion:{slug}");
        if write_new_file(
            world_root,
            &world_root.join("assertions").join(format!("{slug}.md")),
            &assertion_markdown(
                &assertion_id,
                &format!("{}-date", event.kind),
                event_id,
                &format!("{}_date", event.kind.replace('-', "_")),
                date,
                person_id,
                source_id,
            ),
            force,
        )? {
            result.created += 1;
        } else {
            result.skipped_existing += 1;
        }
        result.ids.push(assertion_id);
    }

    if let Some(place) = event
        .place
        .as_deref()
        .filter(|place| !place.trim().is_empty())
    {
        let slug = safe_slug(&format!("gedcom-{event_slug}-place"));
        let assertion_id = format!("assertion:{slug}");
        if write_new_file(
            world_root,
            &world_root.join("assertions").join(format!("{slug}.md")),
            &assertion_markdown(
                &assertion_id,
                &format!("{}-place", event.kind),
                event_id,
                &format!("{}_place", event.kind.replace('-', "_")),
                place,
                person_id,
                source_id,
            ),
            force,
        )? {
            result.created += 1;
        } else {
            result.skipped_existing += 1;
        }
        result.ids.push(assertion_id);
    }

    Ok(result)
}

fn assertion_markdown(
    id: &str,
    kind: &str,
    subject: &str,
    predicate: &str,
    value: &str,
    person_id: &str,
    source_id: &str,
) -> String {
    format!(
        "+++\nschema_version = 1\nid = \"{}\"\nkind = \"{}\"\nsubject = \"{}\"\npredicate = \"{}\"\nvalue = \"{}\"\nsources = [\"{}\"]\nconfidence = \"medium\"\n\n[metadata]\ngedcom_person = \"{}\"\n+++\n\nGenerated from a GEDCOM event fact. Review source citation details.\n",
        escape_toml_basic(id),
        escape_toml_basic(kind),
        escape_toml_basic(subject),
        escape_toml_basic(predicate),
        escape_toml_basic(value),
        escape_toml_basic(source_id),
        escape_toml_basic(person_id),
    )
}

fn event_markdown(
    slug: &str,
    event: &MinimalGedcomEvent,
    person_id: &str,
    places: &BTreeMap<String, String>,
    source_id: &str,
    assertions: &[String],
) -> String {
    let time = event
        .date
        .as_deref()
        .map(|date| format!("time = \"{}\"\n", escape_toml_basic(date)))
        .unwrap_or_default();
    let place = event
        .place
        .as_deref()
        .and_then(|place| places.get(place).map(|slug| format!("place:{slug}")));
    let places = place
        .as_deref()
        .map(|place_id| {
            format!(
                "places = [{{ entity = \"{}\", role = \"place\" }}]\n",
                escape_toml_basic(place_id)
            )
        })
        .unwrap_or_else(|| "places = []\n".to_string());
    let assertions = toml_string_array(assertions);
    format!(
        "+++\nschema_version = 1\nid = \"event:{}\"\nkind = \"{}\"\ntitle = \"{} event\"\n{}participants = [{{ entity = \"{}\", role = \"subject\" }}]\n{}assertions = {assertions}\nsources = [\"{}\"]\n+++\n\nImported from GEDCOM.{}\n",
        escape_toml_basic(slug),
        escape_toml_basic(&event.kind),
        escape_toml_basic(&event.kind),
        time,
        escape_toml_basic(person_id),
        places,
        escape_toml_basic(source_id),
        notes_markdown_block(&event.notes),
    )
}

fn notes_markdown_block(notes: &[String]) -> String {
    if notes.is_empty() {
        return String::new();
    }

    let mut block = String::from("\n\n## GEDCOM notes\n");
    for note in notes {
        block.push_str("\n- ");
        block.push_str(note.trim());
    }
    block
}

fn toml_string_array(values: &[String]) -> String {
    let items = values
        .iter()
        .map(|value| format!("\"{}\"", escape_toml_basic(value)))
        .collect::<Vec<_>>()
        .join(", ");
    format!("[{items}]")
}

fn event_kind_dir(kind: &str) -> &str {
    match kind {
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

fn normalize_import_path(root: &Path, path: &str) -> Result<PathBuf, LocalAuthoringError> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err(LocalAuthoringError::Validation {
            message: "GEDCOM import path cannot be empty".to_string(),
        });
    }

    let path = Path::new(trimmed);
    let relative = if path.is_absolute() {
        path.strip_prefix(root)
            .map_err(|_| LocalAuthoringError::Validation {
                message: format!(
                    "absolute GEDCOM path `{}` is not under world root `{}`",
                    path.display(),
                    root.display()
                ),
            })?
    } else {
        path
    };

    if relative.components().any(|component| {
        matches!(
            component,
            std::path::Component::ParentDir | std::path::Component::RootDir
        )
    }) {
        return Err(LocalAuthoringError::Validation {
            message: format!(
                "GEDCOM import path `{}` must stay inside the world root",
                relative.display()
            ),
        });
    }

    Ok(relative.to_path_buf())
}

fn set_nested_string(table: &mut toml::Table, path: &[&str], value: &str) {
    let Some((last, parents)) = path.split_last() else {
        return;
    };

    let mut current = table;
    for key in parents {
        if !current.contains_key(*key) {
            current.insert((*key).to_string(), toml::Value::Table(toml::Table::new()));
        }
        if !current.get(*key).is_some_and(toml::Value::is_table) {
            current.insert((*key).to_string(), toml::Value::Table(toml::Table::new()));
        }
        current = current
            .get_mut(*key)
            .and_then(toml::Value::as_table_mut)
            .expect("table inserted above");
    }

    current.insert((*last).to_string(), toml::Value::String(value.to_string()));
}

fn import_source_slug(path: &Path) -> String {
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("gedcom-import");
    safe_slug(&format!("gedcom-{stem}"))
}

fn gedcom_xref_slug(xref: &str) -> String {
    safe_slug(xref.trim_matches('@'))
}

fn place_slug(name: &str) -> String {
    safe_slug(&format!("gedcom-place-{name}"))
}

fn safe_slug(value: &str) -> String {
    let mut slug = String::new();
    let mut last_dash = false;
    for ch in value.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            last_dash = false;
        } else if !last_dash {
            slug.push('-');
            last_dash = true;
        }
    }
    slug.trim_matches('-').to_string()
}

fn clean_gedcom_name(value: &str) -> String {
    value
        .replace('/', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn escape_toml_basic(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn relative_path_to_string(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}
