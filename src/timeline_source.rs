//! Human-editable timeline source format.
//!
//! `EventPack` is the normalized exchange/archive shape. It intentionally carries
//! domain profiles, provenance, generated boundary events, and graph relations.
//! This module provides a smaller authoring shape suitable for local TOML files
//! that can be compiled into an `EventPack`.

use std::collections::{BTreeMap, BTreeSet};

use rkyv::{Archive, Deserialize, Serialize};

use crate::attribution::Provenance;
use crate::event::{
    EventBoundaryKind, EventCompositionKind, EventParticipant, EventRelation, EventRelationKind,
    EventTemporalKind, TimeSpec, TimelineEvent,
};
use crate::event_type::{EventTypeId, genealogy_domain_profile};
use crate::model::{DateValue, EventId, PersonId};
use crate::pack::{EventPack, PackId, PackKind, PackMetadata};
use crate::pack_builder::{EventPackBuilder, ManualEventDraft};

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Archive,
    Serialize,
    Deserialize,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct TimelineSource {
    pub meta: TimelineSourceMeta,
    pub items: Vec<TimelineSourceItem>,
}

impl TimelineSource {
    pub fn new(title: impl Into<String>) -> Self {
        let title = title.into();
        Self {
            meta: TimelineSourceMeta {
                id: Some("local:personal-timeline".to_string()),
                title,
                kind: Some(TimelineSourcePackKind::Biography),
                description: None,
                subject: None,
                person_id: Some(1000),
            },
            items: Vec::new(),
        }
    }

    pub fn into_event_pack(self) -> EventPack {
        event_pack_from_timeline_source(&self)
    }
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Archive,
    Serialize,
    Deserialize,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct TimelineSourceMeta {
    pub id: Option<String>,
    pub title: String,
    pub kind: Option<TimelineSourcePackKind>,
    pub description: Option<String>,
    pub subject: Option<String>,
    pub person_id: Option<u64>,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Archive,
    Serialize,
    Deserialize,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct TimelineSourceItemsFile {
    pub items: Vec<TimelineSourceItem>,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Archive,
    Serialize,
    Deserialize,
    serde::Serialize,
    serde::Deserialize,
)]
pub enum TimelineSourcePackKind {
    Biography,
    UserJournal,
    HistoricalTimeline,
    ResearchLog,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Archive,
    Serialize,
    Deserialize,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct TimelineSourceItem {
    pub section: String,
    pub title: String,
    pub start: String,
    pub end: Option<String>,
    #[serde(default)]
    pub current: bool,
    pub place: Option<String>,
    pub details: Option<String>,
    pub start_label: Option<String>,
    pub end_label: Option<String>,
}

impl TimelineSourceItem {
    pub fn event(
        section: impl Into<String>,
        title: impl Into<String>,
        start: impl Into<String>,
    ) -> Self {
        Self {
            section: section.into(),
            title: title.into(),
            start: start.into(),
            end: None,
            current: false,
            place: None,
            details: None,
            start_label: None,
            end_label: None,
        }
    }

    pub fn period(
        section: impl Into<String>,
        title: impl Into<String>,
        start: impl Into<String>,
        end: impl Into<String>,
    ) -> Self {
        Self {
            section: section.into(),
            title: title.into(),
            start: start.into(),
            end: Some(end.into()),
            current: false,
            place: None,
            details: None,
            start_label: None,
            end_label: None,
        }
    }

