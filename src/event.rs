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
pub enum EventScaleKind {
    /// A point-like event treated as one unit at the current modeling scale.
    Atomic,

    /// A higher-scale event composed from other events.
    Composite,

    /// A state/process/period with duration.
    Interval,

    /// A point-like event that starts or ends an interval/composite event.
    Boundary,
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
pub enum EventRelationKind {
    Starts,
    Ends,
    Contains,
    OccursWithin,
    EvidenceFor,
    Summarizes,
    ContextFor,
    SubEvent,
    Custom(String),
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
pub struct EventRelation {
    /// The higher-scale event, context event, or claim being supported.
    pub parent_event_id: EventId,

    /// The lower-scale event participating in the parent/context event.
    pub child_event_id: EventId,

    pub kind: EventRelationKind,
    pub note: Option<String>,
    pub provenance: Provenance,
}

impl EventRelation {
    pub fn new(parent_event_id: EventId, child_event_id: EventId, kind: EventRelationKind) -> Self {
        Self {
            parent_event_id,
            child_event_id,
            kind,
            note: None,
            provenance: Provenance::default(),
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
    pub scale_kinds: Vec<EventScaleKind>,
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
            scale_kinds: vec![EventScaleKind::Atomic],
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

    pub fn with_scale_kind(mut self, scale_kind: EventScaleKind) -> Self {
        if !self.scale_kinds.contains(&scale_kind) {
            self.scale_kinds.push(scale_kind);
        }
        self
    }

    pub fn with_scale_kinds(
        mut self,
        scale_kinds: impl IntoIterator<Item = EventScaleKind>,
    ) -> Self {
        self.scale_kinds.clear();
        for scale_kind in scale_kinds {
            if !self.scale_kinds.contains(&scale_kind) {
                self.scale_kinds.push(scale_kind);
            }
        }
        if self.scale_kinds.is_empty() {
            self.scale_kinds.push(EventScaleKind::Atomic);
        }
        self
    }

    pub fn is_scale_kind(&self, scale_kind: EventScaleKind) -> bool {
        self.scale_kinds.contains(&scale_kind)
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
        assert!(event.is_scale_kind(EventScaleKind::Atomic));
        assert_eq!(event.participants.len(), 1);
    }

    #[test]
    fn timeline_event_can_replace_scale_kinds() {
        let event = TimelineEvent::new(EventId(2), EventTypeId::new("genealogy.life"), "Life")
            .with_scale_kinds([EventScaleKind::Composite, EventScaleKind::Interval]);

        assert!(!event.is_scale_kind(EventScaleKind::Atomic));
        assert!(event.is_scale_kind(EventScaleKind::Composite));
        assert!(event.is_scale_kind(EventScaleKind::Interval));
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

    #[test]
    fn event_relation_models_graph_composition() {
        let relation = EventRelation::new(EventId(10), EventId(1), EventRelationKind::Starts);

        assert_eq!(relation.parent_event_id, EventId(10));
        assert_eq!(relation.child_event_id, EventId(1));
        assert_eq!(relation.kind, EventRelationKind::Starts);
    }
}
