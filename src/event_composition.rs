//! Helpers for composing, expanding, and collapsing scale-relative events.
//!
//! Event composition is graph-shaped: a smaller event can participate in many
//! higher-scale events. These helpers keep that interpretation outside storage
//! so UI and repository layers can choose how to render the same event graph.

use std::collections::BTreeSet;

use crate::event::{
    EventCompositionKind, EventRelation, EventRelationKind, EventTemporalKind, TimeSpec,
    TimelineEvent,
};
use crate::event_type::EventTypeId;
use crate::model::{DateValue, EventId};

pub fn event_by_id(events: &[TimelineEvent], event_id: EventId) -> Option<&TimelineEvent> {
    events.iter().find(|event| event.id == event_id)
}

pub fn child_relations(
    relations: &[EventRelation],
    parent_event_id: EventId,
) -> Vec<&EventRelation> {
    relations
        .iter()
        .filter(|relation| relation.parent_event_id == parent_event_id)
        .collect()
}

pub fn parent_relations(
    relations: &[EventRelation],
    child_event_id: EventId,
) -> Vec<&EventRelation> {
    relations
        .iter()
        .filter(|relation| relation.child_event_id == child_event_id)
        .collect()
}

pub fn child_events<'a>(
    events: &'a [TimelineEvent],
    relations: &[EventRelation],
    parent_event_id: EventId,
) -> Vec<&'a TimelineEvent> {
    child_relations(relations, parent_event_id)
        .into_iter()
        .filter_map(|relation| event_by_id(events, relation.child_event_id))
        .collect()
}

pub fn child_events_by_relation<'a>(
    events: &'a [TimelineEvent],
    relations: &[EventRelation],
    parent_event_id: EventId,
    kind: EventRelationKind,
) -> Vec<&'a TimelineEvent> {
    relations
        .iter()
        .filter(|relation| relation.parent_event_id == parent_event_id && relation.kind == kind)
        .filter_map(|relation| event_by_id(events, relation.child_event_id))
        .collect()
}

pub fn parent_events<'a>(
    events: &'a [TimelineEvent],
    relations: &[EventRelation],
    child_event_id: EventId,
) -> Vec<&'a TimelineEvent> {
    parent_relations(relations, child_event_id)
        .into_iter()
        .filter_map(|relation| event_by_id(events, relation.parent_event_id))
        .collect()
}

/// Return events suitable for a collapsed timeline view.
///
/// Collapsing a composite parent keeps the parent event visible and hides its
/// direct child events. This deliberately only hides direct children; UI code can
/// call this repeatedly or compute transitive closure if it wants deeper
/// collapsing behavior.
pub fn timeline_events_with_collapsed_parents<'a>(
    events: &'a [TimelineEvent],
    relations: &[EventRelation],
    collapsed_parent_ids: impl IntoIterator<Item = EventId>,
) -> Vec<&'a TimelineEvent> {
    let collapsed_parent_ids: BTreeSet<EventId> = collapsed_parent_ids.into_iter().collect();
    let hidden_child_ids: BTreeSet<EventId> = relations
        .iter()
        .filter(|relation| collapsed_parent_ids.contains(&relation.parent_event_id))
        .map(|relation| relation.child_event_id)
        .collect();

    events
        .iter()
        .filter(|event| !hidden_child_ids.contains(&event.id))
        .collect()
}

/// Build a composite interval event from optional boundary events.
///
/// This is intentionally domain-neutral. The caller decides whether the result
/// represents a person's life, a war, a research project, a residence period, or
/// some other macro event.
pub fn composite_interval_from_boundaries(
    id: EventId,
    type_ref: EventTypeId,
    title: impl Into<String>,
    start_boundary: Option<&TimelineEvent>,
    end_boundary: Option<&TimelineEvent>,
) -> TimelineEvent {
    let start = start_boundary.and_then(representative_date_value);
    let end = end_boundary.and_then(representative_date_value);

    TimelineEvent::new(id, type_ref, title)
        .with_composition_kind(EventCompositionKind::Composite)
        .with_temporal_kind(EventTemporalKind::Interval)
        .with_time(TimeSpec::Range { start, end })
}

