//! Adapters between the existing genealogy model and generic timeline events.
//!
//! These adapters let new timeline/journal/calendar code consume current
//! GEDCOM/Wikidata-derived `model::Event` values without forcing an immediate
//! rewrite of the genealogy archive format.

use crate::entity::EntityRef;
use crate::event::{EventParticipant, TimeSpec, TimelineEvent};
use crate::event_type::{EventTypeId, genealogy_domain_profile};
use crate::model::{Event, EventKind, PersonId};

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

pub fn genealogy_event_type_id(kind: &EventKind) -> EventTypeId {
    match kind {
        EventKind::Birth => EventTypeId::new(GENEALOGY_BIRTH_TYPE),
        EventKind::Death => EventTypeId::new(GENEALOGY_DEATH_TYPE),
        EventKind::Marriage => EventTypeId::new(GENEALOGY_MARRIAGE_TYPE),
        EventKind::Baptism => EventTypeId::new(GENEALOGY_BAPTISM_TYPE),
        EventKind::Burial => EventTypeId::new(GENEALOGY_BURIAL_TYPE),
        EventKind::Residence => EventTypeId::new(GENEALOGY_RESIDENCE_TYPE),
        EventKind::Occupation => EventTypeId::new(GENEALOGY_OCCUPATION_TYPE),
        EventKind::Other(value) => EventTypeId::new(format!("genealogy.custom.{value}")),
    }
}

pub fn genealogy_event_role(kind: &EventKind) -> &'static str {
    match kind {
        EventKind::Birth => ROLE_CHILD,
        EventKind::Death | EventKind::Burial => ROLE_DECEASED,
        EventKind::Marriage => ROLE_SPOUSE,
        EventKind::Residence => ROLE_RESIDENT,
        EventKind::Baptism | EventKind::Occupation | EventKind::Other(_) => ROLE_SUBJECT,
    }
}

pub fn genealogy_event_label(kind: &EventKind) -> String {
    match kind {
        EventKind::Birth => "Birth".to_string(),
        EventKind::Death => "Death".to_string(),
        EventKind::Marriage => "Marriage".to_string(),
        EventKind::Baptism => "Baptism".to_string(),
        EventKind::Burial => "Burial".to_string(),
        EventKind::Residence => "Residence".to_string(),
        EventKind::Occupation => "Occupation".to_string(),
        EventKind::Other(value) => value.clone(),
    }
}

pub fn timeline_event_from_genealogy_event(event: &Event) -> TimelineEvent {
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
    events: impl IntoIterator<Item = &'a Event>,
) -> Vec<TimelineEvent> {
    events
        .into_iter()
        .map(timeline_event_from_genealogy_event)
        .collect()
}

pub fn timeline_events_for_person<'a>(
    events: impl IntoIterator<Item = &'a Event>,
    person_id: PersonId,
) -> Vec<TimelineEvent> {
    events
        .into_iter()
        .filter(|event| event.participants.contains(&person_id))
        .map(timeline_event_from_genealogy_event)
        .collect()
}

pub fn timeline_event_has_entity(event: &TimelineEvent, entity: &EntityRef) -> bool {
    event
        .participants
        .iter()
        .any(|participant| &participant.entity == entity)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DateValue, EventId, Provenance};

    #[test]
    fn birth_event_maps_to_genealogy_birth_type_and_child_role() {
        let event = Event {
            id: EventId(7),
            kind: EventKind::Birth,
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
            EventKind::Birth,
            EventKind::Death,
            EventKind::Marriage,
            EventKind::Baptism,
            EventKind::Burial,
            EventKind::Residence,
            EventKind::Occupation,
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
        let matching = Event {
            id: EventId(1),
            kind: EventKind::Residence,
            date: None,
            time: None,
            time_zone: None,
            place: None,
            description: None,
            participants: vec![PersonId(1)],
            provenance: Provenance::default(),
        };
        let other = Event {
            id: EventId(2),
            kind: EventKind::Residence,
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
