//! Small sample packs for tests, demos, and UI scaffolding.
//!
//! These packs are intentionally tiny. They demonstrate the event-pack,
//! builder, validation, and composition primitives without depending on GEDCOM,
//! SQLite, or browser storage.

use crate::attribution::Provenance;
use crate::event::{
    EventBoundaryKind, EventCompositionKind, EventParticipant, EventRelationKind,
    EventTemporalKind, TimeSpec,
};
use crate::event_composition::boundary_relations_for_composite;
use crate::event_type::{EventTypeId, genealogy_domain_profile, journal_domain_profile};
use crate::model::{DateValue, EventId, PersonId};
use crate::pack::{EventPack, PackId, PackKind, PackMetadata};
use crate::pack_builder::{EventPackBuilder, ManualEventDraft};
use crate::timeline_source::{
    TimelineSource, TimelineSourceItem, TimelineSourceMeta, TimelineSourcePackKind,
    event_pack_from_timeline_source,
};

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
        .with_boundary_kind(EventBoundaryKind::Start)
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
        .with_boundary_kind(EventBoundaryKind::End)
        .with_time(TimeSpec::from_date_value(DateValue::from_original(
            "1980",
            Provenance::default(),
        )))
        .with_participant(EventParticipant::new(person_id, "deceased")),
    );
    let life_id = builder.add_manual_event(
        ManualEventDraft::new(EventTypeId::new("genealogy.life"), "Sample person lived")
            .with_composition_kind(EventCompositionKind::Composite)
            .with_temporal_kind(EventTemporalKind::Interval)
            .with_time(TimeSpec::Range {
                start: Some(DateValue::from_original("1901", Provenance::default())),
                end: Some(DateValue::from_original("1980", Provenance::default())),
            })
            .with_participant(EventParticipant::new(person_id, "subject"))
            .with_description("Template life interval: birth year/date to present. Add a death event/end boundary only when modeling a deceased person."),
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
        .with_composition_kind(EventCompositionKind::Composite)
        .with_temporal_kind(EventTemporalKind::Interval)
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

pub fn sample_life_stages_pack() -> EventPack {
    event_pack_from_timeline_source(&sample_life_stages_source())
}

pub fn sample_life_stages_source() -> TimelineSource {
    TimelineSource {
        meta: TimelineSourceMeta {
            id: Some("sample:life-stages".to_string()),
            title: "Life timeline template".to_string(),
            kind: Some(TimelineSourcePackKind::Biography),
            description: Some(
                "A starter personal timeline source. Replace `Your Name`, dates, places, and details with your own life events."
                    .to_string(),
            ),
            subject: Some("Your Name".to_string()),
            person_id: Some(42),
        },
        items: vec![
            TimelineSourceItem {
                section: "birth".to_string(),
                title: "Your Name is born".to_string(),
                start: "1990".to_string(),
                end: None,
                current: false,
                place: Some("Replace with birthplace".to_string()),
                place_lat: None,
                place_lon: None,
                place_timezone: None,
                place_geoname_id: None,
                details: Some("Replace 1990 with your birth year/date and add any notes you want to keep.".to_string()),
                start_label: None,
                end_label: None,
            },
            TimelineSourceItem {
                section: "education".to_string(),
                title: "Elementary or primary school".to_string(),
                start: "1996".to_string(),
                end: Some("2002".to_string()),
                current: false,
                place: Some("Replace with school/place".to_string()),
                place_lat: None,
                place_lon: None,
                place_timezone: None,
                place_geoname_id: None,
                details: Some("Replace with your school name, place, and notes for this period.".to_string()),
                start_label: Some("Starts elementary school".to_string()),
                end_label: Some("Finishes elementary school".to_string()),
            },
            TimelineSourceItem {
                section: "education".to_string(),
                title: "High school".to_string(),
                start: "2004".to_string(),
                end: Some("2008".to_string()),
                current: false,
                place: Some("Replace with high school/place".to_string()),
                place_lat: None,
                place_lon: None,
                place_timezone: None,
                place_geoname_id: None,
                details: Some("Replace with high school name, place, and notes for this period.".to_string()),
                start_label: Some("Starts high school".to_string()),
                end_label: Some("Graduates high school".to_string()),
            },
            TimelineSourceItem {
                section: "education".to_string(),
                title: "College, training, or apprenticeship".to_string(),
                start: "2008".to_string(),
                end: Some("2012".to_string()),
                current: false,
                place: Some("Replace with program/place".to_string()),
                place_lat: None,
                place_lon: None,
                place_timezone: None,
                place_geoname_id: None,
                details: Some("Replace with education, training, or apprenticeship details.".to_string()),
                start_label: Some("Starts college/training".to_string()),
                end_label: Some("Completes college/training".to_string()),
            },
            TimelineSourceItem {
                section: "career".to_string(),
                title: "Current or long-term work".to_string(),
                start: "2013".to_string(),
                end: None,
                current: true,
                place: Some("Replace with workplace/place".to_string()),
                place_lat: None,
                place_lon: None,
                place_timezone: None,
                place_geoname_id: None,
                details: Some("Replace with your role, employer, vocation, project, or career milestone. Leave current = true if ongoing.".to_string()),
                start_label: Some("Starts current work".to_string()),
                end_label: None,
            },
            TimelineSourceItem {
                section: "relationship".to_string(),
                title: "Relationship period".to_string(),
                start: "2017".to_string(),
                end: None,
                current: true,
                place: Some("Replace with place, or remove".to_string()),
                place_lat: None,
                place_lon: None,
                place_timezone: None,
                place_geoname_id: None,
                details: Some("Replace with a relationship period, marriage, partnership, or delete this item if not relevant.".to_string()),
                start_label: Some("Relationship milestone".to_string()),
                end_label: None,
            },
            TimelineSourceItem {
                section: "life".to_string(),
                title: "Your Name lived".to_string(),
                start: "1990".to_string(),
                end: None,
                current: true,
                place: None,
                place_lat: None,
                place_lon: None,
                place_timezone: None,
                place_geoname_id: None,
                details: Some("Template life interval: birth year/date to present. Add an end date only when modeling a deceased person.".to_string()),
                start_label: Some("Your Name is born".to_string()),
                end_label: None,
            },
        ],
    }
}

pub fn sample_timeline_packs() -> Vec<EventPack> {
    vec![
        sample_journal_pack(),
        sample_biography_pack(),
        sample_history_pack(),
        sample_life_stages_pack(),
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

        assert_eq!(document.active_packs().count(), 4);
        assert!(
            !document
                .active_events_in_year_span(YearSpan::exact(1942))
                .is_empty()
        );
        assert!(!document.active_events_for_entity(PersonId(1)).is_empty());
        assert!(!document.active_events_for_entity(PersonId(42)).is_empty());
    }
}