/// Build graph relations between a composite interval and its known boundaries.
pub fn boundary_relations_for_composite(
    parent_event_id: EventId,
    start_boundary: Option<EventId>,
    end_boundary: Option<EventId>,
) -> Vec<EventRelation> {
    let mut relations = Vec::new();

    if let Some(start_boundary) = start_boundary {
        relations.push(EventRelation::new(
            parent_event_id,
            start_boundary,
            EventRelationKind::Starts,
        ));
    }

    if let Some(end_boundary) = end_boundary {
        relations.push(EventRelation::new(
            parent_event_id,
            end_boundary,
            EventRelationKind::Ends,
        ));
    }

    relations
}

fn representative_date_value(event: &TimelineEvent) -> Option<DateValue> {
    match &event.time {
        TimeSpec::Unknown | TimeSpec::OriginalOnly { .. } => None,
        TimeSpec::Date(date) | TimeSpec::Approximate { value: date, .. } => Some(date.clone()),
        TimeSpec::Range { start, end } => start.clone().or_else(|| end.clone()),
        TimeSpec::Before(date) | TimeSpec::After(date) => Some(date.clone()),
        TimeSpec::Between { start, .. } => Some(start.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DateValue, EventParticipant, PersonId, Provenance};

    #[test]
    fn composite_interval_uses_boundary_dates() {
        let birth = TimelineEvent::new(
            EventId(1),
            EventTypeId::new("genealogy.birth"),
            "Birth of Ada",
        )
        .with_time(TimeSpec::from_date_value(DateValue::from_original(
            "1815",
            Provenance::default(),
        )));
        let death = TimelineEvent::new(
            EventId(2),
            EventTypeId::new("genealogy.death"),
            "Death of Ada",
        )
        .with_time(TimeSpec::from_date_value(DateValue::from_original(
            "1852",
            Provenance::default(),
        )));

        let life = composite_interval_from_boundaries(
            EventId(100),
            EventTypeId::new("genealogy.life"),
            "Ada Lovelace lived",
            Some(&birth),
            Some(&death),
        );

        assert_eq!(life.composition, EventCompositionKind::Composite);
        assert_eq!(life.temporal, EventTemporalKind::Interval);
        assert_eq!(life.boundary, crate::EventBoundaryKind::None);
        assert!(life.is_composite());
        assert!(life.is_interval());
        assert_eq!(life.time.display(), "1815 to 1852");
    }

    #[test]
    fn collapsed_parent_hides_direct_children() {
        let life = TimelineEvent::new(EventId(100), EventTypeId::new("genealogy.life"), "Life")
            .with_composition_kind(EventCompositionKind::Composite)
            .with_temporal_kind(EventTemporalKind::Interval);
        let birth = TimelineEvent::new(EventId(1), EventTypeId::new("genealogy.birth"), "Birth")
            .with_participant(EventParticipant::new(PersonId(7), "child"));
        let death = TimelineEvent::new(EventId(2), EventTypeId::new("genealogy.death"), "Death");
        let unrelated = TimelineEvent::new(EventId(3), EventTypeId::new("journal.entry"), "Note");
        let events = vec![life, birth, death, unrelated];
        let relations = vec![
            EventRelation::new(EventId(100), EventId(1), EventRelationKind::Starts),
            EventRelation::new(EventId(100), EventId(2), EventRelationKind::Ends),
        ];

        let visible = timeline_events_with_collapsed_parents(&events, &relations, [EventId(100)]);
        let visible_ids: Vec<EventId> = visible.into_iter().map(|event| event.id).collect();

        assert_eq!(visible_ids, vec![EventId(100), EventId(3)]);
    }

    #[test]
    fn child_event_lookup_follows_graph_relations() {
        let parent = TimelineEvent::new(EventId(10), EventTypeId::new("history.war"), "War");
        let child = TimelineEvent::new(EventId(11), EventTypeId::new("history.battle"), "Battle");
        let events = vec![parent, child];
        let relations = vec![EventRelation::new(
            EventId(10),
            EventId(11),
            EventRelationKind::Contains,
        )];

        let children = child_events(&events, &relations, EventId(10));

        assert_eq!(children.len(), 1);
        assert_eq!(children[0].id, EventId(11));
    }
}
