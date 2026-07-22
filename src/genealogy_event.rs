//! Effective genealogy-domain event model.
//!
//! New core event work should use [`crate::Event`] / [`crate::TimelineEvent`].
//! This module contains the compact genealogy projection retained for imported
//! family-tree archives and tree documents.

use rkyv::{Archive, Deserialize, Serialize};

use crate::attribution::Provenance;
use crate::model::{DateValue, EventId, PersonId, PlaceId};

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
pub enum GenealogyEventKind {
    Birth,
    Death,
    Marriage,
    Baptism,
    Burial,
    Residence,
    Occupation,

    /// Fallback for source-specific genealogy/import kinds.
    Other(String),
}

/// Genealogy-domain projection of a canonical [`crate::TimelineEvent`].
///
/// Kleio's event primitive is [`crate::TimelineEvent`]. This type remains in the
/// genealogy archive model so imported family-tree data can keep compact,
/// genealogy-specific indexes while adapters project it into canonical events.
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
pub struct GenealogyEvent {
    pub id: EventId,
    pub kind: GenealogyEventKind,

    pub date: Option<DateValue>,

    /// Source-specific time string (may be local time).
    pub time: Option<String>,

    /// Source-specific time zone string.
    pub time_zone: Option<String>,

    pub place: Option<PlaceId>,

    pub description: Option<String>,

    pub participants: Vec<PersonId>,

    pub provenance: Provenance,
}
