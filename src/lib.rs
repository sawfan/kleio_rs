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
pub mod biography;
#[cfg(feature = "sqlite")]
pub mod db;
pub mod entity;
pub mod event;
pub mod event_adapter;
pub mod event_composition;
pub mod event_query;
pub mod event_type;
pub mod event_validation;
pub mod genealogy_timeline;
pub mod import;
pub mod import_batch;
pub mod import_validation;
pub mod model;
pub mod pack;
pub mod pack_builder;
pub mod pack_import;
pub mod pack_json;
pub mod pack_samples;
pub mod pack_toml;
pub mod store;
pub mod timeline_document_io;
pub mod timeline_repository;
pub mod timeline_repository_async;
pub mod timeline_source;
pub mod tree;

pub use archive::*;
pub use attribution::*;
pub use biography::*;
pub use entity::*;
pub use event::*;
pub use event_adapter::*;
pub use event_composition::*;
pub use event_query::*;
pub use event_type::*;
pub use event_validation::*;
pub use genealogy_timeline::*;
pub use import_batch::*;
pub use import_validation::*;
pub use model::*;
pub use pack::*;
pub use pack_builder::*;
pub use pack_import::*;
pub use pack_json::*;
pub use pack_samples::*;
pub use pack_toml::*;
pub use store::*;
pub use timeline_document_io::*;
pub use timeline_repository::*;
pub use timeline_repository_async::*;
pub use timeline_source::*;
pub use tree::*;