    pub fn current_period(
        section: impl Into<String>,
        title: impl Into<String>,
        start: impl Into<String>,
    ) -> Self {
        Self {
            section: section.into(),
            title: title.into(),
            start: start.into(),
            end: None,
            current: true,
            place: None,
            details: None,
            start_label: None,
            end_label: None,
        }
    }
}

pub fn timeline_source_to_toml_pretty(source: &TimelineSource) -> Result<String, toml::ser::Error> {
    toml::to_string_pretty(source)
}

pub fn timeline_source_from_toml(toml_text: &str) -> Result<TimelineSource, toml::de::Error> {
    toml::from_str(toml_text)
}

pub fn timeline_source_from_parts(
    meta: TimelineSourceMeta,
    items: impl IntoIterator<Item = TimelineSourceItem>,
) -> TimelineSource {
    TimelineSource {
        meta,
        items: items.into_iter().collect(),
    }
}

pub fn timeline_source_to_parts(
    source: TimelineSource,
) -> (TimelineSourceMeta, Vec<TimelineSourceItem>) {
    (source.meta, source.items)
}

pub fn timeline_source_meta_to_toml_pretty(
    meta: &TimelineSourceMeta,
) -> Result<String, toml::ser::Error> {
    toml::to_string_pretty(meta)
}

pub fn timeline_source_meta_from_toml(
    toml_text: &str,
) -> Result<TimelineSourceMeta, toml::de::Error> {
    toml::from_str(toml_text)
}

pub fn timeline_source_items_to_toml_pretty(
    items: &[TimelineSourceItem],
) -> Result<String, toml::ser::Error> {
    toml::to_string_pretty(&TimelineSourceItemsFile {
        items: items.to_vec(),
    })
}

pub fn timeline_source_items_from_toml(
    toml_text: &str,
) -> Result<Vec<TimelineSourceItem>, toml::de::Error> {
    let file: TimelineSourceItemsFile = toml::from_str(toml_text)?;
    Ok(file.items)
}

pub fn timeline_source_to_json_pretty(
    source: &TimelineSource,
) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(source)
}

pub fn timeline_source_from_json(json: &str) -> Result<TimelineSource, serde_json::Error> {
    serde_json::from_str(json)
}

pub fn event_pack_from_timeline_source(source: &TimelineSource) -> EventPack {
    let person_id = PersonId(source.meta.person_id.unwrap_or(1000));
    let pack_id = source
        .meta
        .id
        .clone()
        .unwrap_or_else(|| stable_pack_id_from_title(&source.meta.title));
    let mut metadata = PackMetadata::new(PackId::new(pack_id), source.meta.title.clone());
    metadata.description = source.meta.description.clone();

    let mut builder = EventPackBuilder::new(metadata, source_pack_kind(source.meta.kind));
    builder.add_domain_profile(genealogy_domain_profile());

    let mut compiled_items = Vec::new();
    for item in &source.items {
        compiled_items.push(add_source_item_to_pack(&mut builder, item, person_id));
    }
    add_life_containment_relations(&mut builder, &compiled_items);

    builder.into_pack()
}

pub fn timeline_source_from_event_pack(pack: &EventPack) -> TimelineSource {
    let person_id = pack
        .events
        .iter()
        .flat_map(|event| event.participants.iter())
        .find_map(|participant| match participant.entity {
            crate::entity::EntityRef::Person(person_id) => Some(person_id.0),
            _ => None,
        })
        .unwrap_or(1000);

    let boundary_labels = boundary_labels_by_parent(pack);
    let boundary_child_ids = boundary_child_event_ids(pack);
    let items = pack
        .events
        .iter()
        .filter(|event| !boundary_child_ids.contains(&event.id))
        .filter_map(|event| source_item_from_event(event, boundary_labels.get(&event.id)))
        .collect();

    TimelineSource {
        meta: TimelineSourceMeta {
            id: Some(pack.metadata.id.as_str().to_string()),
            title: pack.metadata.title.clone(),
            kind: source_kind_from_pack_kind(&pack.kind),
            description: pack.metadata.description.clone(),
            subject: None,
            person_id: Some(person_id),
        },
        items,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CompiledSourceItem {
    event_id: EventId,
    section: String,
    is_period: bool,
}

fn add_source_item_to_pack(
    builder: &mut EventPackBuilder,
    item: &TimelineSourceItem,
    person_id: PersonId,
) -> CompiledSourceItem {
    let section = item.section.trim();
    let end = item
        .end
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty());
    let is_period = end.is_some() || item.current;
    let event_id = builder.add_manual_event(source_item_event_draft(item, person_id, is_period));

    if is_period {
        let start_boundary_id = builder.add_manual_event(boundary_draft_for_source_item(
            source_boundary_type(section, true),
            item.start_label
                .clone()
                .filter(|label| !label.trim().is_empty())
                .unwrap_or_else(|| format!("Started {}", item.title)),
            &item.start,
            person_id,
            EventBoundaryKind::Start,
        ));
        builder.add_event_relation(EventRelation::new(
            event_id,
            start_boundary_id,
            EventRelationKind::Starts,
        ));

        if let Some(end) = end {
            let end_boundary_id = builder.add_manual_event(boundary_draft_for_source_item(
                source_boundary_type(section, false),
                item.end_label
                    .clone()
                    .filter(|label| !label.trim().is_empty())
                    .unwrap_or_else(|| format!("Ended {}", item.title)),
                end,
                person_id,
                EventBoundaryKind::End,
            ));
            builder.add_event_relation(EventRelation::new(
                event_id,
                end_boundary_id,
                EventRelationKind::Ends,
            ));
        }
    }

    CompiledSourceItem {
        event_id,
        section: section.to_string(),
        is_period,
    }
}

fn add_life_containment_relations(
    builder: &mut EventPackBuilder,
    compiled_items: &[CompiledSourceItem],
) {
    let Some(life_event_id) = compiled_items
        .iter()
        .find(|item| item.section == "life")
        .map(|item| item.event_id)
    else {
        return;
    };

    for item in compiled_items {
        if item.event_id == life_event_id || !item.is_period || item.section == "life" {
            continue;
        }

        builder.add_event_relation(EventRelation::new(
            life_event_id,
            item.event_id,
            EventRelationKind::OccursWithin,
        ));
    }
}

fn source_item_event_draft(
    item: &TimelineSourceItem,
    person_id: PersonId,
    is_period: bool,
) -> ManualEventDraft {
    let section = item.section.trim();
    let event_type = source_event_type(section, is_period);
    let mut draft = ManualEventDraft::new(EventTypeId::new(event_type), item.title.clone())
        .with_participant(EventParticipant::new(person_id, source_role(section)));

    let end = item
        .end
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty());
    draft = if is_period {
        draft
            .with_time(TimeSpec::Range {
                start: Some(DateValue::from_original(
                    item.start.clone(),
                    Provenance::default(),
                )),
                end: end.map(|value| DateValue::from_original(value, Provenance::default())),
            })
            .with_composition_kind(EventCompositionKind::Composite)
            .with_temporal_kind(EventTemporalKind::Interval)
    } else {
        draft
            .with_time(TimeSpec::from_date_value(DateValue::from_original(
                item.start.clone(),
                Provenance::default(),
            )))
            .with_composition_kind(EventCompositionKind::Atomic)
            .with_temporal_kind(EventTemporalKind::Instant)
    };

    let description = source_description(item.place.as_deref(), item.details.as_deref());
    if !description.is_empty() {
        draft = draft.with_description(description);
    }

    draft
}

fn boundary_draft_for_source_item(
    event_type: &'static str,
    title: String,
    date: &str,
    person_id: PersonId,
    boundary: EventBoundaryKind,
) -> ManualEventDraft {
    ManualEventDraft::new(EventTypeId::new(event_type), title)
        .with_time(TimeSpec::from_date_value(DateValue::from_original(
            date.to_string(),
            Provenance::default(),
        )))
        .with_composition_kind(EventCompositionKind::Atomic)
        .with_temporal_kind(EventTemporalKind::Instant)
        .with_boundary_kind(boundary)
        .with_participant(EventParticipant::new(
            person_id,
            source_role_for_event_type(event_type),
        ))
}

fn source_event_type(section: &str, is_period: bool) -> &'static str {
    match (section, is_period) {
        ("life", true) => "genealogy.life",
        ("birth", false) => "genealogy.birth",
        ("death", false) => "genealogy.death",
        ("residence", true) => "residence.period",
        ("residence", false) => "residence.event",
        ("education", true) => "education.period",
        ("education", false) => "education.event",
        ("career", true) => "career.job",
        ("career", false) => "career.event",
        ("relationship", true) => "genealogy.marriage",
        ("relationship", false) => "relationship.event",
        ("family", _) => "genealogy.event",
        ("travel", true) => "travel.period",
        ("travel", false) => "travel.event",
        _ => "personal.event",
    }
}

