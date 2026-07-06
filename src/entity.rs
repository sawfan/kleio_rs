//! Generic entities used by chronology/event views.
//!
//! This module is a transitional layer toward a broader Kleio model. Existing
//! genealogy-oriented `Person`, `Place`, and `Family` records remain valid; the
//! generic refs here let newer event/timeline code refer to people, places, and
//! future non-person subjects through one participant interface.

use rkyv::{Archive, Deserialize, Serialize};

use crate::attribution::{Provenance, SourceRef};
use crate::model::{FamilyId, PersonId, PlaceId};

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
pub struct EntityId(pub String);

impl EntityId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

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
pub enum EntityRef {
    /// A future-native generic entity record.
    Entity(EntityId),

    /// Bridge to the existing genealogy person model.
    Person(PersonId),

    /// Bridge to the existing genealogy family model.
    Family(FamilyId),

    /// Bridge to the existing place model.
    Place(PlaceId),

    /// Bridge to a source/document/citation target.
    Source(SourceRef),

    /// Lossless project/import-specific reference while a first-class entity is
    /// not available yet.
    External(String),
}

impl From<EntityId> for EntityRef {
    fn from(value: EntityId) -> Self {
        Self::Entity(value)
    }
}

impl From<PersonId> for EntityRef {
    fn from(value: PersonId) -> Self {
        Self::Person(value)
    }
}

impl From<FamilyId> for EntityRef {
    fn from(value: FamilyId) -> Self {
        Self::Family(value)
    }
}

impl From<PlaceId> for EntityRef {
    fn from(value: PlaceId) -> Self {
        Self::Place(value)
    }
}

impl From<SourceRef> for EntityRef {
    fn from(value: SourceRef) -> Self {
        Self::Source(value)
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
pub enum EntityKind {
    Person,
    Family,
    Organization,
    Place,
    MilitaryUnit,
    Nation,
    Ship,
    Work,
    Artifact,
    Source,
    Topic,
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
pub struct Entity {
    pub id: EntityId,
    pub kind: EntityKind,
    pub name: String,
    pub aliases: Vec<String>,
    pub description: Option<String>,
    pub sources: Vec<SourceRef>,
    pub provenance: Provenance,
}

impl Entity {
    pub fn new(id: EntityId, kind: EntityKind, name: impl Into<String>) -> Self {
        Self {
            id,
            kind,
            name: name.into(),
            aliases: Vec::new(),
            description: None,
            sources: Vec::new(),
            provenance: Provenance::default(),
        }
    }
}
