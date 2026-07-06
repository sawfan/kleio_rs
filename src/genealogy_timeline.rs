//! Genealogy-specific timeline composition helpers.
//!
//! This module builds on generic event composition without changing the
//! genealogy import/archive model. It provides convenience projections such as a
//! person's scale-relative "life" interval composed from birth/death boundary
//! events.

use crate::event::{
    EventParticipant, EventRelation, EventRelationKind, EventScaleKind, TimelineEvent,
};
use crate::event_adapter::{GENEALOGY_BIRTH_TYPE, GENEALOGY_DEATH_TYPE, ROLE_SUBJECT};
use crate::event_composition::{
    boundary_relations_for_composite, composite_interval_from_boundaries, event_by_id,
};
use crate::event_type::EventTypeId;
use crate::model::{EventId, PersonId};

pub const GENEALOGY_LIFE_TYPE: &str = "genealogy.life";

pub fn person_life_event(
    id: EventId,
    person_id: PersonId,
    title: impl Into<String>,
    birth_event: Option<&TimelineEvent>,
    death_event: Option<&TimelineEvent>,
) -> TimelineEvent {
    composite_interval_from_boundaries(
        id,
        EventTypeId::new(GENEALOGY_LIFE_TYPE),
        title,
        birth_event,
        death_event,
    )
    .with_participant(EventParticipant::new(person_id, ROLE_SUBJECT))
}

pub fn person_life_relations(
    life_event_id: EventId,
    birth_event_id: Option<EventId>,
    death_event_id: Option<EventId>,
) -> Vec<EventRelation> {
    boundary_relations_for_composite(life_event_id, birth_event_id, death_event_id)
}

pub fn find_person_birth_event<'a>(
    events: &'a [TimelineEvent],
    person_id: PersonId,
) -> Option<&'a TimelineEvent> {
    find_person_event_by_type(events, person_id, GENEALOGY_BIRTH_TYPE)
}

pub fn find_person_death_event<'a>(
    events: &'a [TimelineEvent],
    person_id: PersonId,
) -> Option<&'a TimelineEvent> {
    find_person_event_by_type(events, person_id, GENEALOGY_DEATH_TYPE)
}

pub fn compose_person_life_from_events(
    life_event_id: EventId,
    person_id: PersonId,
    title: impl Into<String>,
    events: &[TimelineEvent],
) -> (TimelineEvent, Vec<EventRelation>) {
    let birth_event = find_person_birth_event(events, person_id);
    let death_event = find_person_death_event(events, person_id);
    let life_event = person_life_event(life_event_id, person_id, title, birth_event, death_event);
    let relations = person_life_relations(
        life_event_id,
        birth_event.map(|event| event.id),
        death_event.map(|event| event.id),
    );

    (life_event, relations)
}

pub fn is_life_event(event: &TimelineEvent) -> bool {
    event.type_ref.as_str() == GENEALOGY_LIFE_TYPE
        && event.is_scale_kind(EventScaleKind::Composite)
        && event.is_scale_kind(EventScaleKind::Interval)
}

pub fn life_boundary_events<'a>(
    events: &'a [TimelineEvent],
    relations: &[EventRelation],
    life_event_id: EventId,
) -> (Option<&'a TimelineEvent>, Option<&'a TimelineEvent>) {
    let start = relations
        .iter()
        .find(|relation| {
            relation.parent_event_id == life_event_id && relation.kind == EventRelationKind::Starts
        })
        .and_then(|relation| event_by_id(events, relation.child_event_id));
    let end = relations
        .iter()
        .find(|relation| {
            relation.parent_event_id == life_event_id && relation.kind == EventRelationKind::Ends
        })
        .and_then(|relation| event_by_id(events, relation.child_event_id));

    (start, end)
}

fn find_person_event_by_type<'a>(
    events: &'a [TimelineEvent],
    person_id: PersonId,
    event_type: &str,
) -> Option<&'a TimelineEvent> {
    events.iter().find(|event| {
        event.type_ref.as_str() == event_type
            && event
                .participants
                .iter()
                .any(|participant| participant.entity == person_id.into())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DateValue, EventParticipant, Provenance, TimeSpec};

    #[test]
    fn compose_person_life_uses_birth_and_death_boundaries() {
        let birth = TimelineEvent::new(
            EventId(1),
            EventTypeId::new(GENEALOGY_BIRTH_TYPE),
            "Birth of Ada",
        )
        .with_time(TimeSpec::from_date_value(DateValue::from_original(
            "1815",
            Provenance::default(),
        )))
        .with_participant(EventParticipant::new(PersonId(7), "child"));
        let death = TimelineEvent::new(
            EventId(2),
            EventTypeId::new(GENEALOGY_DEATH_TYPE),
            "Death of Ada",
        )
        .with_time(TimeSpec::from_date_value(DateValue::from_original(
            "1852",
            Provenance::default(),
        )))
        .with_participant(EventParticipant::new(PersonId(7), "deceased"));
        let events = vec![birth, death];

        let (life, relations) = compose_person_life_from_events(
            EventId(100),
            PersonId(7),
            "Ada Lovelace lived",
            &events,
        );

        assert!(is_life_event(&life));
        assert_eq!(life.time.display(), "1815 to 1852");
        assert_eq!(relations.len(), 2);
    }

    #[test]
    fn life_boundary_lookup_returns_start_and_end_events() {
        let events = vec![
            TimelineEvent::new(EventId(1), EventTypeId::new(GENEALOGY_BIRTH_TYPE), "Birth"),
            TimelineEvent::new(EventId(2), EventTypeId::new(GENEALOGY_DEATH_TYPE), "Death"),
        ];
        let relations = person_life_relations(EventId(100), Some(EventId(1)), Some(EventId(2)));

        let (start, end) = life_boundary_events(&events, &relations, EventId(100));

        assert_eq!(start.map(|event| event.id), Some(EventId(1)));
        assert_eq!(end.map(|event| event.id), Some(EventId(2)));
    }
}
