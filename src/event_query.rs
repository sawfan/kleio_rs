//! Query helpers for generic timeline events.
//!
//! These helpers intentionally operate on slices/iterators so they can be used
//! by browser UI code, archive projections, and future SQLite-backed
//! repositories without committing to a storage backend.

use crate::entity::EntityRef;
use crate::event::{TimeSpec, TimelineEvent};
use crate::event_type::EventTypeId;
use crate::model::{DateRange, DateValue};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct YearSpan {
    pub earliest_year: Option<i32>,
    pub latest_year: Option<i32>,
}

impl YearSpan {
    pub fn new(earliest_year: Option<i32>, latest_year: Option<i32>) -> Self {
        Self {
            earliest_year,
            latest_year,
        }
    }

    pub fn exact(year: i32) -> Self {
        Self::new(Some(year), Some(year))
    }

    pub fn overlaps(self, other: Self) -> bool {
        let self_start = self.earliest_year.unwrap_or(i32::MIN);
        let self_end = self.latest_year.unwrap_or(i32::MAX);
        let other_start = other.earliest_year.unwrap_or(i32::MIN);
        let other_end = other.latest_year.unwrap_or(i32::MAX);

        self_start <= other_end && other_start <= self_end
    }
}

impl From<&DateRange> for YearSpan {
    fn from(value: &DateRange) -> Self {
        Self::new(value.earliest_year, value.latest_year)
    }
}

pub fn date_value_year_span(date: &DateValue) -> Option<YearSpan> {
    date.range.as_ref().map(YearSpan::from)
}

pub fn time_spec_year_span(time: &TimeSpec) -> Option<YearSpan> {
    match time {
        TimeSpec::Unknown | TimeSpec::OriginalOnly { .. } => None,
        TimeSpec::Date(date) | TimeSpec::Approximate { value: date, .. } => {
            date_value_year_span(date)
        }
        TimeSpec::Range { start, end } => range_year_span(start.as_ref(), end.as_ref()),
        TimeSpec::Before(date) => date_value_year_span(date)
            .map(|span| YearSpan::new(None, span.latest_year.or(span.earliest_year))),
        TimeSpec::After(date) => date_value_year_span(date)
            .map(|span| YearSpan::new(span.earliest_year.or(span.latest_year), None)),
        TimeSpec::Between { start, end } => range_year_span(Some(start), Some(end)),
    }
}

pub fn timeline_event_year_span(event: &TimelineEvent) -> Option<YearSpan> {
    time_spec_year_span(&event.time)
}

pub fn timeline_event_has_entity(event: &TimelineEvent, entity: &EntityRef) -> bool {
    event
        .participants
        .iter()
        .any(|participant| &participant.entity == entity)
}

pub fn timeline_event_has_type(event: &TimelineEvent, event_type: &EventTypeId) -> bool {
    &event.type_ref == event_type
}

pub fn timeline_event_overlaps_year_span(event: &TimelineEvent, span: YearSpan) -> bool {
    timeline_event_year_span(event).is_some_and(|event_span| event_span.overlaps(span))
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TimelineEventFilter {
    pub entity: Option<EntityRef>,
    pub event_type: Option<EventTypeId>,
    pub years: Option<YearSpan>,
    pub include_undated: bool,
}

impl TimelineEventFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn for_entity(entity: impl Into<EntityRef>) -> Self {
        Self {
            entity: Some(entity.into()),
            ..Self::default()
        }
    }

    pub fn with_event_type(mut self, event_type: EventTypeId) -> Self {
        self.event_type = Some(event_type);
        self
    }

    pub fn with_year_span(mut self, years: YearSpan) -> Self {
        self.years = Some(years);
        self
    }

    pub fn include_undated(mut self, include_undated: bool) -> Self {
        self.include_undated = include_undated;
        self
    }

    pub fn matches(&self, event: &TimelineEvent) -> bool {
        if let Some(entity) = self.entity.as_ref()
            && !timeline_event_has_entity(event, entity)
        {
            return false;
        }

        if let Some(event_type) = self.event_type.as_ref()
            && !timeline_event_has_type(event, event_type)
        {
            return false;
        }

        if let Some(years) = self.years {
            match timeline_event_year_span(event) {
                Some(event_years) if event_years.overlaps(years) => {}
                None if self.include_undated => {}
                _ => return false,
            }
        }

        true
    }
}

pub fn filter_timeline_events<'a>(
    events: impl IntoIterator<Item = &'a TimelineEvent>,
    filter: &TimelineEventFilter,
) -> Vec<&'a TimelineEvent> {
    events
        .into_iter()
        .filter(|event| filter.matches(event))
        .collect()
}

fn range_year_span(start: Option<&DateValue>, end: Option<&DateValue>) -> Option<YearSpan> {
    let start_span = start.and_then(date_value_year_span);
    let end_span = end.and_then(date_value_year_span);

    match (start_span, end_span) {
        (Some(start), Some(end)) => Some(YearSpan::new(
            start.earliest_year.or(start.latest_year),
            end.latest_year.or(end.earliest_year),
        )),
        (Some(start), None) => Some(YearSpan::new(
            start.earliest_year.or(start.latest_year),
            None,
        )),
        (None, Some(end)) => Some(YearSpan::new(None, end.latest_year.or(end.earliest_year))),
        (None, None) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        EventId, EventParticipant, EventTypeId, PersonId, Provenance, TimeSpec, TimelineEvent,
    };

    #[test]
    fn year_spans_overlap_open_and_closed_ranges() {
        assert!(YearSpan::exact(1944).overlaps(YearSpan::new(Some(1939), Some(1945))));
        assert!(!YearSpan::exact(1950).overlaps(YearSpan::new(Some(1939), Some(1945))));
        assert!(YearSpan::new(None, Some(1900)).overlaps(YearSpan::exact(1850)));
        assert!(YearSpan::new(Some(1900), None).overlaps(YearSpan::exact(2026)));
    }

    #[test]
    fn time_spec_extracts_year_span_from_approximate_date() {
        let date = DateValue::from_original("1850", Provenance::default());
        let time = TimeSpec::Approximate {
            value: date,
            qualifier: crate::Approximation::Circa,
        };

        assert_eq!(time_spec_year_span(&time), Some(YearSpan::exact(1850)));
    }

    #[test]
    fn filter_matches_entity_type_and_year() {
        let birth = TimelineEvent::new(
            EventId(1),
            EventTypeId::new("genealogy.birth"),
            "Birth of Ada",
        )
        .with_time(TimeSpec::from_date_value(DateValue::from_original(
            "1815",
            Provenance::default(),
        )))
        .with_participant(EventParticipant::new(PersonId(7), "child"));
        let death = TimelineEvent::new(
            EventId(2),
            EventTypeId::new("genealogy.death"),
            "Death of Ada",
        )
        .with_time(TimeSpec::from_date_value(DateValue::from_original(
            "1852",
            Provenance::default(),
        )))
        .with_participant(EventParticipant::new(PersonId(7), "deceased"));
        let events = vec![birth, death];

        let filter = TimelineEventFilter::for_entity(PersonId(7))
            .with_event_type(EventTypeId::new("genealogy.birth"))
            .with_year_span(YearSpan::new(Some(1800), Some(1820)));
        let matches = filter_timeline_events(&events, &filter);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].id, EventId(1));
    }
}
