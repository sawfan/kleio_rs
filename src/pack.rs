//! Event packs and timeline documents.
//!
//! Packs are versioned bundles of timeline data that can be attached to a
//! project: family archives, journals, research logs, local histories, or public
//! historical timelines. They are intentionally generic and can carry domain
//! profiles alongside entities/events so project-local vocabularies remain
//! portable.

use rkyv::{Archive, Deserialize, Serialize};

use crate::attribution::{Provenance, SourceRef, Tag};
use crate::entity::{Entity, EntityRef};
use crate::event::TimelineEvent;
use crate::event_query::{TimelineEventFilter, YearSpan, filter_timeline_events};
use crate::event_type::{DomainProfile, EventTypeId};

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Archive,
    Serialize,
    Deserialize,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct PackId(pub String);

impl PackId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
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
pub enum PackKind {
    UserJournal,
    Genealogy,
    HistoricalTimeline,
    ResearchLog,
    ImportedDataset,
    ReferenceDataset,
    Custom(String),
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
pub struct PackMetadata {
    pub id: PackId,
    pub title: String,
    pub version: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub license: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

impl PackMetadata {
    pub fn new(id: PackId, title: impl Into<String>) -> Self {
        Self {
            id,
            title: title.into(),
            version: "0.1.0".to_string(),
            description: None,
            author: None,
            license: None,
            created_at: None,
            updated_at: None,
        }
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
pub struct SourceRecord {
    pub id: SourceRef,
    pub title: String,
    pub description: Option<String>,
    pub url: Option<String>,
    pub provenance: Provenance,
}

impl SourceRecord {
    pub fn new(id: SourceRef, title: impl Into<String>) -> Self {
        Self {
            id,
            title: title.into(),
            description: None,
            url: None,
            provenance: Provenance::default(),
        }
    }
}

#[derive(
    Debug, Clone, PartialEq, Archive, Serialize, Deserialize, serde::Serialize, serde::Deserialize,
)]
pub struct EventPack {
    pub metadata: PackMetadata,
    pub kind: PackKind,
    pub domain_profiles: Vec<DomainProfile>,
    pub entities: Vec<Entity>,
    pub events: Vec<TimelineEvent>,
    pub sources: Vec<SourceRecord>,
    pub tags: Vec<Tag>,
    pub provenance: Provenance,
}

impl EventPack {
    pub fn empty(metadata: PackMetadata, kind: PackKind) -> Self {
        Self {
            metadata,
            kind,
            domain_profiles: Vec::new(),
            entities: Vec::new(),
            events: Vec::new(),
            sources: Vec::new(),
            tags: Vec::new(),
            provenance: Provenance::default(),
        }
    }

    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    pub fn events_for_entity(&self, entity: impl Into<EntityRef>) -> Vec<&TimelineEvent> {
        let filter = TimelineEventFilter::for_entity(entity);
        filter_timeline_events(&self.events, &filter)
    }

    pub fn events_by_type(&self, event_type: EventTypeId) -> Vec<&TimelineEvent> {
        let filter = TimelineEventFilter::new().with_event_type(event_type);
        filter_timeline_events(&self.events, &filter)
    }

    pub fn events_in_year_span(&self, years: YearSpan) -> Vec<&TimelineEvent> {
        let filter = TimelineEventFilter::new().with_year_span(years);
        filter_timeline_events(&self.events, &filter)
    }
}

#[derive(
    Debug, Clone, PartialEq, Archive, Serialize, Deserialize, serde::Serialize, serde::Deserialize,
)]
pub struct TimelineDocument {
    pub version: u32,
    pub packs: Vec<EventPack>,
    pub active_pack_ids: Vec<PackId>,
}

impl TimelineDocument {
    pub const CURRENT_VERSION: u32 = 1;

    pub fn empty() -> Self {
        Self {
            version: Self::CURRENT_VERSION,
            packs: Vec::new(),
            active_pack_ids: Vec::new(),
        }
    }

    pub fn active_packs(&self) -> impl Iterator<Item = &EventPack> {
        self.packs.iter().filter(|pack| {
            self.active_pack_ids
                .iter()
                .any(|id| id == &pack.metadata.id)
        })
    }

    pub fn active_events(&self) -> Vec<&TimelineEvent> {
        self.active_packs()
            .flat_map(|pack| pack.events.iter())
            .collect()
    }

    pub fn add_pack(&mut self, pack: EventPack, active: bool) {
        if active
            && !self
                .active_pack_ids
                .iter()
                .any(|id| id == &pack.metadata.id)
        {
            self.active_pack_ids.push(pack.metadata.id.clone());
        }
        self.packs.push(pack);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        DateValue, EventId, EventParticipant, EventTypeId, PersonId, Provenance, TimeSpec,
    };

    #[test]
    fn pack_filters_events_by_entity_and_year() {
        let mut pack = EventPack::empty(
            PackMetadata::new(PackId::new("pack:family"), "Family Pack"),
            PackKind::Genealogy,
        );
        pack.events.push(
            TimelineEvent::new(EventId(1), EventTypeId::new("genealogy.birth"), "Birth")
                .with_time(TimeSpec::from_date_value(DateValue::from_original(
                    "1901",
                    Provenance::default(),
                )))
                .with_participant(EventParticipant::new(PersonId(7), "child")),
        );
        pack.events.push(
            TimelineEvent::new(EventId(2), EventTypeId::new("genealogy.death"), "Death")
                .with_time(TimeSpec::from_date_value(DateValue::from_original(
                    "1980",
                    Provenance::default(),
                )))
                .with_participant(EventParticipant::new(PersonId(7), "deceased")),
        );

        assert_eq!(pack.events_for_entity(PersonId(7)).len(), 2);
        assert_eq!(pack.events_in_year_span(YearSpan::exact(1901)).len(), 1);
    }

    #[test]
    fn timeline_document_returns_only_active_pack_events() {
        let active_pack = EventPack {
            events: vec![TimelineEvent::new(
                EventId(1),
                EventTypeId::new("journal.entry"),
                "Active entry",
            )],
            ..EventPack::empty(
                PackMetadata::new(PackId::new("pack:active"), "Active"),
                PackKind::UserJournal,
            )
        };
        let inactive_pack = EventPack {
            events: vec![TimelineEvent::new(
                EventId(2),
                EventTypeId::new("journal.entry"),
                "Inactive entry",
            )],
            ..EventPack::empty(
                PackMetadata::new(PackId::new("pack:inactive"), "Inactive"),
                PackKind::UserJournal,
            )
        };
        let mut document = TimelineDocument::empty();

        document.add_pack(active_pack, true);
        document.add_pack(inactive_pack, false);

        let active_events = document.active_events();
        assert_eq!(active_events.len(), 1);
        assert_eq!(active_events[0].id, EventId(1));
    }
}