fn source_boundary_type(section: &str, starts: bool) -> &'static str {
    match (section, starts) {
        ("life", true) => "genealogy.birth",
        ("life", false) => "genealogy.death",
        ("residence", true) => "residence.moved_in",
        ("residence", false) => "residence.moved_out",
        ("education", true) => "education.started",
        ("education", false) => "education.finished",
        ("career", true) => "career.started_job",
        ("career", false) => "career.ended_job",
        ("relationship", true) => "relationship.started",
        ("relationship", false) => "relationship.ended",
        ("travel", true) => "travel.arrived",
        ("travel", false) => "travel.departed",
        (_, true) => "personal.started",
        (_, false) => "personal.ended",
    }
}

fn source_role(section: &str) -> &'static str {
    match section {
        "birth" => "child",
        "death" => "deceased",
        "residence" => "resident",
        _ => "subject",
    }
}

fn source_role_for_event_type(event_type: &str) -> &'static str {
    match event_type {
        "genealogy.birth" => "child",
        "genealogy.death" => "deceased",
        "genealogy.residence" | "residence.period" | "residence.event" => "resident",
        _ => "subject",
    }
}

fn source_pack_kind(kind: Option<TimelineSourcePackKind>) -> PackKind {
    match kind.unwrap_or(TimelineSourcePackKind::Biography) {
        TimelineSourcePackKind::Biography => PackKind::Biography,
        TimelineSourcePackKind::UserJournal => PackKind::UserJournal,
        TimelineSourcePackKind::HistoricalTimeline => PackKind::HistoricalTimeline,
        TimelineSourcePackKind::ResearchLog => PackKind::ResearchLog,
    }
}

