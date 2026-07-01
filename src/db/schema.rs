use std::path::Path;

use rusqlite::Connection;

use super::error::DbError;

pub const CURRENT_SCHEMA_VERSION: i64 = 1;

pub fn open_database(path: impl AsRef<Path>) -> Result<Connection, DbError> {
    let conn = Connection::open(path)?;
    enable_foreign_keys(&conn)?;
    Ok(conn)
}

pub fn enable_foreign_keys(conn: &Connection) -> Result<(), DbError> {
    conn.pragma_update(None, "foreign_keys", true)?;
    Ok(())
}

pub fn init_schema(conn: &Connection) -> Result<(), DbError> {
    conn.execute_batch(
        r#"
        PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS project (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS gedcom_import (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            filename TEXT NOT NULL,
            file_hash TEXT NOT NULL,
            imported_at TEXT NOT NULL,
            parser_version TEXT,
            raw_gedcom TEXT NOT NULL,
            FOREIGN KEY(project_id) REFERENCES project(id)
        );

        CREATE INDEX IF NOT EXISTS idx_gedcom_import_project_id
            ON gedcom_import(project_id);

        CREATE INDEX IF NOT EXISTS idx_gedcom_import_file_hash
            ON gedcom_import(file_hash);
        "#,
    )?;
    conn.pragma_update(None, "user_version", CURRENT_SCHEMA_VERSION)?;
    Ok(())
}
