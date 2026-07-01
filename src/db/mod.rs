//! SQLite project database support.
//!
//! The initial schema stores GEDCOM uploads as immutable datasource rows. Parsed
//! people/families/events and user-editable overrides should live in separate
//! tables added by later migrations.

pub mod error;
pub mod gedcom_import;
pub mod place_resolution;
pub mod project;
pub mod project_document;
pub mod schema;

pub use error::DbError;
pub use gedcom_import::{GedcomImport, GedcomImportSummary, hash_gedcom_text, import_gedcom_file};
pub use place_resolution::{PlaceResolution, list_place_resolutions, upsert_place_resolution};
pub use project::{Project, create_project};
pub use project_document::{ProjectDocument, get_project_document, save_project_document};
pub use schema::{CURRENT_SCHEMA_VERSION, hash_gedcom_sha256, init_schema, open_database};
