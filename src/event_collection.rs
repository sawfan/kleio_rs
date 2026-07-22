//! Curated collections and ordered sequences of timeline events.
//!
//! `TimelineEvent` remains the record for something that happened. An
//! `EventCollection` is the separate curatorial layer that says why a set of
//! events belongs together: a chronology, a research set, a comparison group, or
//! another reusable grouping. Ordered sequences are modeled as a collection kind
//! instead of as a distinct event type so non-temporal comparison sets do not
//! need to pretend to be events.

use std::collections::{BTreeMap, BTreeSet};

use rkyv::{Archive, Deserialize, Serialize};

use crate::attribution::{Provenance, Tag};
use crate::event::TimelineEvent;
use crate::event_query::timeline_event_year_span;
use crate::model::EventId;

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
#[rkyv(derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash))]
pub struct EventCollectionId(pub String);

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
pub enum EventCollectionKind {
    /// A curated group where order is not part of the collection's meaning.
    Set,

    /// A curated group where member order is meaningful.
    Sequence(EventSequenceOrder),
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Archive,
    Serialize,
    Deserialize,
    serde::Serialize,
    serde::Deserialize,
)]
pub enum EventSequenceOrder {
    /// Sort by the events' own time values when possible.
    Chronological,

    /// Preserve explicit member ordinals and insertion order.
    Manual,

    /// Use explicit member ordinals first, then fall back to event time.
    ManualThenChronological,
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
pub struct EventCollectionMember {
    pub event_id: EventId,

    /// Optional label used only within this collection.
    pub label: Option<String>,

    /// Collection-local role such as "reference", "candidate", "trigger", or
    /// another domain/application-specific role. This deliberately remains a
    /// plain string so Kleio does not need to know every consumer vocabulary.
    pub role: Option<String>,

    /// Explicit order for manually ordered sequences. Equal or missing ordinals
    /// preserve insertion order unless a sequence order says to use event time as
    /// a fallback.
    pub ordinal: Option<i32>,

    pub note: Option<String>,
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
pub struct EventCollection {
    pub id: EventCollectionId,
    pub title: String,
    pub description: Option<String>,
    pub kind: EventCollectionKind,
    pub members: Vec<EventCollectionMember>,
    pub tags: Vec<Tag>,
    pub provenance: Provenance,
}

impl EventCollectionId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl Default for EventCollectionKind {
    fn default() -> Self {
        Self::Set
    }
}

impl EventCollectionKind {
    pub fn is_sequence(&self) -> bool {
        matches!(self, Self::Sequence(_))
    }

    pub fn sequence_order(&self) -> Option<EventSequenceOrder> {
        match self {
            Self::Set => None,
            Self::Sequence(order) => Some(*order),
        }
    }
}

impl EventSequenceOrder {
    pub fn label(self) -> &'static str {
        match self {
            Self::Chronological => "Chronological",
            Self::Manual => "Manual",
            Self::ManualThenChronological => "Manual, then chronological",
        }
    }
}

impl EventCollectionMember {
    pub fn new(event_id: EventId) -> Self {
        Self {
            event_id,
            label: None,
            role: None,
            ordinal: None,
            note: None,
        }
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn with_role(mut self, role: impl Into<String>) -> Self {
        self.role = Some(role.into());
        self
    }

    pub fn with_ordinal(mut self, ordinal: i32) -> Self {
        self.ordinal = Some(ordinal);
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }
}

impl EventCollection {
    pub fn new(id: EventCollectionId, title: impl Into<String>, kind: EventCollectionKind) -> Self {
        Self {
            id,
            title: title.into(),
            description: None,
            kind,
            members: Vec::new(),
            tags: Vec::new(),
            provenance: Provenance::default(),
        }
    }

    pub fn set(id: EventCollectionId, title: impl Into<String>) -> Self {
        Self::new(id, title, EventCollectionKind::Set)
    }

