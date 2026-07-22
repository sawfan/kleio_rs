//! Biography timeline projections.
//!
//! A biography timeline is a view model over generic `TimelineEvent` values. It
//! can include scale-relative macro events, such as a person's life interval,
//! while preserving the underlying birth/death/detail events.

use crate::entity::EntityRef;
use crate::event::{EventRelation, TimelineEvent};
use crate::event_composition::timeline_events_with_collapsed_parents;
use crate::event_query::{TimelineEventFilter, YearSpan, filter_timeline_events};
use crate::genealogy_timeline::{
    GENEALOGY_LIFE_TYPE, compose_person_life_from_events, is_life_event,
};
use crate::model::{EventId, PersonId};
use crate::pack::{EventPack, PackId, PackKind, PackMetadata};
use crate::{GenealogyEvent, timeline_event_from_genealogy_event, timeline_events_for_person};

#[derive(Debug, Clone, PartialEq)]
pub struct BiographyTimeline {
    pub subject: EntityRef,
    pub events: Vec<TimelineEvent>,
    pub relations: Vec<EventRelation>,
    pub macro_event_ids: Vec<EventId>,
}

impl BiographyTimeline {
    pub fn empty(subject: EntityRef) -> Self {
        Self {
            subject,
            events: Vec::new(),
            relations: Vec::new(),
            macro_event_ids: Vec::new(),
        }
    }

    pub fn visible_events(&self, collapsed: bool) -> Vec<&TimelineEvent> {
        if collapsed {
            timeline_events_with_collapsed_parents(
                &self.events,
                &self.relations,
                self.macro_event_ids.iter().copied(),
            )
        } else {
            self.events.iter().collect()
        }
    }

    pub fn filtered_events(&self, filter: &TimelineEventFilter) -> Vec<&TimelineEvent> {
        filter_timeline_events(&self.events, filter)
    }

    pub fn events_in_year_span(&self, years: YearSpan) -> Vec<&TimelineEvent> {
        let filter = TimelineEventFilter::new().with_year_span(years);
        self.filtered_events(&filter)
    }

    pub fn into_event_pack(self, metadata: PackMetadata, kind: PackKind) -> EventPack {
        let mut pack = EventPack::empty(metadata, kind);
        pack.events = self.events;
        pack.event_relations = self.relations;
        pack
    }
}

pub fn biography_timeline_for_person(
    person_id: PersonId,
    genealogy_events: &[GenealogyEvent],
    life_event_id: Option<EventId>,
    life_title: impl Into<String>,
) -> BiographyTimeline {
    let mut timeline = BiographyTimeline::empty(EntityRef::Person(person_id));
    timeline.events = timeline_events_for_person(genealogy_events, person_id);

    if let Some(life_event_id) = life_event_id {
        let (life_event, relations) =
            compose_person_life_from_events(life_event_id, person_id, life_title, &timeline.events);
        if is_life_event(&life_event) && !relations.is_empty() {
            timeline.macro_event_ids.push(life_event.id);
            timeline.events.push(life_event);
            timeline.relations.extend(relations);
        }
    }

    timeline
}

pub fn biography_event_pack_for_person(
    pack_id: PackId,
    pack_title: impl Into<String>,
    person_id: PersonId,
    genealogy_events: &[GenealogyEvent],
    life_event_id: Option<EventId>,
    life_title: impl Into<String>,
) -> EventPack {
    let timeline =
        biography_timeline_for_person(person_id, genealogy_events, life_event_id, life_title);
    let mut metadata = PackMetadata::new(pack_id, pack_title);
    metadata.description = Some(format!(
        "Biography timeline projection for person {} using {}.",
        person_id.0, GENEALOGY_LIFE_TYPE
    ));
    timeline.into_event_pack(metadata, PackKind::Biography)
}

