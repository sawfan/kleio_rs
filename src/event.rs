//! Generic event records for timelines, journals, and domain projections.
//!
//! This module does not replace the existing genealogy `model::Event` yet. It
//! provides the domain-profile-aware event shape that new Kleio timeline work
//! can use while older GEDCOM/genealogy code continues to compile unchanged.

use rkyv::{Archive, Deserialize, Serialize};

use crate::attribution::{Provenance, SourceRef, Tag};
use crate::entity::EntityRef;
use crate::event_type::{EventTypeId, RoleId};
use crate::model::{DateValue, EventId, PlaceId};

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
pub enum Approximation {
    Circa,
    Estimated,
    Probably,
    Possibly,
    Inferred,
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
pub enum TimeSpec {
    Unknown,

    /// Transitional precision-aware date value backed by the existing Kleio
    /// genealogy date model.
    Date(DateValue),

    Range {
        start: Option<DateValue>,
        end: Option<DateValue>,
    },

    Approximate {
        value: DateValue,
        qualifier: Approximation,
    },

    Before(DateValue),
    After(DateValue),

    Between {
        start: DateValue,
        end: DateValue,
    },

    /// Preserve source text when it cannot yet be parsed safely.
    OriginalOnly {
        original: String,
    },
}

impl TimeSpec {
    pub fn from_date_value(date: DateValue) -> Self {
        Self::Date(date)
    }

    pub fn display(&self) -> String {
        match self {
            Self::Unknown => "unknown date".to_string(),
            Self::Date(date) => date.display(),
            Self::Range { start, end } => match (start, end) {
                (Some(start), Some(end)) => format!("{} to {}", start.display(), end.display()),
                (Some(start), None) => format!("from {}", start.display()),
                (None, Some(end)) => format!("until {}", end.display()),
                (None, None) => "unknown range".to_string(),
            },
            Self::Approximate { value, qualifier } => {
                format!("{} {}", qualifier.label(), value.display())
            }
            Self::Before(value) => format!("before {}", value.display()),
            Self::After(value) => format!("after {}", value.display()),
            Self::Between { start, end } => {
                format!("between {} and {}", start.display(), end.display())
            }
            Self::OriginalOnly { original } => original.clone(),
        }
    }
}

impl Approximation {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Circa => "circa",
            Self::Estimated => "estimated",
            Self::Probably => "probably",
            Self::Possibly => "possibly",
            Self::Inferred => "inferred",
        }
    }
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
pub struct EventParticipant {
    pub entity: EntityRef,
    pub role: RoleId,
    pub note: Option<String>,
}

impl EventParticipant {
    pub fn new(entity: impl Into<EntityRef>, role: impl Into<String>) -> Self {
        Self {
            entity: entity.into(),
            role: RoleId::new(role.into()),
            note: None,
        }
    }
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
pub struct TimelineEvent {
    pub id: EventId,
    pub title: String,
    pub type_ref: EventTypeId,
    pub time: TimeSpec,
    pub place: Option<PlaceId>,
    pub participants: Vec<EventParticipant>,
    pub description: Option<String>,
    pub sources: Vec<SourceRef>,
    pub tags: Vec<Tag>,
    pub provenance: Provenance,
}

impl TimelineEvent {
    pub fn new(id: EventId, type_ref: EventTypeId, title: impl Into<String>) -> Self {
        Self {
            id,
            title: title.into(),
            type_ref,
            time: TimeSpec::Unknown,
            place: None,
            participants: Vec::new(),
            description: None,
            sources: Vec::new(),
            tags: Vec::new(),
            provenance: Provenance::default(),
        }
    }

    pub fn with_time(mut self, time: TimeSpec) -> Self {
        self.time = time;
        self
    }

    pub fn with_participant(mut self, participant: EventParticipant) -> Self {
        self.participants.push(participant);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DateValue, EventTypeId, PersonId, Provenance};

    #[test]
    fn timeline_event_uses_domain_scoped_type_ref() {
        let event = TimelineEvent::new(
            EventId(1),
            EventTypeId::new("genealogy.birth"),
            "Birth of Ada",
        )
        .with_participant(EventParticipant::new(PersonId(42), "child"));

        assert_eq!(event.type_ref.as_str(), "genealogy.birth");
        assert_eq!(event.participants.len(), 1);
    }

    #[test]
    fn time_spec_preserves_approximate_display() {
        let date = DateValue::from_original("1850", Provenance::default());
        let time = TimeSpec::Approximate {
            value: date,
            qualifier: Approximation::Circa,
        };

        assert_eq!(time.display(), "circa 1850");
    }
}
