//! Experimental streaming import helpers for Wikidata truthy dumps.
//!
//! This module deliberately implements only the small slice of N-Triples parsing
//! needed for an exploratory Kleio ETL path. It is not a general RDF engine.
//! Compressed-dump processing lives in the `wikidata_import` example, where it
//! can use dev-dependencies without becoming part of Kleio's released library API.

use std::collections::{BTreeMap, HashSet};
use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::archive::archive_genealogy_archive;
use crate::attribution::{Attribute, Provenance, SourceRef, Tag};
use crate::model::{
    DateValue, Event, EventId, EventKind, Family, FamilyId, GenealogyIndex, Name, Note, Person,
    PersonId, Place, PlaceId, Sex,
};
#[cfg(not(target_arch = "wasm32"))]
use crate::store::GenealogyStore;

const WIKIDATA_ENTITY_PREFIX: &str = "http://www.wikidata.org/entity/";
const WIKIDATA_ENTITY_HTTPS_PREFIX: &str = "https://www.wikidata.org/entity/";
const WIKIDATA_DIRECT_PROP_PREFIX: &str = "http://www.wikidata.org/prop/direct/";
const WIKIDATA_DIRECT_PROP_HTTPS_PREFIX: &str = "https://www.wikidata.org/prop/direct/";

pub const HUMAN_QID: &str = "Q5";
pub const DEFAULT_DUMP_PATH: &str = "vendor/latest-truthy.nt.bz2";
pub const DEFAULT_OUTPUT_PATH: &str = "target/wikidata-sample.ndjson";
pub const DEFAULT_CLOSURE_OUTPUT_PATH: &str = "target/wikidata-closure.ndjson";
pub const DEFAULT_DRAFT_OUTPUT_PATH: &str = "target/wikidata-person-drafts.ndjson";
pub const DEFAULT_KLEIO_ARCHIVE_PATH: &str = "target/wikidata-kleio.rkyv";
pub const DEFAULT_LABEL_SEEDS_PATH: &str = "target/wikidata-label-seeds.txt";
pub const DEFAULT_MAX_LINES: u64 = 100_000;
pub const DEFAULT_MAX_FACTS: u64 = 10_000;
pub const DEFAULT_PROGRESS_EVERY: u64 = 100_000;

const RELEVANT_PROPERTIES: &[&str] = &[
    "P31",   // instance of
    "P569",  // date of birth
    "P570",  // date of death
    "P19",   // place of birth
    "P20",   // place of death
    "P22",   // father
    "P25",   // mother
    "P26",   // spouse
    "P40",   // child
    "P3373", // sibling
    "P735",  // given name
    "P734",  // family name
    "P106",  // occupation
    "P21",   // sex or gender
    "P27",   // country of citizenship
    "P625",  // coordinate location
];

