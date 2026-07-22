//! Adapters between genealogy projections and canonical events.
//!
//! New importer and timeline code should produce `EventPack` / `TimelineEvent`
//! values first. These adapters are for genealogy-specific archive/tree
//! projections such as GEDCOM-derived family-tree data.

use crate::entity::{Entity, EntityId, EntityKind};
use crate::event::{
    EventBoundaryKind, EventCompositionKind, EventParticipant, TimeSpec, TimelineEvent,
};
use crate::event_type::EventTypeId;
use crate::genealogy_event::{GenealogyEvent, GenealogyEventKind};
use crate::model::{GenealogyIndex, PersonId};
use crate::pack::{EventPack, PackKind, PackMetadata, SourceRecord};

pub const GENEALOGY_BIRTH_TYPE: &str = "genealogy.birth";
pub const GENEALOGY_DEATH_TYPE: &str = "genealogy.death";
pub const GENEALOGY_MARRIAGE_TYPE: &str = "genealogy.marriage";
pub const GENEALOGY_BAPTISM_TYPE: &str = "genealogy.baptism";
pub const GENEALOGY_BURIAL_TYPE: &str = "genealogy.burial";
pub const GENEALOGY_RESIDENCE_TYPE: &str = "genealogy.residence";
pub const GENEALOGY_OCCUPATION_TYPE: &str = "genealogy.occupation";

pub const ROLE_SUBJECT: &str = "subject";
pub const ROLE_CHILD: &str = "child";
pub const ROLE_DECEASED: &str = "deceased";
pub const ROLE_SPOUSE: &str = "spouse";
pub const ROLE_RESIDENT: &str = "resident";
pub const ROLE_PARTICIPANT: &str = "participant";

pub fn genealogy_event_type_id(kind: &GenealogyEventKind) -> EventTypeId {
    match kind {
        GenealogyEventKind::Birth => EventTypeId::new(GENEALOGY_BIRTH_TYPE),
        GenealogyEventKind::Death => EventTypeId::new(GENEALOGY_DEATH_TYPE),
        GenealogyEventKind::Marriage => EventTypeId::new(GENEALOGY_MARRIAGE_TYPE),
        GenealogyEventKind::Baptism => EventTypeId::new(GENEALOGY_BAPTISM_TYPE),
        GenealogyEventKind::Burial => EventTypeId::new(GENEALOGY_BURIAL_TYPE),
        GenealogyEventKind::Residence => EventTypeId::new(GENEALOGY_RESIDENCE_TYPE),
        GenealogyEventKind::Occupation => EventTypeId::new(GENEALOGY_OCCUPATION_TYPE),
        GenealogyEventKind::Other(value) => EventTypeId::new(format!("genealogy.custom.{value}")),
    }
}

pub fn genealogy_event_role(kind: &GenealogyEventKind) -> &'static str {
    match kind {
        GenealogyEventKind::Birth => ROLE_CHILD,
        GenealogyEventKind::Death | GenealogyEventKind::Burial => ROLE_DECEASED,
        GenealogyEventKind::Marriage => ROLE_SPOUSE,
        GenealogyEventKind::Residence => ROLE_RESIDENT,
        GenealogyEventKind::Baptism
        | GenealogyEventKind::Occupation
        | GenealogyEventKind::Other(_) => ROLE_SUBJECT,
    }
}

pub fn genealogy_event_label(kind: &GenealogyEventKind) -> String {
    match kind {
        GenealogyEventKind::Birth => "Birth".to_string(),
        GenealogyEventKind::Death => "Death".to_string(),
        GenealogyEventKind::Marriage => "Marriage".to_string(),
        GenealogyEventKind::Baptism => "Baptism".to_string(),
        GenealogyEventKind::Burial => "Burial".to_string(),
        GenealogyEventKind::Residence => "Residence".to_string(),
        GenealogyEventKind::Occupation => "Occupation".to_string(),
        GenealogyEventKind::Other(value) => value.clone(),
    }
}

