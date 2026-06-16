//! Generic attribution / provenance hooks.
//!
//! The core model should stay source-agnostic, while still preserving
//! importer-specific fields and supporting future exchange formats such as
//! GEDCOM 7.
//!
//! This module provides lightweight, extensible structures that can be attached
//! to model entities without hard-coding source schemas.

use rkyv::{Archive, Deserialize, Serialize};

/// A globally unique identifier for the originating source record.
///
/// Examples:
/// - `gedcom7:I42`
/// - `wikidata:Q123`
/// - `local:person:7`
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Archive, Serialize, Deserialize)]
pub struct SourceRef(pub String);

/// A simple namespaced tag.
///
/// Examples:
/// - `user:tag:NeedsReview`
/// - `import:category:Artists`
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Archive, Serialize, Deserialize)]
pub struct Tag(pub String);

/// A generic key/value attribute.
///
/// This is meant for lossless import of miscellaneous fields when the core
/// schema does not (yet) have first-class fields.
#[derive(Debug, Clone, PartialEq, Eq, Archive, Serialize, Deserialize)]
pub struct Attribute {
    pub key: String,
    pub value: String,
}

/// A citation / evidence pointer.
///
/// This is intentionally minimal and can be expanded later.
#[derive(Debug, Clone, PartialEq, Eq, Archive, Serialize, Deserialize)]
pub struct Citation {
    pub source: SourceRef,
    pub detail: Option<String>,
    pub url: Option<String>,
    pub quote: Option<String>,
}

/// Provenance attached to an entity or assertion.
#[derive(Debug, Clone, PartialEq, Eq, Default, Archive, Serialize, Deserialize)]
pub struct Provenance {
    pub sources: Vec<SourceRef>,
    pub citations: Vec<Citation>,
    pub tags: Vec<Tag>,
    pub attributes: Vec<Attribute>,
}
