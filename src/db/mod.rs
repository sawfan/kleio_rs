//! SQLite project database support.
//!
//! The initial schema stores GEDCOM uploads as immutable datasource rows. Parsed
//! people/families/events and user-editable overrides should live in separate
//! tables added by later migrations.

pub mod error;
pub mod gedcom_import;
pub mod project;
pub mod schema;

pub use error::DbError;
pub use gedcom_import::{GedcomImport, GedcomImportSummary, hash_gedcom_text, import_gedcom_file};
pub use project::{Project, create_project};
pub use schema::{CURRENT_SCHEMA_VERSION, init_schema, open_database};