pub fn genealogy_event_classification(
    kind: &GenealogyEventKind,
) -> (
    EventCompositionKind,
    crate::EventTemporalKind,
    EventBoundaryKind,
) {
    match kind {
        GenealogyEventKind::Birth => (
            EventCompositionKind::Atomic,
            crate::EventTemporalKind::Instant,
            EventBoundaryKind::Start,
        ),
        GenealogyEventKind::Death => (
            EventCompositionKind::Atomic,
            crate::EventTemporalKind::Instant,
            EventBoundaryKind::End,
        ),
        GenealogyEventKind::Residence | GenealogyEventKind::Occupation => (
            EventCompositionKind::Atomic,
            crate::EventTemporalKind::Interval,
            EventBoundaryKind::None,
        ),
        GenealogyEventKind::Marriage
        | GenealogyEventKind::Baptism
        | GenealogyEventKind::Burial
        | GenealogyEventKind::Other(_) => (
            EventCompositionKind::Atomic,
            crate::EventTemporalKind::Instant,
            EventBoundaryKind::None,
        ),
    }
}

pub fn genealogy_event_scale_kinds(kind: &GenealogyEventKind) -> Vec<crate::EventScaleKind> {
    match kind {
        GenealogyEventKind::Birth | GenealogyEventKind::Death => {
            vec![
                crate::EventScaleKind::Atomic,
                crate::EventScaleKind::Boundary,
            ]
        }
        GenealogyEventKind::Marriage
        | GenealogyEventKind::Baptism
        | GenealogyEventKind::Burial
        | GenealogyEventKind::Residence
        | GenealogyEventKind::Occupation
        | GenealogyEventKind::Other(_) => vec![crate::EventScaleKind::Atomic],
    }
}

pub fn timeline_event_from_genealogy_event(event: &GenealogyEvent) -> TimelineEvent {
    let mut timeline_event = TimelineEvent::new(
        event.id,
        genealogy_event_type_id(&event.kind),
        event
            .description
            .clone()
            .unwrap_or_else(|| genealogy_event_label(&event.kind)),
    );

    timeline_event.time = event
        .date
        .clone()
        .map(TimeSpec::from_date_value)
        .unwrap_or(TimeSpec::Unknown);
    let (composition, temporal, boundary) = genealogy_event_classification(&event.kind);
    timeline_event.composition = composition;
    timeline_event.temporal = temporal;
    timeline_event.boundary = boundary;
    timeline_event.scale_kinds = genealogy_event_scale_kinds(&event.kind);
    timeline_event.place = event.place;
    timeline_event.description = event.description.clone();
    timeline_event.provenance = event.provenance.clone();
    timeline_event.sources = event.provenance.sources.clone();
    timeline_event.tags = event.provenance.tags.clone();

    let role = genealogy_event_role(&event.kind);
    timeline_event.participants = event
        .participants
        .iter()
        .copied()
        .map(|person_id| EventParticipant::new(person_id, role))
        .collect();

    timeline_event
}

pub fn timeline_events_from_genealogy_events<'a>(
    events: impl IntoIterator<Item = &'a GenealogyEvent>,
) -> Vec<TimelineEvent> {
    events
        .into_iter()
        .map(timeline_event_from_genealogy_event)
        .collect()
}

pub fn timeline_events_for_person<'a>(
    events: impl IntoIterator<Item = &'a GenealogyEvent>,
    person_id: PersonId,
) -> Vec<TimelineEvent> {
    events
        .into_iter()
        .filter(|event| event.participants.contains(&person_id))
        .map(timeline_event_from_genealogy_event)
        .collect()
}

/// Project a genealogy index into the canonical event-pack model.
///
/// This is the preferred boundary for importers that still need to build a
/// genealogy index for tree compatibility: produce an `EventPack` for event
/// consumers, then keep the index as a genealogy-specific projection.
pub fn event_pack_from_genealogy_index(
    index: &GenealogyIndex,
    metadata: PackMetadata,
    kind: PackKind,
) -> EventPack {
    let mut pack = EventPack::empty(metadata, kind);
    pack.domain_profiles.push(crate::genealogy_domain_profile());
    pack.entities = entities_from_genealogy_index(index);
    pack.events = timeline_events_from_genealogy_events(&index.events);
    pack.sources = source_records_from_genealogy_index(index);
    pack
}

