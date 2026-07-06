//! Kleio: source-agnostic people and event primitives.
//!
//! This crate provides a normalized people/events/places model plus derived
//! indexes that can be archived with `rkyv` for fast load times.
//!
//! Design goals:
//! - Represent people, events, families, places, notes, and provenance in a common model.
//! - Preserve importer-specific details via generic attribution/tags without baking in
//!   any single upstream schema.
//! - Provide ergonomic runtime access (`GenealogyStore`) on top of archived bytes.

pub mod archive;
pub mod attribution;
#[cfg(feature = "sqlite")]
pub mod db;
pub mod entity;
pub mod event;
pub mod event_adapter;
pub mod event_query;
pub mod event_type;
pub mod import;
pub mod model;
pub mod pack;
pub mod store;
pub mod tree;

pub use archive::*;
pub use attribution::*;
pub use entity::*;
pub use event::*;
pub use event_adapter::*;
pub use event_query::*;
pub use event_type::*;
pub use model::*;
pub use pack::*;
pub use store::*;
pub use tree::*;
