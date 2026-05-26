//! Kleio: source-agnostic genealogy primitives.
//!
//! This crate provides a normalized people/events/places model plus derived
//! indexes that can be archived with `rkyv` for fast load times.
//!
//! Design goals:
//! - Import from multiple sources (Astrodatabank, GEDCOM 7, etc.) into a common model.
//! - Preserve source-specific details via generic attribution/tags without baking in
//!   any single upstream schema.
//! - Provide ergonomic runtime access (`GenealogyStore`) on top of archived bytes.

pub mod archive;
pub mod attribution;
pub mod model;
pub mod store;

pub use archive::*;
pub use attribution::*;
pub use model::*;
pub use store::*;