    pub fn sequence(
        id: EventCollectionId,
        title: impl Into<String>,
        order: EventSequenceOrder,
    ) -> Self {
        Self::new(id, title, EventCollectionKind::Sequence(order))
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_member(mut self, member: EventCollectionMember) -> Self {
        self.members.push(member);
        self
    }

    pub fn push_member(&mut self, member: EventCollectionMember) {
        self.members.push(member);
    }

    pub fn member_event_ids(&self) -> Vec<EventId> {
        self.members.iter().map(|member| member.event_id).collect()
    }

    pub fn contains_event(&self, event_id: EventId) -> bool {
        self.members
            .iter()
            .any(|member| member.event_id == event_id)
    }

    pub fn member_label<'a>(&'a self, member: &'a EventCollectionMember) -> Option<&'a str> {
        member
            .label
            .as_deref()
            .filter(|label| !label.trim().is_empty())
    }

    pub fn events<'a>(&self, events: &'a [TimelineEvent]) -> Vec<&'a TimelineEvent> {
        collection_events(self, events)
    }

    pub fn ordered_events<'a>(&self, events: &'a [TimelineEvent]) -> Vec<&'a TimelineEvent> {
        ordered_collection_events(self, events)
    }
}

pub fn collection_events<'a>(
    collection: &EventCollection,
    events: &'a [TimelineEvent],
) -> Vec<&'a TimelineEvent> {
    let event_by_id: BTreeMap<EventId, &TimelineEvent> =
        events.iter().map(|event| (event.id, event)).collect();

    collection
        .members
        .iter()
        .filter_map(|member| event_by_id.get(&member.event_id).copied())
        .collect()
}

pub fn ordered_collection_members<'a>(
    collection: &'a EventCollection,
    events: &[TimelineEvent],
) -> Vec<&'a EventCollectionMember> {
    let mut indexed_members: Vec<(usize, &EventCollectionMember)> =
        collection.members.iter().enumerate().collect();

    match collection.kind {
        EventCollectionKind::Set => {}
        EventCollectionKind::Sequence(EventSequenceOrder::Manual) => {
            indexed_members.sort_by(|(left_index, left), (right_index, right)| {
                left.ordinal
                    .cmp(&right.ordinal)
                    .then_with(|| left_index.cmp(right_index))
            });
        }
        EventCollectionKind::Sequence(EventSequenceOrder::Chronological) => {
            let event_by_id: BTreeMap<EventId, &TimelineEvent> =
                events.iter().map(|event| (event.id, event)).collect();
            indexed_members.sort_by(|(left_index, left), (right_index, right)| {
                chronological_member_key(left, &event_by_id)
                    .cmp(&chronological_member_key(right, &event_by_id))
                    .then_with(|| left_index.cmp(right_index))
            });
        }
        EventCollectionKind::Sequence(EventSequenceOrder::ManualThenChronological) => {
            let event_by_id: BTreeMap<EventId, &TimelineEvent> =
                events.iter().map(|event| (event.id, event)).collect();
            indexed_members.sort_by(|(left_index, left), (right_index, right)| {
                left.ordinal
                    .cmp(&right.ordinal)
                    .then_with(|| {
                        chronological_member_key(left, &event_by_id)
                            .cmp(&chronological_member_key(right, &event_by_id))
                    })
                    .then_with(|| left_index.cmp(right_index))
            });
        }
    }

    indexed_members
        .into_iter()
        .map(|(_, member)| member)
        .collect()
}

pub fn ordered_collection_events<'a>(
    collection: &EventCollection,
    events: &'a [TimelineEvent],
) -> Vec<&'a TimelineEvent> {
    let event_by_id: BTreeMap<EventId, &TimelineEvent> =
        events.iter().map(|event| (event.id, event)).collect();

    ordered_collection_members(collection, events)
        .into_iter()
        .filter_map(|member| event_by_id.get(&member.event_id).copied())
        .collect()
}

pub fn collection_missing_event_ids(
    collection: &EventCollection,
    events: &[TimelineEvent],
) -> Vec<EventId> {
    let known_ids: BTreeSet<EventId> = events.iter().map(|event| event.id).collect();
    collection
        .members
        .iter()
        .filter(|member| !known_ids.contains(&member.event_id))
        .map(|member| member.event_id)
        .collect()
}

