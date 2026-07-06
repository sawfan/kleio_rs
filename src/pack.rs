//! Event packs and timeline documents.
//!
//! Packs are versioned bundles of timeline data that can be attached to a
//! project: family archives, journals, research logs, local histories, or public
//! historical timelines. They are intentionally generic and can carry domain
//! profiles alongside entities/events so project-local vocabularies remain
//! portable.

use std::collections::BTreeSet;

use rkyv::{Archive, Deserialize, Serialize};

use crate::attribution::{Provenance, SourceRef, Tag};
use crate::entity::{Entity, EntityRef};
use crate::event::{EventRelation, TimelineEvent};
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
    Biography,
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
    pub event_relations: Vec<EventRelation>,
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
            event_relations: Vec::new(),
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

    pub fn child_event_relations(&self, parent_event_id: crate::EventId) -> Vec<&EventRelation> {
        self.event_relations
            .iter()
            .filter(|relation| relation.parent_event_id == parent_event_id)
            .collect()
    }

    pub fn parent_event_relations(&self, child_event_id: crate::EventId) -> Vec<&EventRelation> {
        self.event_relations
            .iter()
            .filter(|relation| relation.child_event_id == child_event_id)
            .collect()
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

    pub fn pack(&self, pack_id: &PackId) -> Option<&EventPack> {
        self.packs.iter().find(|pack| &pack.metadata.id == pack_id)
    }

    pub fn pack_mut(&mut self, pack_id: &PackId) -> Option<&mut EventPack> {
        self.packs
            .iter_mut()
            .find(|pack| &pack.metadata.id == pack_id)
    }

    pub fn is_pack_active(&self, pack_id: &PackId) -> bool {
        self.active_pack_ids.iter().any(|id| id == pack_id)
    }

    pub fn set_pack_active(&mut self, pack_id: &PackId, active: bool) {
        if active {
            if self.pack(pack_id).is_some() && !self.is_pack_active(pack_id) {
                self.active_pack_ids.push(pack_id.clone());
            }
        } else {
            self.active_pack_ids.retain(|id| id != pack_id);
        }
    }

    pub fn activate_pack(&mut self, pack_id: &PackId) {
        self.set_pack_active(pack_id, true);
    }

    pub fn deactivate_pack(&mut self, pack_id: &PackId) {
        self.set_pack_active(pack_id, false);
    }

    pub fn remove_pack(&mut self, pack_id: &PackId) -> Option<EventPack> {
        let index = self
            .packs
            .iter()
            .position(|pack| &pack.metadata.id == pack_id)?;
        self.active_pack_ids.retain(|id| id != pack_id);
        Some(self.packs.remove(index))
    }

    pub fn active_events(&self) -> Vec<&TimelineEvent> {
        self.active_packs()
            .flat_map(|pack| pack.events.iter())
            .collect()
    }

    pub fn active_event_relations(&self) -> Vec<&EventRelation> {
        self.active_packs()
            .flat_map(|pack| pack.event_relations.iter())
            .collect()
    }

    pub fn active_events_filtered(&self, filter: &TimelineEventFilter) -> Vec<&TimelineEvent> {
        self.active_packs()
            .flat_map(|pack| filter_timeline_events(&pack.events, filter))
            .collect()
    }

    pub fn active_events_for_entity(&self, entity: impl Into<EntityRef>) -> Vec<&TimelineEvent> {
        let filter = TimelineEventFilter::for_entity(entity);
        self.active_events_filtered(&filter)
    }

    pub fn active_events_in_year_span(&self, years: YearSpan) -> Vec<&TimelineEvent> {
        let filter = TimelineEventFilter::new().with_year_span(years);
        self.active_events_filtered(&filter)
    }

    pub fn visible_active_events(
        &self,
        collapsed_parent_ids: impl IntoIterator<Item = crate::EventId>,
    ) -> Vec<&TimelineEvent> {
        let collapsed_parent_ids: BTreeSet<crate::EventId> =
            collapsed_parent_ids.into_iter().collect();
        let hidden_child_ids: BTreeSet<crate::EventId> = self
            .active_packs()
            .flat_map(|pack| pack.event_relations.iter())
            .filter(|relation| collapsed_parent_ids.contains(&relation.parent_event_id))
            .map(|relation| relation.child_event_id)
            .collect();

        self.active_packs()
            .flat_map(|pack| pack.events.iter())
            .filter(|event| !hidden_child_ids.contains(&event.id))
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

        pack.event_relations.push(EventRelation::new(
            EventId(100),
            EventId(1),
            crate::EventRelationKind::Starts,
        ));

        assert_eq!(pack.events_for_entity(PersonId(7)).len(), 2);
        assert_eq!(pack.events_in_year_span(YearSpan::exact(1901)).len(), 1);
        assert_eq!(pack.child_event_relations(EventId(100)).len(), 1);
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

    #[test]
    fn timeline_document_toggles_and_removes_packs() {
        let pack_id = PackId::new("pack:toggle");
        let pack = EventPack::empty(
            PackMetadata::new(pack_id.clone(), "Toggle Pack"),
            PackKind::UserJournal,
        );
        let mut document = TimelineDocument::empty();

        document.add_pack(pack, false);
        assert!(!document.is_pack_active(&pack_id));

        document.activate_pack(&pack_id);
        assert!(document.is_pack_active(&pack_id));

        document.deactivate_pack(&pack_id);
        assert!(!document.is_pack_active(&pack_id));

        let removed = document.remove_pack(&pack_id);
        assert!(removed.is_some());
        assert!(document.pack(&pack_id).is_none());
    }

    #[test]
    fn timeline_document_filters_active_events_and_hides_collapsed_children() {
        let mut pack = EventPack::empty(
            PackMetadata::new(PackId::new("pack:bio"), "Biography"),
            PackKind::Biography,
        );
        pack.events.push(TimelineEvent::new(
            EventId(100),
            EventTypeId::new("genealogy.life"),
            "Life",
        ));
        pack.events.push(
            TimelineEvent::new(EventId(1), EventTypeId::new("genealogy.birth"), "Birth")
                .with_time(TimeSpec::from_date_value(DateValue::from_original(
                    "1901",
                    Provenance::default(),
                )))
                .with_participant(EventParticipant::new(PersonId(7), "child")),
        );
        pack.event_relations.push(EventRelation::new(
            EventId(100),
            EventId(1),
            crate::EventRelationKind::Starts,
        ));
        let mut document = TimelineDocument::empty();
        document.add_pack(pack, true);

        assert_eq!(document.active_events_for_entity(PersonId(7)).len(), 1);
        assert_eq!(
            document
                .active_events_in_year_span(YearSpan::exact(1901))
                .len(),
            1
        );

        let visible_ids: Vec<EventId> = document
            .visible_active_events([EventId(100)])
            .into_iter()
            .map(|event| event.id)
            .collect();
        assert_eq!(visible_ids, vec![EventId(100)]);
    }
}
