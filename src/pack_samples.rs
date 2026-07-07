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
    let person_id = PersonId(42);
    let mut builder = EventPackBuilder::new(
        PackMetadata::new(PackId::new("sample:life-stages"), "Sample life stages"),
        PackKind::Biography,
    );
    builder.add_domain_profile(genealogy_domain_profile());

    let birth_id = builder.add_manual_event(
        ManualEventDraft::new(EventTypeId::new("genealogy.birth"), "Alex Morgan is born")
            .with_boundary_kind(EventBoundaryKind::Start)
            .with_time(TimeSpec::from_date_value(DateValue::from_original(
                "1990",
                Provenance::default(),
            )))
            .with_participant(EventParticipant::new(person_id, "child")),
    );
    let start_elementary_id = builder.add_manual_event(
        boundary_draft(
            "education.started_elementary_school",
            "Starts elementary school",
            "1996",
            person_id,
        )
        .with_description("Alex starts elementary school."),
    );
    let finish_elementary_id = builder.add_manual_event(
        boundary_draft(
            "education.finished_elementary_school",
            "Finishes elementary school",
            "2002",
            person_id,
        )
        .with_description("Alex finishes elementary school."),
    );
    let elementary_id = builder.add_manual_event(
        period_draft(
            "education.elementary_school",
            "Elementary school",
            "1996",
            "2002",
            person_id,
        )
        .with_description("Alex attends elementary school."),
    );
    let start_high_school_id = builder.add_manual_event(
        boundary_draft(
            "education.started_high_school",
            "Starts high school",
            "2004",
            person_id,
        )
        .with_description("Alex starts high school."),
    );
    let finish_high_school_id = builder.add_manual_event(
        boundary_draft(
            "education.finished_high_school",
            "Graduates high school",
            "2008",
            person_id,
        )
        .with_description("Alex graduates high school."),
    );
    let high_school_id = builder.add_manual_event(
        period_draft(
            "education.high_school",
            "High school",
            "2004",
            "2008",
            person_id,
        )
        .with_description("Alex attends high school."),
    );
    let start_college_id = builder.add_manual_event(
        boundary_draft(
            "education.started_college",
            "Starts college",
            "2008",
            person_id,
        )
        .with_description("Alex starts college."),
    );
    let finish_college_id = builder.add_manual_event(
        boundary_draft(
            "education.finished_college",
            "Graduates college",
            "2012",
            person_id,
        )
        .with_description("Alex graduates college."),
    );
    let college_id = builder.add_manual_event(
        period_draft("education.college", "College", "2008", "2012", person_id)
            .with_description("Alex attends college."),
    );
    let start_job_id = builder.add_manual_event(
        boundary_draft("career.started_job", "Starts first job", "2013", person_id)
            .with_description("Alex starts a long-term career position."),
    );
    let job_id = builder.add_manual_event(
        period_draft(
            "career.job",
            "First long-term job",
            "2013",
            "2055",
            person_id,
        )
        .with_description("Alex starts and maintains a long-term career position."),
    );
    let marriage_start_id = builder.add_manual_event(
        boundary_draft("genealogy.marriage", "Gets married", "2017", person_id)
            .with_description("Alex gets married."),
    );
    let marriage_id = builder.add_manual_event(
        period_draft("genealogy.marriage", "Marriage", "2017", "2068", person_id)
            .with_description("Alex's marriage period."),
    );
    let death_id = builder.add_manual_event(
        ManualEventDraft::new(EventTypeId::new("genealogy.death"), "Alex Morgan dies")
            .with_boundary_kind(EventBoundaryKind::End)
            .with_time(TimeSpec::from_date_value(DateValue::from_original(
                "2068",
                Provenance::default(),
            )))
            .with_participant(EventParticipant::new(person_id, "deceased")),
    );
    let life_id = builder.add_manual_event(
        ManualEventDraft::new(EventTypeId::new("genealogy.life"), "Alex Morgan lived")
            .with_composition_kind(EventCompositionKind::Composite)
            .with_temporal_kind(EventTemporalKind::Interval)
            .with_time(TimeSpec::Range {
                start: Some(DateValue::from_original("1990", Provenance::default())),
                end: Some(DateValue::from_original("2068", Provenance::default())),
            })
            .with_participant(EventParticipant::new(person_id, "subject")),
    );

    for relation in boundary_relations_for_composite(life_id, Some(birth_id), Some(death_id)) {
        builder.add_event_relation(relation);
    }
    for (parent_id, start_id, end_id) in [
        (
            elementary_id,
            Some(start_elementary_id),
            Some(finish_elementary_id),
        ),
        (
            high_school_id,
            Some(start_high_school_id),
            Some(finish_high_school_id),
        ),
        (college_id, Some(start_college_id), Some(finish_college_id)),
        (job_id, Some(start_job_id), None),
        (marriage_id, Some(marriage_start_id), Some(death_id)),
    ] {
        for relation in boundary_relations_for_composite(parent_id, start_id, end_id) {
            builder.add_event_relation(relation);
        }
    }

    for child_id in [
        elementary_id,
        high_school_id,
        college_id,
        job_id,
        marriage_id,
    ] {
        builder.add_event_relation(crate::EventRelation::new(
            life_id,
            child_id,
            EventRelationKind::OccursWithin,
        ));
    }

    builder.into_pack()
}

pub fn sample_timeline_packs() -> Vec<EventPack> {
    vec![
        sample_journal_pack(),
        sample_biography_pack(),
        sample_history_pack(),
        sample_life_stages_pack(),
    ]
}

fn boundary_draft(
    event_type: &str,
    title: &str,
    date: &str,
    person_id: PersonId,
) -> ManualEventDraft {
    ManualEventDraft::new(EventTypeId::new(event_type), title)
        .with_boundary_kind(EventBoundaryKind::StartAndEnd)
        .with_time(TimeSpec::from_date_value(DateValue::from_original(
            date,
            Provenance::default(),
        )))
        .with_participant(EventParticipant::new(person_id, "subject"))
}

fn period_draft(
    event_type: &str,
    title: &str,
    start: &str,
    end: &str,
    person_id: PersonId,
) -> ManualEventDraft {
    ManualEventDraft::new(EventTypeId::new(event_type), title)
        .with_composition_kind(EventCompositionKind::Composite)
        .with_temporal_kind(EventTemporalKind::Interval)
        .with_time(TimeSpec::Range {
            start: Some(DateValue::from_original(start, Provenance::default())),
            end: Some(DateValue::from_original(end, Provenance::default())),
        })
        .with_participant(EventParticipant::new(person_id, "subject"))
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
