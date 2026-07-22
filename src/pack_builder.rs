//! Programmatic builders for small/user-authored event packs.
//!
//! These helpers are intended for UI forms and tests. They keep manual pack
//! creation ergonomic without bypassing the same `EventPack` primitives used by
//! imports and serialized packs.

use rkyv::{Archive, Deserialize, Serialize};

use crate::attribution::{Provenance, SourceRef, Tag};
use crate::entity::Entity;
use crate::event::{
    EventCompositionKind, EventParticipant, EventRelation, EventScaleKind, EventTemporalKind,
    TimeSpec, TimelineEvent,
};
use crate::event_collection::EventCollection;
use crate::event_type::{DomainProfile, EventTypeId};
use crate::event_validation::{
    EventValidationIssue, ValidationSeverity, validate_event_collections, validate_event_relations,
    validate_timeline_event,
};
use crate::model::{EventId, PlaceId};
use crate::pack::{EventPack, PackKind, PackMetadata, SourceRecord};

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
pub struct ManualEventDraft {
    pub id: Option<EventId>,
    pub title: String,
    pub type_ref: EventTypeId,
    pub time: TimeSpec,
    pub composition: EventCompositionKind,
    pub temporal: EventTemporalKind,
    pub boundary: crate::EventBoundaryKind,
    pub scale_kinds: Vec<EventScaleKind>,
    pub place: Option<PlaceId>,
    pub participants: Vec<EventParticipant>,
    pub description: Option<String>,
    pub sources: Vec<SourceRef>,
    pub tags: Vec<Tag>,
    pub provenance: Provenance,
}

impl ManualEventDraft {
    pub fn new(type_ref: EventTypeId, title: impl Into<String>) -> Self {
        Self {
            id: None,
            title: title.into(),
            type_ref,
            time: TimeSpec::Unknown,
            composition: EventCompositionKind::Atomic,
            temporal: EventTemporalKind::Instant,
            boundary: crate::EventBoundaryKind::None,
            scale_kinds: vec![EventScaleKind::Atomic],
            place: None,
            participants: Vec::new(),
            description: None,
            sources: Vec::new(),
            tags: Vec::new(),
            provenance: Provenance::default(),
        }
    }

    pub fn with_id(mut self, id: EventId) -> Self {
        self.id = Some(id);
        self
    }

    pub fn with_time(mut self, time: TimeSpec) -> Self {
        self.time = time;
        self
    }

    pub fn with_scale_kinds(
        mut self,
        scale_kinds: impl IntoIterator<Item = EventScaleKind>,
    ) -> Self {
        self.scale_kinds.clear();
        for scale_kind in scale_kinds {
            if !self.scale_kinds.contains(&scale_kind) {
                self.scale_kinds.push(scale_kind);
            }
        }
        if self.scale_kinds.is_empty() {
            self.scale_kinds.push(EventScaleKind::Atomic);
        }
        sync_axes_from_scale_kind_list(
            &self.scale_kinds,
            &mut self.composition,
            &mut self.temporal,
            &mut self.boundary,
        );
        self
    }

    pub fn with_composition_kind(mut self, composition: EventCompositionKind) -> Self {
        self.composition = composition;
        self.scale_kinds = scale_kinds_from_axes(&self.composition, &self.temporal, &self.boundary);
        self
    }

    pub fn with_temporal_kind(mut self, temporal: EventTemporalKind) -> Self {
        self.temporal = temporal;
        self.scale_kinds = scale_kinds_from_axes(&self.composition, &self.temporal, &self.boundary);
        self
    }

    pub fn with_boundary_kind(mut self, boundary: crate::EventBoundaryKind) -> Self {
        self.boundary = boundary;
        self.scale_kinds = scale_kinds_from_axes(&self.composition, &self.temporal, &self.boundary);
        self
    }