fn entities_from_genealogy_index(index: &GenealogyIndex) -> Vec<Entity> {
    let mut entities = Vec::new();

    for person in &index.people {
        let name = person
            .names
            .first()
            .map(|name| name.display.clone())
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| format!("Person {}", person.id.0));
        let mut entity = Entity::new(
            EntityId::new(format!("person:{}", person.id.0)),
            EntityKind::Person,
            name,
        );
        entity.sources = person.provenance.sources.clone();
        entity.provenance = person.provenance.clone();
        entities.push(entity);
    }

    for place in &index.places {
        let mut entity = Entity::new(
            EntityId::new(format!("place:{}", place.id.0)),
            EntityKind::Place,
            place.name.clone(),
        );
        entity.provenance = place.provenance.clone();
        entity.sources = place.provenance.sources.clone();
        entities.push(entity);
    }

    entities
}

fn source_records_from_genealogy_index(index: &GenealogyIndex) -> Vec<SourceRecord> {
    let mut sources = Vec::new();
    for source_ref in index
        .people
        .iter()
        .flat_map(|person| {
            person
                .provenance
                .sources
                .iter()
                .chain(person.source_record.iter())
        })
        .chain(
            index
                .events
                .iter()
                .flat_map(|event| event.provenance.sources.iter()),
        )
        .chain(
            index
                .families
                .iter()
                .flat_map(|family| family.provenance.sources.iter()),
        )
    {
        if sources
            .iter()
            .any(|source: &SourceRecord| source.id == *source_ref)
        {
            continue;
        }
        sources.push(SourceRecord::new(source_ref.clone(), source_ref.0.clone()));
    }
    sources
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DateValue, EntityRef, EventId, Provenance, genealogy_domain_profile};

    #[test]
    fn birth_event_maps_to_genealogy_birth_type_and_child_role() {
        let event = GenealogyEvent {
            id: EventId(7),
            kind: GenealogyEventKind::Birth,
            date: Some(DateValue::from_original("1901", Provenance::default())),
            time: None,
            time_zone: None,
            place: None,
            description: None,
            participants: vec![PersonId(42)],
            provenance: Provenance::default(),
        };

        let timeline_event = timeline_event_from_genealogy_event(&event);

        assert_eq!(timeline_event.type_ref.as_str(), GENEALOGY_BIRTH_TYPE);
        assert_eq!(timeline_event.title, "Birth");
        assert_eq!(timeline_event.time.display(), "1901");
        assert!(timeline_event.is_scale_kind(crate::EventScaleKind::Atomic));
        assert_eq!(timeline_event.temporal, crate::EventTemporalKind::Instant);
        assert_eq!(timeline_event.boundary, crate::EventBoundaryKind::Start);
        assert!(timeline_event.is_scale_kind(crate::EventScaleKind::Boundary));
        assert_eq!(timeline_event.participants.len(), 1);
        assert_eq!(timeline_event.participants[0].role.as_str(), ROLE_CHILD);
        assert_eq!(
            timeline_event.participants[0].entity,
            EntityRef::Person(PersonId(42))
        );
    }

    #[test]
    fn built_in_genealogy_event_kinds_map_to_profile_types() {
        let profile = genealogy_domain_profile();
        let kinds = [
            GenealogyEventKind::Birth,
            GenealogyEventKind::Death,
            GenealogyEventKind::Marriage,
            GenealogyEventKind::Baptism,
            GenealogyEventKind::Burial,
            GenealogyEventKind::Residence,
            GenealogyEventKind::Occupation,
        ];

        for kind in kinds {
            let event_type_id = genealogy_event_type_id(&kind);
            assert!(
                profile.event_type(&event_type_id).is_some(),
                "missing profile event type for {}",
                event_type_id.as_str()
            );
        }
    }

    #[test]
    fn person_filter_maps_only_matching_events() {
        let matching = GenealogyEvent {
            id: EventId(1),
            kind: GenealogyEventKind::Residence,
            date: None,
            time: None,
            time_zone: None,
            place: None,
            description: None,
            participants: vec![PersonId(1)],
            provenance: Provenance::default(),
        };
        let other = GenealogyEvent {
            id: EventId(2),
            kind: GenealogyEventKind::Residence,
            date: None,
            time: None,
            time_zone: None,
            place: None,
            description: None,
            participants: vec![PersonId(2)],
            provenance: Provenance::default(),
        };
        let events = vec![matching, other];

        let timeline_events = timeline_events_for_person(&events, PersonId(1));

        assert_eq!(timeline_events.len(), 1);
        assert_eq!(timeline_events[0].id, EventId(1));
    }
}