fn chronological_member_key(
    member: &EventCollectionMember,
    event_by_id: &BTreeMap<EventId, &TimelineEvent>,
) -> (bool, Option<i32>, Option<i32>, String) {
    let Some(event) = event_by_id.get(&member.event_id) else {
        return (true, None, None, String::new());
    };
    let year_span = timeline_event_year_span(event);
    (
        year_span.is_none(),
        year_span.and_then(|span| span.earliest_year.or(span.latest_year)),
        year_span.and_then(|span| span.latest_year.or(span.earliest_year)),
        event.title.clone(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DateValue, EventTypeId, TimeSpec};

    fn event(id: u64, title: &str, year: Option<i32>) -> TimelineEvent {
        let mut event = TimelineEvent::new(EventId(id), EventTypeId::new("test.event"), title);
        if let Some(year) = year {
            event = event.with_time(TimeSpec::from_date_value(DateValue::from_original(
                year.to_string(),
                Provenance::default(),
            )));
        }
        event
    }

    #[test]
    fn set_preserves_member_order() {
        let collection = EventCollection::set(EventCollectionId::new("comparison"), "Comparison")
            .with_member(EventCollectionMember::new(EventId(2)))
            .with_member(EventCollectionMember::new(EventId(1)));
        let events = vec![
            event(1, "First", Some(1900)),
            event(2, "Second", Some(1800)),
        ];

        let ordered_ids: Vec<EventId> = collection
            .ordered_events(&events)
            .into_iter()
            .map(|event| event.id)
            .collect();

        assert_eq!(ordered_ids, vec![EventId(2), EventId(1)]);
    }

    #[test]
    fn chronological_sequence_sorts_by_event_time() {
        let collection = EventCollection::sequence(
            EventCollectionId::new("life"),
            "Life",
            EventSequenceOrder::Chronological,
        )
        .with_member(EventCollectionMember::new(EventId(2)))
        .with_member(EventCollectionMember::new(EventId(1)))
        .with_member(EventCollectionMember::new(EventId(3)));
        let events = vec![
            event(1, "1900", Some(1900)),
            event(2, "1850", Some(1850)),
            event(3, "Unknown", None),
        ];

        let ordered_ids: Vec<EventId> = collection
            .ordered_events(&events)
            .into_iter()
            .map(|event| event.id)
            .collect();

        assert_eq!(ordered_ids, vec![EventId(2), EventId(1), EventId(3)]);
    }

    #[test]
    fn manual_sequence_sorts_by_ordinal_then_insertion_order() {
        let collection = EventCollection::sequence(
            EventCollectionId::new("manual"),
            "Manual",
            EventSequenceOrder::Manual,
        )
        .with_member(EventCollectionMember::new(EventId(1)).with_ordinal(20))
        .with_member(EventCollectionMember::new(EventId(2)).with_ordinal(10))
        .with_member(EventCollectionMember::new(EventId(3)).with_ordinal(10))
        .with_member(EventCollectionMember::new(EventId(4)));
        let events = vec![
            event(1, "One", Some(1900)),
            event(2, "Two", Some(1950)),
            event(3, "Three", Some(1800)),
            event(4, "Four", Some(1700)),
        ];

        let ordered_ids: Vec<EventId> = collection
            .ordered_events(&events)
            .into_iter()
            .map(|event| event.id)
            .collect();

        assert_eq!(
            ordered_ids,
            vec![EventId(4), EventId(2), EventId(3), EventId(1)]
        );
    }

    #[test]
    fn missing_event_ids_report_unresolved_members() {
        let collection = EventCollection::set(EventCollectionId::new("mixed"), "Mixed")
            .with_member(EventCollectionMember::new(EventId(1)))
            .with_member(EventCollectionMember::new(EventId(99)));
        let events = vec![event(1, "Known", Some(1900))];

        assert_eq!(
            collection_missing_event_ids(&collection, &events),
            vec![EventId(99)]
        );
    }
}