    pub fn with_participant(mut self, participant: EventParticipant) -> Self {
        self.participants.push(participant);
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackEventValidation {
    pub event_id: EventId,
    pub issues: Vec<EventValidationIssue>,
}

impl PackEventValidation {
    pub fn has_errors(&self) -> bool {
        self.issues
            .iter()
            .any(|issue| issue.severity == ValidationSeverity::Error)
    }

    pub fn has_warnings(&self) -> bool {
        self.issues
            .iter()
            .any(|issue| issue.severity == ValidationSeverity::Warning)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EventPackBuilder {
    pack: EventPack,
}

impl EventPackBuilder {
    pub fn new(metadata: PackMetadata, kind: PackKind) -> Self {
        Self {
            pack: EventPack::empty(metadata, kind),
        }
    }

    pub fn from_pack(pack: EventPack) -> Self {
        Self { pack }
    }

    pub fn pack(&self) -> &EventPack {
        &self.pack
    }

    pub fn pack_mut(&mut self) -> &mut EventPack {
        &mut self.pack
    }

    pub fn into_pack(self) -> EventPack {
        self.pack
    }

    pub fn next_event_id(&self) -> EventId {
        let next = self
            .pack
            .events
            .iter()
            .map(|event| event.id.0)
            .max()
            .unwrap_or_default()
            + 1;
        EventId(next)
    }

    pub fn add_domain_profile(&mut self, profile: DomainProfile) -> &mut Self {
        self.pack.domain_profiles.push(profile);
        self
    }

    pub fn add_entity(&mut self, entity: Entity) -> &mut Self {
        self.pack.entities.push(entity);
        self
    }

    pub fn add_source(&mut self, source: SourceRecord) -> &mut Self {
        self.pack.sources.push(source);
        self
    }

    pub fn add_tag(&mut self, tag: Tag) -> &mut Self {
        self.pack.tags.push(tag);
        self
    }

    pub fn add_event(&mut self, event: TimelineEvent) -> &mut Self {
        self.pack.events.push(event);
        self
    }

    pub fn add_event_collection(&mut self, collection: EventCollection) -> &mut Self {
        self.pack.event_collections.push(collection);
        self
    }

    pub fn add_event_relation(&mut self, relation: EventRelation) -> &mut Self {
        self.pack.event_relations.push(relation);
        self
    }

    pub fn add_manual_event(&mut self, draft: ManualEventDraft) -> EventId {
        let event_id = draft.id.unwrap_or_else(|| self.next_event_id());
        let event = TimelineEvent {
            id: event_id,
            title: draft.title,
            type_ref: draft.type_ref,
            time: draft.time,
            composition: draft.composition,
            temporal: draft.temporal,
            boundary: draft.boundary,
            scale_kinds: draft.scale_kinds,
            place: draft.place,
            participants: draft.participants,
            description: draft.description,
            sources: draft.sources,
            tags: draft.tags,
            provenance: draft.provenance,
        };
        self.pack.events.push(event);
        event_id
    }

    pub fn add_manual_event_validated(
        &mut self,
        draft: ManualEventDraft,
    ) -> (EventId, Vec<EventValidationIssue>) {
        let event_id = self.add_manual_event(draft);
        let issues = self
            .pack
            .events
            .iter()
            .find(|event| event.id == event_id)
            .map(|event| validate_timeline_event(event, &self.pack.domain_profiles))
            .unwrap_or_default();
        (event_id, issues)
    }

    pub fn validate_event(&self, event_id: EventId) -> Option<PackEventValidation> {
        let event = self.pack.events.iter().find(|event| event.id == event_id)?;
        let issues = validate_timeline_event(event, &self.pack.domain_profiles);
        (!issues.is_empty()).then_some(PackEventValidation { event_id, issues })
    }

    pub fn validate_events(&self) -> Vec<PackEventValidation> {
        let mut validations: Vec<PackEventValidation> = self
            .pack
            .events
            .iter()
            .filter_map(|event| self.validate_event(event.id))
            .collect();

        for issue in validate_event_relations(&self.pack.events, &self.pack.event_relations) {
            let event_id = match &issue.kind {
                crate::EventValidationIssueKind::BoundaryRoleMismatch { event_id, .. }
                | crate::EventValidationIssueKind::BoundaryRoleInferred { event_id, .. } => {
                    *event_id
                }
                _ => continue,
            };
            if let Some(validation) = validations
                .iter_mut()
                .find(|validation| validation.event_id == event_id)
            {
                validation.issues.push(issue);
            } else {
                validations.push(PackEventValidation {
                    event_id,
                    issues: vec![issue],
                });
            }
        }

        for issue in validate_event_collections(&self.pack.event_collections, &self.pack.events) {
            let event_id = match &issue.kind {
                crate::EventValidationIssueKind::CollectionMissingEvent { event_id, .. }
                | crate::EventValidationIssueKind::CollectionDuplicateMember { event_id, .. } => {
                    *event_id
                }
                _ => continue,
            };
            if let Some(validation) = validations
                .iter_mut()
                .find(|validation| validation.event_id == event_id)
            {
                validation.issues.push(issue);
            } else {
                validations.push(PackEventValidation {
                    event_id,
                    issues: vec![issue],
                });
            }
        }

        validations
    }
}

fn sync_axes_from_scale_kind_list(
    scale_kinds: &[EventScaleKind],
    composition: &mut EventCompositionKind,
    temporal: &mut EventTemporalKind,
    boundary: &mut crate::EventBoundaryKind,
) {
    *composition = if scale_kinds.contains(&EventScaleKind::Composite) {
        EventCompositionKind::Composite
    } else {
        EventCompositionKind::Atomic
    };
    *temporal = if scale_kinds.contains(&EventScaleKind::Interval) {
        EventTemporalKind::Interval
    } else {
        EventTemporalKind::Instant
    };
    *boundary = if scale_kinds.contains(&EventScaleKind::Boundary) {
        crate::EventBoundaryKind::StartAndEnd
    } else {
        crate::EventBoundaryKind::None
    };
}

fn scale_kinds_from_axes(
    composition: &EventCompositionKind,
    temporal: &EventTemporalKind,
    boundary: &crate::EventBoundaryKind,
) -> Vec<EventScaleKind> {
    let mut scale_kinds = Vec::new();
    scale_kinds.push(match composition {
        EventCompositionKind::Atomic => EventScaleKind::Atomic,
        EventCompositionKind::Composite => EventScaleKind::Composite,
    });
    if *temporal == EventTemporalKind::Interval {
        scale_kinds.push(EventScaleKind::Interval);
    }
    if *boundary != crate::EventBoundaryKind::None {
        scale_kinds.push(EventScaleKind::Boundary);
    }
    scale_kinds
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DateValue, PackId, PersonId, genealogy_domain_profile};

    #[test]
    fn builder_allocates_event_ids_for_manual_events() {
        let mut builder = EventPackBuilder::new(
            PackMetadata::new(PackId::new("pack:journal"), "Journal"),
            PackKind::UserJournal,
        );

        let first_id = builder.add_manual_event(
            ManualEventDraft::new(EventTypeId::new("journal.entry"), "First entry").with_time(
                TimeSpec::from_date_value(DateValue::from_original("2026", Provenance::default())),
            ),
        );
        let second_id = builder.add_manual_event(
            ManualEventDraft::new(EventTypeId::new("journal.entry"), "Second entry")
                .with_participant(EventParticipant::new(PersonId(7), "subject")),
        );
        let pack = builder.into_pack();

        assert_eq!(first_id, EventId(1));
        assert_eq!(second_id, EventId(2));
        assert_eq!(pack.events.len(), 2);
        assert_eq!(pack.events[0].time.display(), "2026");
    }

    #[test]
    fn builder_preserves_explicit_event_ids() {
        let mut builder = EventPackBuilder::new(
            PackMetadata::new(PackId::new("pack:history"), "History"),
            PackKind::HistoricalTimeline,
        );

        let event_id = builder.add_manual_event(
            ManualEventDraft::new(EventTypeId::new("history.event"), "Known event")
                .with_id(EventId(42)),
        );

        assert_eq!(event_id, EventId(42));
        assert_eq!(builder.next_event_id(), EventId(43));
    }

    #[test]
    fn builder_validates_manual_events_against_domain_profiles() {
        let mut builder = EventPackBuilder::new(
            PackMetadata::new(PackId::new("pack:family"), "Family"),
            PackKind::Genealogy,
        );
        builder.add_domain_profile(genealogy_domain_profile());

        let (event_id, issues) = builder.add_manual_event_validated(ManualEventDraft::new(
            EventTypeId::new("genealogy.birth"),
            "Incomplete birth",
        ));
        let validations = builder.validate_events();

        assert_eq!(event_id, EventId(1));
        assert!(
            issues
                .iter()
                .any(|issue| issue.severity == ValidationSeverity::Error)
        );
        assert_eq!(validations.len(), 1);
        assert!(validations[0].has_errors());
    }
}
