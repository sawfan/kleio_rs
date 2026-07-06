//! Small sample packs for tests, demos, and UI scaffolding.
//!
//! These packs are intentionally tiny. They demonstrate the event-pack,
//! builder, validation, and composition primitives without depending on GEDCOM,
//! SQLite, or browser storage.

use crate::attribution::Provenance;
use crate::event::{EventParticipant, EventRelationKind, EventScaleKind, TimeSpec};
use crate::event_composition::boundary_relations_for_composite;
use crate::event_type::{EventTypeId, genealogy_domain_profile, journal_domain_profile};
use crate::model::{DateValue, EventId, PersonId};
use crate::pack::{EventPack, PackId, PackKind, PackMetadata};
use crate::pack_builder::{EventPackBuilder, ManualEventDraft};

pub fn sample_journal_pack() -> EventPack {
    let mut builder = EventPackBuilder::new(
        PackMetadata::new(PackId::new("sample:journal"), "Sample journal"),
        PackKind::UserJournal,
    );
    builder.add_domain_profile(journal_domain_profile());
    builder.add_manual_event(
        ManualEventDraft::new(
            EventTypeId::new("journal.entry"),
            "Started a research journal",
        )
        .with_time(TimeSpec::from_date_value(DateValue::from_original(
            "2026-07-06",
            Provenance::default(),
        )))
        .with_description("A small sample journal entry for Kleio timeline UI scaffolding."),
    );

    builder.into_pack()
}

pub fn sample_biography_pack() -> EventPack {
    let person_id = PersonId(1);
    let mut builder = EventPackBuilder::new(
        PackMetadata::new(PackId::new("sample:biography"), "Sample biography"),
        PackKind::Biography,
    );
    builder.add_domain_profile(genealogy_domain_profile());

    let birth_id = builder.add_manual_event(
        ManualEventDraft::new(
            EventTypeId::new("genealogy.birth"),
            "Birth of sample person",
        )
        .with_scale_kinds([EventScaleKind::Atomic, EventScaleKind::Boundary])
        .with_time(TimeSpec::from_date_value(DateValue::from_original(
            "1901",
            Provenance::default(),
        )))
        .with_participant(EventParticipant::new(person_id, "child")),
    );
    let death_id = builder.add_manual_event(
        ManualEventDraft::new(
            EventTypeId::new("genealogy.death"),
            "Death of sample person",
        )
        .with_scale_kinds([EventScaleKind::Atomic, EventScaleKind::Boundary])
        .with_time(TimeSpec::from_date_value(DateValue::from_original(
            "1980",
            Provenance::default(),
        )))
        .with_participant(EventParticipant::new(person_id, "deceased")),
    );
    let life_id = builder.add_manual_event(
        ManualEventDraft::new(EventTypeId::new("genealogy.life"), "Sample person lived")
            .with_scale_kinds([EventScaleKind::Composite, EventScaleKind::Interval])
            .with_time(TimeSpec::Range {
                start: Some(DateValue::from_original("1901", Provenance::default())),
                end: Some(DateValue::from_original("1980", Provenance::default())),
            })
            .with_participant(EventParticipant::new(person_id, "subject")),
    );

    for relation in boundary_relations_for_composite(life_id, Some(birth_id), Some(death_id)) {
        builder.add_event_relation(relation);
    }

    builder.into_pack()
}

pub fn sample_history_pack() -> EventPack {
    let mut builder = EventPackBuilder::new(
        PackMetadata::new(PackId::new("sample:history"), "Sample history"),
        PackKind::HistoricalTimeline,
    );
    builder.add_manual_event(
        ManualEventDraft::new(
            EventTypeId::new("history.period"),
            "A sample historical period",
        )
        .with_scale_kinds([EventScaleKind::Composite, EventScaleKind::Interval])
        .with_time(TimeSpec::Range {
            start: Some(DateValue::from_original("1939", Provenance::default())),
            end: Some(DateValue::from_original("1945", Provenance::default())),
        }),
    );
    builder.add_manual_event(
        ManualEventDraft::new(
            EventTypeId::new("history.event"),
            "A sample event inside the period",
        )
        .with_time(TimeSpec::from_date_value(DateValue::from_original(
            "1942",
            Provenance::default(),
        ))),
    );
    builder.add_event_relation(crate::EventRelation::new(
        EventId(1),
        EventId(2),
        EventRelationKind::Contains,
    ));

    builder.into_pack()
}

pub fn sample_timeline_packs() -> Vec<EventPack> {
    vec![
        sample_journal_pack(),
        sample_biography_pack(),
        sample_history_pack(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EventRelationKind, TimelineDocument, YearSpan, validate_timeline_event};

    #[test]
    fn sample_journal_pack_contains_valid_journal_entry() {
        let pack = sample_journal_pack();
        let profiles = pack.domain_profiles.clone();

        assert_eq!(pack.kind, PackKind::UserJournal);
        assert_eq!(pack.events.len(), 1);
        assert!(validate_timeline_event(&pack.events[0], &profiles).is_empty());
    }

    #[test]
    fn sample_biography_pack_contains_life_boundaries() {
        let pack = sample_biography_pack();

        assert_eq!(pack.kind, PackKind::Biography);
        assert_eq!(pack.events.len(), 3);
        assert_eq!(pack.event_relations.len(), 2);
        assert!(
            pack.event_relations
                .iter()
                .any(|relation| relation.kind == EventRelationKind::Starts)
        );
        assert!(
            pack.event_relations
                .iter()
                .any(|relation| relation.kind == EventRelationKind::Ends)
        );
    }

    #[test]
    fn sample_packs_can_feed_timeline_document_queries() {
        let mut document = TimelineDocument::empty();
        for pack in sample_timeline_packs() {
            document.add_pack(pack, true);
        }

        assert_eq!(document.active_packs().count(), 3);
        assert!(
            !document
                .active_events_in_year_span(YearSpan::exact(1942))
                .is_empty()
        );
        assert!(!document.active_events_for_entity(PersonId(1)).is_empty());
    }
}