pub fn biography_timeline_from_timeline_events(
    subject: EntityRef,
    events: impl IntoIterator<Item = TimelineEvent>,
    relations: impl IntoIterator<Item = EventRelation>,
    macro_event_ids: impl IntoIterator<Item = EventId>,
) -> BiographyTimeline {
    BiographyTimeline {
        subject,
        events: events.into_iter().collect(),
        relations: relations.into_iter().collect(),
        macro_event_ids: macro_event_ids.into_iter().collect(),
    }
}

pub fn adapt_genealogy_events_for_biography(
    genealogy_events: &[GenealogyEvent],
    person_id: PersonId,
) -> Vec<TimelineEvent> {
    genealogy_events
        .iter()
        .filter(|event| event.participants.contains(&person_id))
        .map(timeline_event_from_genealogy_event)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DateValue, GenealogyEventKind, Provenance, TimeSpec};

    fn genealogy_event(
        id: u64,
        kind: GenealogyEventKind,
        year: &str,
        person_id: PersonId,
    ) -> GenealogyEvent {
        GenealogyEvent {
            id: EventId(id),
            kind,
            date: Some(DateValue::from_original(year, Provenance::default())),
            time: None,
            time_zone: None,
            place: None,
            description: None,
            participants: vec![person_id],
            provenance: Provenance::default(),
        }
    }

    #[test]
    fn biography_timeline_composes_life_macro_event() {
        let person_id = PersonId(7);
        let events = vec![
            genealogy_event(1, GenealogyEventKind::Birth, "1815", person_id),
            genealogy_event(2, GenealogyEventKind::Death, "1852", person_id),
        ];

        let timeline = biography_timeline_for_person(
            person_id,
            &events,
            Some(EventId(100)),
            "Ada Lovelace lived",
        );

        assert_eq!(timeline.events.len(), 3);
        assert_eq!(timeline.relations.len(), 2);
        assert_eq!(timeline.macro_event_ids, vec![EventId(100)]);
        assert!(timeline.events.iter().any(is_life_event));
    }

    #[test]
    fn collapsed_biography_hides_life_boundary_children() {
        let person_id = PersonId(7);
        let events = vec![
            genealogy_event(1, GenealogyEventKind::Birth, "1815", person_id),
            genealogy_event(2, GenealogyEventKind::Death, "1852", person_id),
        ];
        let timeline = biography_timeline_for_person(
            person_id,
            &events,
            Some(EventId(100)),
            "Ada Lovelace lived",
        );

        let visible_ids: Vec<EventId> = timeline
            .visible_events(true)
            .into_iter()
            .map(|event| event.id)
            .collect();

        assert_eq!(visible_ids, vec![EventId(100)]);
    }

    #[test]
    fn biography_timeline_can_materialize_event_pack() {
        let person_id = PersonId(7);
        let events = vec![
            genealogy_event(1, GenealogyEventKind::Birth, "1815", person_id),
            genealogy_event(2, GenealogyEventKind::Death, "1852", person_id),
        ];

        let pack = biography_event_pack_for_person(
            PackId::new("pack:bio:7"),
            "Ada biography",
            person_id,
            &events,
            Some(EventId(100)),
            "Ada Lovelace lived",
        );

        assert_eq!(pack.kind, PackKind::Biography);
        assert_eq!(pack.events.len(), 3);
        assert_eq!(pack.event_relations.len(), 2);
    }

    #[test]
    fn biography_filter_uses_timeline_query_helpers() {
        let event =
            TimelineEvent::new(EventId(1), crate::EventTypeId::new("journal.entry"), "Note")
                .with_time(TimeSpec::from_date_value(DateValue::from_original(
                    "2026",
                    Provenance::default(),
                )));
        let timeline = biography_timeline_from_timeline_events(
            EntityRef::Person(PersonId(7)),
            vec![event],
            Vec::new(),
            Vec::new(),
        );

        assert_eq!(timeline.events_in_year_span(YearSpan::exact(2026)).len(), 1);
        assert_eq!(timeline.events_in_year_span(YearSpan::exact(1900)).len(), 0);
    }
}