pub fn property_name(pid: &str) -> Option<&'static str> {
    match pid {
        "P31" => Some("instance_of"),
        "P569" => Some("date_of_birth"),
        "P570" => Some("date_of_death"),
        "P19" => Some("place_of_birth"),
        "P20" => Some("place_of_death"),
        "P22" => Some("father"),
        "P25" => Some("mother"),
        "P26" => Some("spouse"),
        "P40" => Some("child"),
        "P3373" => Some("sibling"),
        "P735" => Some("given_name"),
        "P734" => Some("family_name"),
        "P106" => Some("occupation"),
        "P21" => Some("sex_or_gender"),
        "P27" => Some("country_of_citizenship"),
        "P625" => Some("coordinate_location"),
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WikidataValueKind {
    EntityQid,
    Literal,
    DateTime,
    Coordinate,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WikidataFact {
    pub subject_qid: String,
    pub property_pid: String,
    pub property_name: String,
    pub value: String,
    pub value_kind: WikidataValueKind,
    pub line_number: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedTriple {
    pub subject_qid: String,
    pub property_pid: String,
    pub value: String,
    pub value_kind: WikidataValueKind,
}

#[derive(Debug, Clone)]
pub struct TruthyImportOptions {
    pub dump_path: PathBuf,
    pub output_path: PathBuf,
    pub max_lines: u64,
    pub max_facts: u64,
    pub progress_every: u64,
    pub subject: Option<String>,
    pub stop_after_subject: bool,
}

impl Default for TruthyImportOptions {
    fn default() -> Self {
        Self {
            dump_path: PathBuf::from(DEFAULT_DUMP_PATH),
            output_path: PathBuf::from(DEFAULT_OUTPUT_PATH),
            max_lines: DEFAULT_MAX_LINES,
            max_facts: DEFAULT_MAX_FACTS,
            progress_every: DEFAULT_PROGRESS_EVERY,
            subject: None,
            stop_after_subject: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TruthyImportReport {
    pub lines_read: u64,
    pub facts_written: u64,
    pub humans_detected: u64,
    pub stopped_after_subject: bool,
    pub output_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct WikidataClosureOptions {
    pub dump_path: PathBuf,
    pub seed_path: PathBuf,
    pub output_path: PathBuf,
    pub max_lines: u64,
    pub max_facts: u64,
    pub progress_every: u64,
}

impl Default for WikidataClosureOptions {
    fn default() -> Self {
        Self {
            dump_path: PathBuf::from(DEFAULT_DUMP_PATH),
            seed_path: PathBuf::from(DEFAULT_OUTPUT_PATH),
            output_path: PathBuf::from(DEFAULT_CLOSURE_OUTPUT_PATH),
            max_lines: DEFAULT_MAX_LINES,
            max_facts: DEFAULT_MAX_FACTS,
            progress_every: DEFAULT_PROGRESS_EVERY,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikidataClosureReport {
    pub seed_qids: usize,
    pub seed_subjects_seen: usize,
    pub lines_read: u64,
    pub facts_written: u64,
    pub humans_detected: u64,
    pub output_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct WikidataDraftOptions {
    pub input_path: PathBuf,
    pub output_path: PathBuf,
    pub label_cache_path: Option<PathBuf>,
}

impl Default for WikidataDraftOptions {
    fn default() -> Self {
        Self {
            input_path: PathBuf::from(DEFAULT_OUTPUT_PATH),
            output_path: PathBuf::from(DEFAULT_DRAFT_OUTPUT_PATH),
            label_cache_path: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikidataDraftReport {
    pub facts_read: u64,
    pub drafts_written: u64,
    pub humans_written: u64,
    pub labels_loaded: usize,
    pub output_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct WikidataKleioOptions {
    pub input_path: PathBuf,
    pub output_path: PathBuf,
    pub include_non_humans: bool,
}

impl Default for WikidataKleioOptions {
    fn default() -> Self {
        Self {
            input_path: PathBuf::from(DEFAULT_DRAFT_OUTPUT_PATH),
            output_path: PathBuf::from(DEFAULT_KLEIO_ARCHIVE_PATH),
            include_non_humans: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikidataKleioReport {
    pub drafts_read: u64,
    pub people_written: u64,
    pub events_written: u64,
    pub families_written: u64,
    pub places_written: u64,
    pub output_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikidataKleioInspectReport {
    pub path: PathBuf,
    pub people: usize,
    pub events: usize,
    pub families: usize,
    pub places: usize,
    pub notes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikidataDraftSummaryReport {
    pub input_path: PathBuf,
    pub drafts_read: u64,
    pub humans: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum PlaceRole {
    Birth,
    Death,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct WikidataPersonDraft {
    pub qid: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub labels: BTreeMap<String, String>,
    pub is_human: bool,
    pub instance_of_qids: Vec<String>,
    pub birth_dates: Vec<String>,
    pub death_dates: Vec<String>,
    pub birth_place_qids: Vec<String>,
    pub death_place_qids: Vec<String>,
    pub father_qids: Vec<String>,
    pub mother_qids: Vec<String>,
    pub spouse_qids: Vec<String>,
    pub child_qids: Vec<String>,
    pub sibling_qids: Vec<String>,
    pub given_name_qids: Vec<String>,
    pub family_name_qids: Vec<String>,
    pub occupation_qids: Vec<String>,
    pub sex_or_gender_qids: Vec<String>,
    pub citizenship_qids: Vec<String>,
    pub coordinate_locations: Vec<String>,
    pub source_fact_count: u64,
}

pub fn whitelisted_properties() -> HashSet<&'static str> {
    RELEVANT_PROPERTIES.iter().copied().collect()
}

pub fn is_relevant_property(pid: &str) -> bool {
    property_name(pid).is_some()
}

pub fn build_person_drafts(options: &WikidataDraftOptions) -> io::Result<WikidataDraftReport> {
    let input = File::open(&options.input_path)?;
    let reader = BufReader::with_capacity(1024 * 1024, input);
    let labels = options
        .label_cache_path
        .as_deref()
        .map(load_label_cache)
        .transpose()?
        .unwrap_or_default();

    let mut drafts: BTreeMap<String, WikidataPersonDraft> = BTreeMap::new();
    let mut facts_read = 0_u64;

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let fact: WikidataFact = serde_json::from_str(&line).map_err(io::Error::other)?;
        facts_read += 1;
        record_fact_into_drafts(&mut drafts, &fact);
    }

    prepare_output_parent(&options.output_path)?;
    let output = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&options.output_path)?;
    let mut writer = BufWriter::with_capacity(1024 * 1024, output);

    let mut drafts_written = 0_u64;
    let mut humans_written = 0_u64;
    for draft in drafts.values_mut() {
        apply_labels_to_draft(draft, &labels);
        if draft.is_human {
            humans_written += 1;
        }
        serde_json::to_writer(&mut writer, draft).map_err(io::Error::other)?;
        writer.write_all(b"\n")?;
        drafts_written += 1;
    }
    writer.flush()?;

    eprintln!(
        "wikidata-draft: read {facts_read} facts, wrote {drafts_written} person drafts ({humans_written} humans, labels_loaded={}) to {}",
        labels.len(),
        options.output_path.display()
    );

    Ok(WikidataDraftReport {
        facts_read,
        drafts_written,
        humans_written,
        labels_loaded: labels.len(),
        output_path: options.output_path.clone(),
    })
}

pub fn load_label_cache(path: &Path) -> io::Result<BTreeMap<String, String>> {
    let input = File::open(path)?;
    serde_json::from_reader(input).map_err(io::Error::other)
}

pub fn write_label_seeds_from_facts(input_path: &Path, output_path: &Path) -> io::Result<usize> {
    let qids = collect_referenced_qids_from_facts(input_path)?;
    prepare_output_parent(output_path)?;
    let output = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(output_path)?;
    let mut writer = BufWriter::new(output);
    let mut sorted_qids = qids.into_iter().collect::<Vec<_>>();
    sorted_qids.sort();
    for qid in &sorted_qids {
        writeln!(writer, "{qid}")?;
    }
    writer.flush()?;
    Ok(sorted_qids.len())
}

pub fn write_kleio_archive_from_drafts(
    options: &WikidataKleioOptions,
) -> io::Result<WikidataKleioReport> {
    let input = File::open(&options.input_path)?;
    let reader = BufReader::with_capacity(1024 * 1024, input);

    let mut drafts = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let draft: WikidataPersonDraft = serde_json::from_str(&line).map_err(io::Error::other)?;
        if options.include_non_humans || draft.is_human {
            drafts.push(draft);
        }
    }

    let drafts_read = drafts.len() as u64;
    let index = build_genealogy_index_from_drafts(&drafts);
    let report = WikidataKleioReport {
        drafts_read,
        people_written: index.people.len() as u64,
        events_written: index.events.len() as u64,
        families_written: index.families.len() as u64,
        places_written: index.places.len() as u64,
        output_path: options.output_path.clone(),
    };
    let archive = index.to_archive();
    let bytes = archive_genealogy_archive(&archive).map_err(io::Error::other)?;
    prepare_output_parent(&options.output_path)?;
    std::fs::write(&options.output_path, bytes)?;

    eprintln!(
        "wikidata-kleio: wrote {} people, {} events, {} families, {} places from {} drafts to {}",
        report.people_written,
        report.events_written,
        report.families_written,
        report.places_written,
        report.drafts_read,
        report.output_path.display()
    );

    Ok(report)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn inspect_kleio_archive(path: &Path) -> io::Result<WikidataKleioInspectReport> {
    let store = GenealogyStore::from_file(path).map_err(io::Error::other)?;
    let archived = store.archived().map_err(io::Error::other)?;
    let report = WikidataKleioInspectReport {
        path: path.to_path_buf(),
        people: archived.people.len(),
        events: archived.events.len(),
        families: archived.families.len(),
        places: archived.places.len(),
        notes: archived.notes.len(),
    };

    eprintln!(
        "wikidata-kleio inspect: {} people={} events={} families={} places={} notes={}",
        report.path.display(),
        report.people,
        report.events,
        report.families,
        report.places,
        report.notes
    );

    Ok(report)
}

pub fn summarize_person_drafts(
    path: &Path,
    limit: usize,
) -> io::Result<WikidataDraftSummaryReport> {
    let input = File::open(path)?;
    let reader = BufReader::with_capacity(1024 * 1024, input);
    let mut drafts_read = 0_u64;
    let mut humans = 0_u64;
    let mut aggregate = DraftAggregateSummary::default();
    let mut examples = Vec::new();

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let draft: WikidataPersonDraft = serde_json::from_str(&line).map_err(io::Error::other)?;
        drafts_read += 1;
        if draft.is_human {
            humans += 1;
        }
        aggregate.record(&draft);
        if examples.len() < limit {
            examples.push(draft);
        }
    }

    eprintln!(
        "wikidata-drafts summary: {} drafts ({} humans) from {}",
        drafts_read,
        humans,
        path.display()
    );
    aggregate.print();
    if !examples.is_empty() {
        eprintln!("wikidata-drafts examples:");
        for draft in &examples {
            eprintln!(
                "  {}{} facts={} birth_dates={} death_dates={} parents={} spouses={} children={} occupations={}",
                draft.qid,
                draft
                    .label
                    .as_ref()
                    .map(|label| format!(" ({label})"))
                    .unwrap_or_default(),
                draft.source_fact_count,
                draft.birth_dates.len(),
                draft.death_dates.len(),
                draft.father_qids.len() + draft.mother_qids.len(),
                draft.spouse_qids.len(),
                draft.child_qids.len(),
                draft.occupation_qids.len(),
            );
        }
    }

    Ok(WikidataDraftSummaryReport {
        input_path: path.to_path_buf(),
        drafts_read,
        humans,
    })
}

#[derive(Debug, Default)]
struct DraftAggregateSummary {
    with_label: u64,
    with_birth_date: u64,
    with_death_date: u64,
    with_birth_place: u64,
    with_death_place: u64,
    with_parent: u64,
    with_spouse: u64,
    with_child: u64,
    with_sibling: u64,
    with_occupation: u64,
    total_occupation_values: u64,
    total_label_values: u64,
}

impl DraftAggregateSummary {
    fn record(&mut self, draft: &WikidataPersonDraft) {
        self.with_label += u64::from(draft.label.is_some());
        self.with_birth_date += u64::from(!draft.birth_dates.is_empty());
        self.with_death_date += u64::from(!draft.death_dates.is_empty());
        self.with_birth_place += u64::from(!draft.birth_place_qids.is_empty());
        self.with_death_place += u64::from(!draft.death_place_qids.is_empty());
        self.with_parent +=
            u64::from(!draft.father_qids.is_empty() || !draft.mother_qids.is_empty());
        self.with_spouse += u64::from(!draft.spouse_qids.is_empty());
        self.with_child += u64::from(!draft.child_qids.is_empty());
        self.with_sibling += u64::from(!draft.sibling_qids.is_empty());
        self.with_occupation += u64::from(!draft.occupation_qids.is_empty());
        self.total_occupation_values += draft.occupation_qids.len() as u64;
        self.total_label_values += draft.labels.len() as u64;
    }

    fn print(&self) {
        eprintln!("  with_label={}", self.with_label);
        eprintln!("  with_birth_date={}", self.with_birth_date);
        eprintln!("  with_death_date={}", self.with_death_date);
        eprintln!("  with_birth_place={}", self.with_birth_place);
        eprintln!("  with_death_place={}", self.with_death_place);
        eprintln!("  with_parent={}", self.with_parent);
        eprintln!("  with_spouse={}", self.with_spouse);
        eprintln!("  with_child={}", self.with_child);
        eprintln!("  with_sibling={}", self.with_sibling);
        eprintln!("  with_occupation={}", self.with_occupation);
        eprintln!("  total_occupation_values={}", self.total_occupation_values);
        eprintln!("  total_label_values={}", self.total_label_values);
    }
}

fn build_genealogy_index_from_drafts(drafts: &[WikidataPersonDraft]) -> GenealogyIndex {
    let mut people = Vec::new();
    let mut events = Vec::new();
    let mut families = Vec::new();
    let notes = Vec::<Note>::new();
    let mut places = Vec::new();
    let mut person_ids_by_qid = BTreeMap::new();
    let mut place_ids_by_key = BTreeMap::<(String, PlaceRole), PlaceId>::new();
    let mut next_person_id = 1_u64;
    let mut next_event_id = 1_u64;
    let mut next_family_id = 1_u64;
    let mut next_place_id = 1_u64;

    for draft in drafts {
        person_ids_by_qid.insert(draft.qid.clone(), PersonId(next_person_id));
        next_person_id += 1;
    }

    for draft in drafts {
        let person_id = person_ids_by_qid[&draft.qid];
        let mut person_events = Vec::new();
        let mut families_as_child = Vec::new();
        let mut families_as_spouse = Vec::new();

        for birth_date in &draft.birth_dates {
            let event_id = EventId(next_event_id);
            next_event_id += 1;
            let place = first_place_for_role(
                draft,
                PlaceRole::Birth,
                &mut places,
                &mut place_ids_by_key,
                &mut next_place_id,
            );
            events.push(Event {
                id: event_id,
                kind: EventKind::Birth,
                date: Some(DateValue::from_original(
                    birth_date.clone(),
                    wikidata_provenance(&draft.qid),
                )),
                time: None,
                time_zone: None,
                place,
                description: None,
                participants: vec![person_id],
                provenance: wikidata_provenance(&draft.qid),
            });
            person_events.push(event_id);
        }

        for death_date in &draft.death_dates {
            let event_id = EventId(next_event_id);
            next_event_id += 1;
            let place = first_place_for_role(
                draft,
                PlaceRole::Death,
                &mut places,
                &mut place_ids_by_key,
                &mut next_place_id,
            );
            events.push(Event {
                id: event_id,
                kind: EventKind::Death,
                date: Some(DateValue::from_original(
                    death_date.clone(),
                    wikidata_provenance(&draft.qid),
                )),
                time: None,
                time_zone: None,
                place,
                description: None,
                participants: vec![person_id],
                provenance: wikidata_provenance(&draft.qid),
            });
            person_events.push(event_id);
        }

        for occupation_qid in &draft.occupation_qids {
            let event_id = EventId(next_event_id);
            next_event_id += 1;
            events.push(Event {
                id: event_id,
                kind: EventKind::Occupation,
                date: None,
                time: None,
                time_zone: None,
                place: None,
                description: Some(label_or_qid(draft, occupation_qid)),
                participants: vec![person_id],
                provenance: wikidata_provenance(&draft.qid),
            });
            person_events.push(event_id);
        }

        for parent_qid in draft.father_qids.iter().chain(&draft.mother_qids) {
            if let Some(parent_id) = person_ids_by_qid.get(parent_qid).copied() {
                let family_id = FamilyId(next_family_id);
                next_family_id += 1;
                families.push(Family {
                    id: family_id,
                    spouses: vec![parent_id],
                    children: vec![person_id],
                    events: Vec::new(),
                    provenance: wikidata_provenance(&draft.qid),
                });
                families_as_child.push(family_id);
            }
        }

        for spouse_qid in &draft.spouse_qids {
            if let Some(spouse_id) = person_ids_by_qid.get(spouse_qid).copied() {
                let family_id = FamilyId(next_family_id);
                next_family_id += 1;
                families.push(Family {
                    id: family_id,
                    spouses: vec![person_id, spouse_id],
                    children: Vec::new(),
                    events: Vec::new(),
                    provenance: wikidata_provenance(&draft.qid),
                });
                families_as_spouse.push(family_id);
            }
        }

        let display_name = draft
            .label
            .clone()
            .unwrap_or_else(|| label_name_from_parts(draft).unwrap_or_else(|| draft.qid.clone()));
        let name = Name {
            display: display_name,
            given: first_labeled_qid(draft, &draft.given_name_qids),
            surname: first_labeled_qid(draft, &draft.family_name_qids),
            aliases: Vec::new(),
            provenance: wikidata_provenance(&draft.qid),
        };
        people.push(Person {
            id: person_id,
            names: vec![name],
            sex: sex_from_draft(draft),
            events: person_events,
            families_as_child,
            families_as_spouse,
            notes: Vec::new(),
            source_record: Some(SourceRef(format!("wikidata:{}", draft.qid))),
            provenance: wikidata_provenance(&draft.qid),
        });
    }

    GenealogyIndex::build(people, events, families, places, notes)
}

fn first_place_for_role(
    draft: &WikidataPersonDraft,
    role: PlaceRole,
    places: &mut Vec<Place>,
    place_ids_by_key: &mut BTreeMap<(String, PlaceRole), PlaceId>,
    next_place_id: &mut u64,
) -> Option<PlaceId> {
    let qid = match role {
        PlaceRole::Birth => draft.birth_place_qids.first()?,
        PlaceRole::Death => draft.death_place_qids.first()?,
    };
    let key = (qid.clone(), role);
    if let Some(id) = place_ids_by_key.get(&key).copied() {
        return Some(id);
    }

    let id = PlaceId(*next_place_id);
    *next_place_id += 1;
    places.push(Place {
        id,
        name: label_or_qid(draft, qid),
        lat_lon: None,
        geosuggest_id: None,
        provenance: wikidata_provenance(qid),
    });
    place_ids_by_key.insert(key, id);
    Some(id)
}

fn label_name_from_parts(draft: &WikidataPersonDraft) -> Option<String> {
    let given = first_labeled_qid(draft, &draft.given_name_qids);
    let surname = first_labeled_qid(draft, &draft.family_name_qids);
    match (given, surname) {
        (Some(given), Some(surname)) => Some(format!("{given} {surname}")),
        (Some(given), None) => Some(given),
        (None, Some(surname)) => Some(surname),
        (None, None) => None,
    }
}

fn first_labeled_qid(draft: &WikidataPersonDraft, qids: &[String]) -> Option<String> {
    qids.iter()
        .find_map(|qid| draft.labels.get(qid).cloned())
        .or_else(|| qids.first().cloned())
}

fn label_or_qid(draft: &WikidataPersonDraft, qid: &str) -> String {
    draft
        .labels
        .get(qid)
        .cloned()
        .unwrap_or_else(|| qid.to_string())
}

fn sex_from_draft(draft: &WikidataPersonDraft) -> Option<Sex> {
    if draft.sex_or_gender_qids.iter().any(|qid| qid == "Q6581097") {
        Some(Sex::Male)
    } else if draft.sex_or_gender_qids.iter().any(|qid| qid == "Q6581072") {
        Some(Sex::Female)
    } else if draft.sex_or_gender_qids.is_empty() {
        None
    } else {
        Some(Sex::Other)
    }
}

fn wikidata_provenance(qid: &str) -> Provenance {
    Provenance {
        sources: vec![SourceRef(format!("wikidata:{qid}"))],
        citations: Vec::new(),
        tags: vec![Tag("import:wikidata-truthy".to_string())],
        attributes: vec![Attribute {
            key: "wikidata_qid".to_string(),
            value: qid.to_string(),
        }],
    }
}

pub fn collect_referenced_qids_from_facts(path: &Path) -> io::Result<HashSet<String>> {
    let input = File::open(path)?;
    let reader = BufReader::with_capacity(1024 * 1024, input);
    let mut qids = HashSet::new();

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let fact: WikidataFact = serde_json::from_str(&line).map_err(io::Error::other)?;
        qids.insert(fact.subject_qid);
        if fact.value_kind == WikidataValueKind::EntityQid {
            qids.insert(fact.value);
        }
    }

    Ok(qids)
}

pub fn run_truthy_closure_import_from_reader<R: BufRead>(
    reader: R,
    options: &WikidataClosureOptions,
) -> io::Result<WikidataClosureReport> {
    if options.max_lines == 0 || options.max_facts == 0 {
        eprintln!("wikidata-closure: max-lines and max-facts are bounded to zero; nothing to do");
        return Ok(WikidataClosureReport {
            seed_qids: 0,
            seed_subjects_seen: 0,
            lines_read: 0,
            facts_written: 0,
            humans_detected: 0,
            output_path: options.output_path.clone(),
        });
    }

    let seed_qids = collect_referenced_qids_from_facts(&options.seed_path)?;
    eprintln!(
        "wikidata-closure: collected {} seed QIDs from {}",
        seed_qids.len(),
        options.seed_path.display()
    );

    let mut reader = reader;

    prepare_output_parent(&options.output_path)?;
    let output = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&options.output_path)?;
    let mut writer = BufWriter::with_capacity(1024 * 1024, output);

    eprintln!(
        "wikidata-closure: streaming {} -> {} (max_lines={}, max_facts={})",
        options.dump_path.display(),
        options.output_path.display(),
        options.max_lines,
        options.max_facts
    );

    let started_at = Instant::now();
    let mut line = String::new();
    let mut lines_read = 0_u64;
    let mut facts_written = 0_u64;
    let mut humans_detected = 0_u64;
    let mut seed_subjects_seen = HashSet::new();
    let mut summary = ImportSummary::default();

    loop {
        line.clear();
        let bytes = reader.read_line(&mut line)?;
        if bytes == 0 {
            break;
        }
        lines_read += 1;

        if let Some(fact) = parse_relevant_fact(&line, lines_read)
            && seed_qids.contains(&fact.subject_qid)
        {
            if is_human_instance_fact(&fact) {
                humans_detected += 1;
            }
            seed_subjects_seen.insert(fact.subject_qid.clone());
            summary.record(&fact);
            serde_json::to_writer(&mut writer, &fact).map_err(io::Error::other)?;
            writer.write_all(b"\n")?;
            facts_written += 1;

            if facts_written.is_multiple_of(1_000) {
                writer.flush()?;
            }
        }

        maybe_print_progress(
            lines_read,
            facts_written,
            humans_detected,
            &TruthyImportOptions {
                max_lines: options.max_lines,
                max_facts: options.max_facts,
                progress_every: options.progress_every,
                ..TruthyImportOptions::default()
            },
            started_at,
        );

        if lines_read >= options.max_lines || facts_written >= options.max_facts {
            break;
        }
    }

    writer.flush()?;
    print_progress(lines_read, facts_written, humans_detected, started_at, true);
    summary.print();

    Ok(WikidataClosureReport {
        seed_qids: seed_qids.len(),
        seed_subjects_seen: seed_subjects_seen.len(),
        lines_read,
        facts_written,
        humans_detected,
        output_path: options.output_path.clone(),
    })
}

pub fn draft_from_facts<'a>(
    subject_qid: &str,
    facts: impl IntoIterator<Item = &'a WikidataFact>,
) -> WikidataPersonDraft {
    let mut drafts = BTreeMap::new();
    for fact in facts {
        record_fact_into_drafts(&mut drafts, fact);
    }
    drafts
        .remove(subject_qid)
        .unwrap_or_else(|| WikidataPersonDraft {
            qid: subject_qid.to_string(),
            ..WikidataPersonDraft::default()
        })
}

pub fn apply_labels_to_draft(draft: &mut WikidataPersonDraft, labels: &BTreeMap<String, String>) {
    if let Some(label) = labels.get(&draft.qid) {
        draft.label = Some(label.clone());
    }

    let mut referenced_qids = Vec::new();
    collect_draft_reference_qids(draft, &mut referenced_qids);
    for qid in referenced_qids {
        if let Some(label) = labels.get(&qid) {
            draft.labels.insert(qid, label.clone());
        }
    }
}

fn collect_draft_reference_qids(draft: &WikidataPersonDraft, out: &mut Vec<String>) {
    for values in [
        &draft.instance_of_qids,
        &draft.birth_place_qids,
        &draft.death_place_qids,
        &draft.father_qids,
        &draft.mother_qids,
        &draft.spouse_qids,
        &draft.child_qids,
        &draft.sibling_qids,
        &draft.given_name_qids,
        &draft.family_name_qids,
        &draft.occupation_qids,
        &draft.sex_or_gender_qids,
        &draft.citizenship_qids,
    ] {
        for qid in values {
            if !out.iter().any(|existing| existing == qid) {
                out.push(qid.clone());
            }
        }
    }
}

fn record_fact_into_drafts(
    drafts: &mut BTreeMap<String, WikidataPersonDraft>,
    fact: &WikidataFact,
) {
    let draft = drafts
        .entry(fact.subject_qid.clone())
        .or_insert_with(|| WikidataPersonDraft {
            qid: fact.subject_qid.clone(),
            ..WikidataPersonDraft::default()
        });
    draft.source_fact_count += 1;

    match fact.property_pid.as_str() {
        "P31" => {
            push_unique(&mut draft.instance_of_qids, &fact.value);
            if fact.value_kind == WikidataValueKind::EntityQid && fact.value == HUMAN_QID {
                draft.is_human = true;
            }
        }
        "P569" => push_unique(&mut draft.birth_dates, &fact.value),
        "P570" => push_unique(&mut draft.death_dates, &fact.value),
        "P19" => push_entity_qid(&mut draft.birth_place_qids, fact),
        "P20" => push_entity_qid(&mut draft.death_place_qids, fact),
        "P22" => push_entity_qid(&mut draft.father_qids, fact),
        "P25" => push_entity_qid(&mut draft.mother_qids, fact),
        "P26" => push_entity_qid(&mut draft.spouse_qids, fact),
        "P40" => push_entity_qid(&mut draft.child_qids, fact),
        "P3373" => push_entity_qid(&mut draft.sibling_qids, fact),
        "P735" => push_entity_qid(&mut draft.given_name_qids, fact),
        "P734" => push_entity_qid(&mut draft.family_name_qids, fact),
        "P106" => push_entity_qid(&mut draft.occupation_qids, fact),
        "P21" => push_entity_qid(&mut draft.sex_or_gender_qids, fact),
        "P27" => push_entity_qid(&mut draft.citizenship_qids, fact),
        "P625" => push_unique(&mut draft.coordinate_locations, &fact.value),
        _ => {}
    }
}

fn push_entity_qid(values: &mut Vec<String>, fact: &WikidataFact) {
    if fact.value_kind == WikidataValueKind::EntityQid {
        push_unique(values, &fact.value);
    }
}

fn push_unique(values: &mut Vec<String>, value: &str) {
    if !values.iter().any(|existing| existing == value) {
        values.push(value.to_string());
    }
}

pub fn run_truthy_import_from_reader<R: BufRead>(
    reader: R,
    options: &TruthyImportOptions,
) -> io::Result<TruthyImportReport> {
    if options.max_lines == 0 || options.max_facts == 0 {
        eprintln!("wikidata-truthy: max-lines and max-facts are bounded to zero; nothing to do");
        return Ok(TruthyImportReport {
            lines_read: 0,
            facts_written: 0,
            humans_detected: 0,
            stopped_after_subject: false,
            output_path: options.output_path.clone(),
        });
    }

    let mut reader = reader;

    prepare_output_parent(&options.output_path)?;
    let output = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&options.output_path)?;
    let mut writer = BufWriter::with_capacity(1024 * 1024, output);

    eprintln!(
        "wikidata-truthy: streaming {} -> {} (max_lines={}, max_facts={}, subject={}, stop_after_subject={})",
        options.dump_path.display(),
        options.output_path.display(),
        options.max_lines,
        options.max_facts,
        options.subject.as_deref().unwrap_or("*"),
        options.stop_after_subject
    );

    let started_at = Instant::now();
    let mut line = String::new();
    let mut lines_read = 0_u64;
    let mut facts_written = 0_u64;
    let mut humans_detected = 0_u64;
    let mut subject_seen = false;
    let mut stopped_after_subject = false;
    let mut summary = ImportSummary::default();

    loop {
        line.clear();
        let bytes = reader.read_line(&mut line)?;
        if bytes == 0 {
            break;
        }

        lines_read += 1;

        if let Some(fact) = parse_relevant_fact(&line, lines_read) {
            if options
                .subject
                .as_deref()
                .is_some_and(|qid| qid != fact.subject_qid)
            {
                if should_stop(lines_read, facts_written, options) {
                    break;
                }
                if should_stop_after_subject_block(&fact.subject_qid, subject_seen, options) {
                    stopped_after_subject = true;
                    eprintln!(
                        "wikidata-truthy: stopping after subject {} at line {} (next relevant subject: {})",
                        options.subject.as_deref().unwrap_or("*"),
                        lines_read,
                        fact.subject_qid
                    );
                    break;
                }
                maybe_print_progress(
                    lines_read,
                    facts_written,
                    humans_detected,
                    options,
                    started_at,
                );
                continue;
            }

            if is_human_instance_fact(&fact) {
                humans_detected += 1;
            }
            subject_seen = true;
            summary.record(&fact);

            serde_json::to_writer(&mut writer, &fact).map_err(io::Error::other)?;
            writer.write_all(b"\n")?;
            facts_written += 1;

            if facts_written.is_multiple_of(1_000) {
                writer.flush()?;
            }
        }

        maybe_print_progress(
            lines_read,
            facts_written,
            humans_detected,
            options,
            started_at,
        );

        if should_stop(lines_read, facts_written, options) {
            break;
        }
    }

    writer.flush()?;
    print_progress(lines_read, facts_written, humans_detected, started_at, true);
    summary.print();

    Ok(TruthyImportReport {
        lines_read,
        facts_written,
        humans_detected,
        stopped_after_subject,
        output_path: options.output_path.clone(),
    })
}

pub fn parse_relevant_fact(line: &str, line_number: u64) -> Option<WikidataFact> {
    let parsed = parse_truthy_triple(line)?;
    if !is_relevant_property(&parsed.property_pid) {
        return None;
    }

    let property_name = property_name(&parsed.property_pid)?.to_string();

    Some(WikidataFact {
        subject_qid: parsed.subject_qid,
        property_pid: parsed.property_pid,
        property_name,
        value: parsed.value,
        value_kind: parsed.value_kind,
        line_number,
    })
}

pub fn parse_truthy_triple(line: &str) -> Option<ParsedTriple> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }

    let (subject_iri, rest) = take_iri(line)?;
    let subject_qid = extract_qid(subject_iri)?;

    let rest = rest.trim_start();
    let (predicate_iri, rest) = take_iri(rest)?;
    let property_pid = extract_direct_pid(predicate_iri)?;

    let object = strip_trailing_dot(rest.trim_start())?.trim();
    let (value, value_kind) = parse_object(object, property_pid);

    Some(ParsedTriple {
        subject_qid: subject_qid.to_string(),
        property_pid: property_pid.to_string(),
        value,
        value_kind,
    })
}

pub fn is_human_instance_fact(fact: &WikidataFact) -> bool {
    fact.property_pid == "P31"
        && fact.value_kind == WikidataValueKind::EntityQid
        && fact.value == HUMAN_QID
}

fn prepare_output_parent(path: &Path) -> io::Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn should_stop(lines_read: u64, facts_written: u64, options: &TruthyImportOptions) -> bool {
    lines_read >= options.max_lines || facts_written >= options.max_facts
}

fn should_stop_after_subject_block(
    current_subject_qid: &str,
    subject_seen: bool,
    options: &TruthyImportOptions,
) -> bool {
    options.stop_after_subject
        && subject_seen
        && options
            .subject
            .as_deref()
            .is_some_and(|subject_qid| subject_qid != current_subject_qid)
}

#[derive(Debug, Default)]
struct ImportSummary {
    by_subject: BTreeMap<String, SubjectSummary>,
}

impl ImportSummary {
    fn record(&mut self, fact: &WikidataFact) {
        let subject = self
            .by_subject
            .entry(fact.subject_qid.clone())
            .or_insert_with(|| SubjectSummary {
                is_human: false,
                facts_by_property: BTreeMap::new(),
            });
        subject.is_human |= is_human_instance_fact(fact);
        subject
            .facts_by_property
            .entry((fact.property_pid.clone(), fact.property_name.clone()))
            .or_default()
            .push(fact.value.clone());
    }

    fn print(&self) {
        if self.by_subject.is_empty() {
            eprintln!("wikidata-truthy summary: no relevant facts written");
            return;
        }

        eprintln!("wikidata-truthy summary:");
        for (subject_qid, subject) in &self.by_subject {
            let human_suffix = if subject.is_human {
                " human=P31=Q5"
            } else {
                ""
            };
            eprintln!("  subject {subject_qid}{human_suffix}");
            for ((pid, name), values) in &subject.facts_by_property {
                let rendered_values = values
                    .iter()
                    .take(8)
                    .map(String::as_str)
                    .collect::<Vec<_>>()
                    .join(", ");
                let more = values
                    .len()
                    .checked_sub(8)
                    .filter(|remaining| *remaining > 0)
                    .map(|remaining| format!(" (+{remaining} more)"))
                    .unwrap_or_default();
                eprintln!("    {pid} ({name}): {rendered_values}{more}");
            }
        }
    }
}

#[derive(Debug)]
struct SubjectSummary {
    is_human: bool,
    facts_by_property: BTreeMap<(String, String), Vec<String>>,
}

fn maybe_print_progress(
    lines_read: u64,
    facts_written: u64,
    humans_detected: u64,
    options: &TruthyImportOptions,
    started_at: Instant,
) {
    if options.progress_every > 0 && lines_read.is_multiple_of(options.progress_every) {
        print_progress(
            lines_read,
            facts_written,
            humans_detected,
            started_at,
            false,
        );
    }
}

fn print_progress(
    lines_read: u64,
    facts_written: u64,
    humans_detected: u64,
    started_at: Instant,
    final_report: bool,
) {
    let elapsed = started_at.elapsed();
    let seconds = elapsed.as_secs_f64().max(0.001);
    let facts_per_second = facts_written as f64 / seconds;
    let label = if final_report { "done" } else { "progress" };

    eprintln!(
        "wikidata-truthy {label}: lines_read={lines_read} facts_written={facts_written} humans_detected={humans_detected} elapsed={:.1}s facts_per_second={:.1}",
        elapsed.as_secs_f64(),
        facts_per_second
    );
}

fn take_iri(input: &str) -> Option<(&str, &str)> {
    let input = input.trim_start();
    let inner = input.strip_prefix('<')?;
    let end = inner.find('>')?;
    let (iri, rest_after_iri) = inner.split_at(end);
    Some((iri, rest_after_iri.strip_prefix('>')?))
}

fn strip_trailing_dot(input: &str) -> Option<&str> {
    let input = input.trim_end();
    let without_dot = input.strip_suffix('.')?.trim_end();
    Some(without_dot)
}

fn extract_qid(iri: &str) -> Option<&str> {
    let id = iri
        .strip_prefix(WIKIDATA_ENTITY_PREFIX)
        .or_else(|| iri.strip_prefix(WIKIDATA_ENTITY_HTTPS_PREFIX))?;
    is_qid(id).then_some(id)
}

fn extract_direct_pid(iri: &str) -> Option<&str> {
    let id = iri
        .strip_prefix(WIKIDATA_DIRECT_PROP_PREFIX)
        .or_else(|| iri.strip_prefix(WIKIDATA_DIRECT_PROP_HTTPS_PREFIX))?;
    is_pid(id).then_some(id)
}

fn is_qid(value: &str) -> bool {
    value
        .strip_prefix('Q')
        .is_some_and(|digits| !digits.is_empty() && digits.chars().all(|ch| ch.is_ascii_digit()))
}

fn is_pid(value: &str) -> bool {
    value
        .strip_prefix('P')
        .is_some_and(|digits| !digits.is_empty() && digits.chars().all(|ch| ch.is_ascii_digit()))
}

fn parse_object(object: &str, property_pid: &str) -> (String, WikidataValueKind) {
    if let Some((iri, _rest)) = take_iri(object)
        && let Some(qid) = extract_qid(iri)
    {
        return (qid.to_string(), WikidataValueKind::EntityQid);
    }

    if let Some(literal) = parse_literal_object(object) {
        let kind = if property_pid == "P625"
            || literal
                .datatype
                .as_deref()
                .is_some_and(is_coordinate_datatype)
        {
            WikidataValueKind::Coordinate
        } else if literal
            .datatype
            .as_deref()
            .is_some_and(is_datetime_datatype)
        {
            WikidataValueKind::DateTime
        } else {
            WikidataValueKind::Literal
        };
        return (literal.value, kind);
    }

    (object.to_string(), WikidataValueKind::Unknown)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LiteralObject {
    value: String,
    datatype: Option<String>,
}

fn parse_literal_object(object: &str) -> Option<LiteralObject> {
    let rest = object.strip_prefix('"')?;
    let mut escaped = false;
    let mut value = String::new();
    let mut literal_end_byte = None;

    for (idx, ch) in rest.char_indices() {
        if escaped {
            value.push(match ch {
                't' => '\t',
                'n' => '\n',
                'r' => '\r',
                '"' => '"',
                '\\' => '\\',
                other => other,
            });
            escaped = false;
            continue;
        }

        match ch {
            '\\' => escaped = true,
            '"' => {
                literal_end_byte = Some(idx);
                break;
            }
            other => value.push(other),
        }
    }

    let after_literal = &rest[literal_end_byte? + 1..];
    let after_literal = after_literal.trim_start();
    let datatype = after_literal
        .strip_prefix("^^")
        .and_then(take_iri)
        .map(|(iri, _)| iri.to_string());

    Some(LiteralObject { value, datatype })
}

fn is_datetime_datatype(datatype: &str) -> bool {
    datatype == "http://www.w3.org/2001/XMLSchema#dateTime"
        || datatype == "https://www.w3.org/2001/XMLSchema#dateTime"
}

fn is_coordinate_datatype(datatype: &str) -> bool {
    datatype == "http://www.opengis.net/ont/geosparql#wktLiteral"
        || datatype == "https://www.opengis.net/ont/geosparql#wktLiteral"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_entity_triple() {
        let triple = parse_truthy_triple(
            "<http://www.wikidata.org/entity/Q42> <http://www.wikidata.org/prop/direct/P31> <http://www.wikidata.org/entity/Q5> .",
        )
        .expect("valid triple");

        assert_eq!(triple.subject_qid, "Q42");
        assert_eq!(triple.property_pid, "P31");
        assert_eq!(triple.value, "Q5");
        assert_eq!(triple.value_kind, WikidataValueKind::EntityQid);
    }

    #[test]
    fn parses_date_literal_triple() {
        let triple = parse_truthy_triple(
            "<http://www.wikidata.org/entity/Q42> <http://www.wikidata.org/prop/direct/P569> \"+1952-03-11T00:00:00Z\"^^<http://www.w3.org/2001/XMLSchema#dateTime> .",
        )
        .expect("valid date triple");

        assert_eq!(triple.subject_qid, "Q42");
        assert_eq!(triple.property_pid, "P569");
        assert_eq!(triple.value, "+1952-03-11T00:00:00Z");
        assert_eq!(triple.value_kind, WikidataValueKind::DateTime);
    }

    #[test]
    fn parses_relationship_triple() {
        let triple = parse_truthy_triple(
            "<http://www.wikidata.org/entity/Q7259> <http://www.wikidata.org/prop/direct/P22> <http://www.wikidata.org/entity/Q7322> .",
        )
        .expect("valid relationship triple");

        assert_eq!(triple.subject_qid, "Q7259");
        assert_eq!(triple.property_pid, "P22");
        assert_eq!(triple.value, "Q7322");
        assert_eq!(triple.value_kind, WikidataValueKind::EntityQid);
    }

    #[test]
    fn filters_only_whitelisted_properties() {
        let relevant = parse_relevant_fact(
            "<http://www.wikidata.org/entity/Q42> <http://www.wikidata.org/prop/direct/P569> \"+1952-03-11T00:00:00Z\"^^<http://www.w3.org/2001/XMLSchema#dateTime> .",
            7,
        );
        let irrelevant = parse_relevant_fact(
            "<http://www.wikidata.org/entity/Q42> <http://www.wikidata.org/prop/direct/P646> \"/m/02x0z\" .",
            8,
        );

        assert!(relevant.is_some());
        assert!(irrelevant.is_none());
    }

    #[test]
    fn detects_human_instance_fact() {
        let fact = parse_relevant_fact(
            "<http://www.wikidata.org/entity/Q42> <http://www.wikidata.org/prop/direct/P31> <http://www.wikidata.org/entity/Q5> .",
            1,
        )
        .expect("human fact");

        assert_eq!(fact.property_pid, "P31");
        assert_eq!(fact.property_name, "instance_of");
        assert!(is_human_instance_fact(&fact));
    }

    #[test]
    fn parses_coordinate_literal_triple() {
        let triple = parse_truthy_triple(
            "<http://www.wikidata.org/entity/Q64> <http://www.wikidata.org/prop/direct/P625> \"Point(13.383333333 52.516666666)\"^^<http://www.opengis.net/ont/geosparql#wktLiteral> .",
        )
        .expect("valid coordinate triple");

        assert_eq!(triple.property_pid, "P625");
        assert_eq!(triple.value, "Point(13.383333333 52.516666666)");
        assert_eq!(triple.value_kind, WikidataValueKind::Coordinate);
    }

    #[test]
    fn relevant_fact_includes_property_name() {
        let fact = parse_relevant_fact(
            "<http://www.wikidata.org/entity/Q42> <http://www.wikidata.org/prop/direct/P22> <http://www.wikidata.org/entity/Q14623675> .",
            12,
        )
        .expect("father fact");

        assert_eq!(fact.property_pid, "P22");
        assert_eq!(fact.property_name, "father");
        assert_eq!(fact.value, "Q14623675");
    }

    #[test]
    fn stop_after_subject_triggers_after_seen_subject_moves_on() {
        let options = TruthyImportOptions {
            subject: Some("Q42".to_string()),
            stop_after_subject: true,
            ..TruthyImportOptions::default()
        };

        assert!(!should_stop_after_subject_block("Q41", false, &options));
        assert!(!should_stop_after_subject_block("Q42", true, &options));
        assert!(should_stop_after_subject_block("Q43", true, &options));
    }

    #[test]
    fn builds_person_draft_from_subject_facts() {
        let facts = vec![
            parse_relevant_fact(
                "<http://www.wikidata.org/entity/Q42> <http://www.wikidata.org/prop/direct/P31> <http://www.wikidata.org/entity/Q5> .",
                1,
            )
            .expect("instance fact"),
            parse_relevant_fact(
                "<http://www.wikidata.org/entity/Q42> <http://www.wikidata.org/prop/direct/P569> \"+1952-03-11T00:00:00Z\"^^<http://www.w3.org/2001/XMLSchema#dateTime> .",
                2,
            )
            .expect("birth date"),
            parse_relevant_fact(
                "<http://www.wikidata.org/entity/Q42> <http://www.wikidata.org/prop/direct/P22> <http://www.wikidata.org/entity/Q14623675> .",
                3,
            )
            .expect("father"),
            parse_relevant_fact(
                "<http://www.wikidata.org/entity/Q42> <http://www.wikidata.org/prop/direct/P40> <http://www.wikidata.org/entity/Q14623683> .",
                4,
            )
            .expect("child"),
        ];

        let draft = draft_from_facts("Q42", &facts);

        assert!(draft.is_human);
        assert_eq!(draft.qid, "Q42");
        assert_eq!(draft.birth_dates, vec!["+1952-03-11T00:00:00Z"]);
        assert_eq!(draft.father_qids, vec!["Q14623675"]);
        assert_eq!(draft.child_qids, vec!["Q14623683"]);
        assert_eq!(draft.source_fact_count, 4);
    }

    #[test]
    fn records_referenced_qids_for_closure() {
        let facts = vec![
            parse_relevant_fact(
                "<http://www.wikidata.org/entity/Q42> <http://www.wikidata.org/prop/direct/P31> <http://www.wikidata.org/entity/Q5> .",
                1,
            )
            .expect("instance fact"),
            parse_relevant_fact(
                "<http://www.wikidata.org/entity/Q42> <http://www.wikidata.org/prop/direct/P19> <http://www.wikidata.org/entity/Q350> .",
                2,
            )
            .expect("birth place"),
        ];
        let mut qids = HashSet::new();
        for fact in facts {
            qids.insert(fact.subject_qid.clone());
            if fact.value_kind == WikidataValueKind::EntityQid {
                qids.insert(fact.value.clone());
            }
        }

        assert!(qids.contains("Q42"));
        assert!(qids.contains("Q5"));
        assert!(qids.contains("Q350"));
    }

    #[test]
    fn applies_label_cache_to_person_draft() {
        let facts = vec![
            parse_relevant_fact(
                "<http://www.wikidata.org/entity/Q42> <http://www.wikidata.org/prop/direct/P31> <http://www.wikidata.org/entity/Q5> .",
                1,
            )
            .expect("instance fact"),
            parse_relevant_fact(
                "<http://www.wikidata.org/entity/Q42> <http://www.wikidata.org/prop/direct/P19> <http://www.wikidata.org/entity/Q350> .",
                2,
            )
            .expect("birth place"),
        ];
        let mut draft = draft_from_facts("Q42", &facts);
        let labels = BTreeMap::from([
            ("Q42".to_string(), "Douglas Adams".to_string()),
            ("Q350".to_string(), "Cambridge".to_string()),
            ("Q5".to_string(), "human".to_string()),
        ]);

        apply_labels_to_draft(&mut draft, &labels);

        assert_eq!(draft.label.as_deref(), Some("Douglas Adams"));
        assert_eq!(
            draft.labels.get("Q350").map(String::as_str),
            Some("Cambridge")
        );
        assert_eq!(draft.labels.get("Q5").map(String::as_str), Some("human"));
    }
}