fn source_kind_from_pack_kind(kind: &PackKind) -> Option<TimelineSourcePackKind> {
    match kind {
        PackKind::Biography | PackKind::Genealogy => Some(TimelineSourcePackKind::Biography),
        PackKind::UserJournal => Some(TimelineSourcePackKind::UserJournal),
        PackKind::HistoricalTimeline => Some(TimelineSourcePackKind::HistoricalTimeline),
        PackKind::ResearchLog => Some(TimelineSourcePackKind::ResearchLog),
        PackKind::ImportedDataset | PackKind::ReferenceDataset | PackKind::Custom(_) => None,
    }
}

fn source_item_from_event(
    event: &TimelineEvent,
    boundary_labels: Option<&BoundaryLabels>,
) -> Option<TimelineSourceItem> {
    let (start, end, current) = source_time_fields(&event.time, event.is_interval())?;
    let (place, details) = split_source_description(event.description.as_deref());
    Some(TimelineSourceItem {
        section: source_section_label(event.type_ref.as_str()).to_string(),
        title: event.title.clone(),
        start,
        end,
        current,
        place,
        details,
        start_label: boundary_labels.and_then(|labels| labels.start.clone()),
        end_label: boundary_labels.and_then(|labels| labels.end.clone()),
    })
}

fn source_time_fields(
    time: &TimeSpec,
    is_interval: bool,
) -> Option<(String, Option<String>, bool)> {
    match time {
        TimeSpec::Unknown => None,
        TimeSpec::Date(date) | TimeSpec::Approximate { value: date, .. } => {
            Some((date.display(), None, false))
        }
        TimeSpec::Range { start, end } => {
            let start = start.as_ref()?.display();
            Some((
                start,
                end.as_ref().map(DateValue::display),
                is_interval && end.is_none(),
            ))
        }
        TimeSpec::Before(date) | TimeSpec::After(date) => Some((date.display(), None, false)),
        TimeSpec::Between { start, end } => Some((start.display(), Some(end.display()), false)),
        TimeSpec::OriginalOnly { original } => Some((original.clone(), None, false)),
    }
}

fn source_section_label(type_ref: &str) -> &'static str {
    if type_ref == "genealogy.life" {
        "life"
    } else if type_ref == "genealogy.birth" {
        "birth"
    } else if type_ref == "genealogy.death" {
        "death"
    } else if type_ref.starts_with("residence.") {
        "residence"
    } else if type_ref.starts_with("education.") {
        "education"
    } else if type_ref.starts_with("career.") {
        "career"
    } else if type_ref == "genealogy.marriage" || type_ref.starts_with("relationship.") {
        "relationship"
    } else if type_ref.starts_with("travel.") {
        "travel"
    } else if type_ref.starts_with("journal.") {
        "journal"
    } else {
        "event"
    }
}

fn source_description(place: Option<&str>, details: Option<&str>) -> String {
    let place = place.unwrap_or_default().trim();
    let details = details.unwrap_or_default().trim();
    match (place.is_empty(), details.is_empty()) {
        (true, true) => String::new(),
        (false, true) => format!("Place: {place}"),
        (true, false) => details.to_string(),
        (false, false) => format!("Place: {place}\n\n{details}"),
    }
}

fn split_source_description(description: Option<&str>) -> (Option<String>, Option<String>) {
    let Some(description) = description else {
        return (None, None);
    };
    if let Some(rest) = description.strip_prefix("Place: ") {
        let (place, details) = rest
            .split_once("\n\n")
            .map(|(place, details)| (place.to_string(), details.to_string()))
            .unwrap_or_else(|| (rest.to_string(), String::new()));
        return (
            (!place.trim().is_empty()).then_some(place),
            (!details.trim().is_empty()).then_some(details),
        );
    }
    (
        None,
        (!description.trim().is_empty()).then_some(description.to_string()),
    )
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct BoundaryLabels {
    start: Option<String>,
    end: Option<String>,
}

fn boundary_labels_by_parent(pack: &EventPack) -> BTreeMap<EventId, BoundaryLabels> {
    let event_titles: BTreeMap<EventId, String> = pack
        .events
        .iter()
        .map(|event| (event.id, event.title.clone()))
        .collect();
    let mut labels = BTreeMap::new();

    for relation in &pack.event_relations {
        let Some(title) = event_titles.get(&relation.child_event_id).cloned() else {
            continue;
        };
        let entry: &mut BoundaryLabels = labels.entry(relation.parent_event_id).or_default();
        match relation.kind {
            EventRelationKind::Starts => entry.start = Some(title),
            EventRelationKind::Ends => entry.end = Some(title),
            EventRelationKind::Contains
            | EventRelationKind::OccursWithin
            | EventRelationKind::EvidenceFor
            | EventRelationKind::Summarizes
            | EventRelationKind::ContextFor
            | EventRelationKind::SubEvent
            | EventRelationKind::Custom(_) => {}
        }
    }

    labels
}

fn boundary_child_event_ids(pack: &EventPack) -> BTreeSet<EventId> {
    pack.event_relations
        .iter()
        .filter(|relation| {
            matches!(
                relation.kind,
                EventRelationKind::Starts | EventRelationKind::Ends
            )
        })
        .map(|relation| relation.child_event_id)
        .collect()
}

fn stable_pack_id_from_title(title: &str) -> String {
    let slug = title
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    format!("local:{}", if slug.is_empty() { "timeline" } else { &slug })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeline_source_toml_compiles_to_event_pack() {
        let source = TimelineSource {
            meta: TimelineSourceMeta {
                id: Some("local:mine".to_string()),
                title: "My Life".to_string(),
                kind: Some(TimelineSourcePackKind::Biography),
                description: Some("Private timeline".to_string()),
                subject: Some("Me".to_string()),
                person_id: Some(7),
            },
            items: vec![
                TimelineSourceItem::current_period("life", "Life", "1990"),
                TimelineSourceItem::event("birth", "I was born", "1990"),
                TimelineSourceItem::period("education", "High school", "2004", "2008"),
                TimelineSourceItem::current_period("career", "Current job", "2013"),
            ],
        };

        let toml = timeline_source_to_toml_pretty(&source).expect("serialize source");
        let parsed = timeline_source_from_toml(&toml).expect("parse source");
        let pack = event_pack_from_timeline_source(&parsed);

        assert_eq!(pack.metadata.title, "My Life");
        assert!(pack.events.iter().any(|event| event.title == "High school"));
        assert!(pack.event_relations.iter().any(|relation| {
            relation.kind == EventRelationKind::OccursWithin
                && pack
                    .events
                    .iter()
                    .any(|event| event.id == relation.parent_event_id && event.title == "Life")
                && pack.events.iter().any(|event| {
                    event.id == relation.child_event_id && event.title == "High school"
                })
        }));
        assert!(
            pack.events
                .iter()
                .any(|event| event.time.display() == "2013 to present")
        );
    }

    #[test]
    fn event_pack_can_export_to_timeline_source() {
        let source = TimelineSource {
            meta: TimelineSourceMeta {
                id: Some("local:mine".to_string()),
                title: "My Life".to_string(),
                kind: Some(TimelineSourcePackKind::Biography),
                description: None,
                subject: None,
                person_id: Some(7),
            },
            items: vec![TimelineSourceItem::period(
                "education",
                "High school",
                "2004",
                "2008",
            )],
        };
        let pack = event_pack_from_timeline_source(&source);
        let exported = timeline_source_from_event_pack(&pack);

        assert_eq!(exported.meta.title, "My Life");
        assert_eq!(exported.items.len(), 1);
        assert_eq!(exported.items[0].title, "High school");
    }
}
