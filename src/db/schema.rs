use std::path::Path;

use rusqlite::Connection;
use sha2::{Digest, Sha256};

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
            byte_len INTEGER NOT NULL DEFAULT 0,
            line_count INTEGER,
            imported_at TEXT NOT NULL,
            parser_version TEXT,
            raw_gedcom TEXT NOT NULL,
            note TEXT,
            FOREIGN KEY(project_id) REFERENCES project(id)
        );

        CREATE INDEX IF NOT EXISTS idx_gedcom_import_project_id
            ON gedcom_import(project_id);

        CREATE INDEX IF NOT EXISTS idx_gedcom_import_file_hash
            ON gedcom_import(file_hash);

        CREATE TABLE IF NOT EXISTS project_document (
            project_id TEXT PRIMARY KEY,
            document_json TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY(project_id) REFERENCES project(id)
        );

        CREATE TABLE IF NOT EXISTS place_resolution (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            place_key TEXT NOT NULL,
            lookup_key TEXT,
            query TEXT,
            selected_label TEXT,
            latitude REAL,
            longitude REAL,
            provider TEXT,
            provider_id TEXT,
            confidence TEXT,
            candidates_json TEXT,
            note TEXT,
            updated_at TEXT NOT NULL,
            UNIQUE(project_id, place_key),
            FOREIGN KEY(project_id) REFERENCES project(id)
        );

        CREATE INDEX IF NOT EXISTS idx_place_resolution_project_id
            ON place_resolution(project_id);
        "#,
    )?;
    migrate_schema(conn)?;
    backfill_gedcom_import_metadata(conn)?;
    conn.pragma_update(None, "user_version", CURRENT_SCHEMA_VERSION)?;
    Ok(())
}

fn migrate_schema(conn: &Connection) -> Result<(), DbError> {
    ensure_column(
        conn,
        "gedcom_import",
        "byte_len",
        "ALTER TABLE gedcom_import ADD COLUMN byte_len INTEGER NOT NULL DEFAULT 0",
    )?;
    ensure_column(
        conn,
        "gedcom_import",
        "line_count",
        "ALTER TABLE gedcom_import ADD COLUMN line_count INTEGER",
    )?;
    ensure_column(
        conn,
        "gedcom_import",
        "note",
        "ALTER TABLE gedcom_import ADD COLUMN note TEXT",
    )?;
    Ok(())
}

fn ensure_column(
    conn: &Connection,
    table: &str,
    column: &str,
    alter_sql: &str,
) -> Result<(), DbError> {
    if !column_exists(conn, table, column)? {
        conn.execute_batch(alter_sql)?;
    }
    Ok(())
}

fn column_exists(conn: &Connection, table: &str, column: &str) -> Result<bool, DbError> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;
    for row in rows {
        if row? == column {
            return Ok(true);
        }
    }
    Ok(false)
}

fn backfill_gedcom_import_metadata(conn: &Connection) -> Result<(), DbError> {
    let mut stmt = conn.prepare(
        r#"
        SELECT id, raw_gedcom
        FROM gedcom_import
        WHERE byte_len = 0 OR line_count IS NULL OR file_hash LIKE 'siphash64:%'
        "#,
    )?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    drop(stmt);

    for (id, raw_gedcom) in rows {
        let byte_len = raw_gedcom.len() as i64;
        let line_count = raw_gedcom.lines().count() as i64;
        let file_hash = hash_gedcom_sha256(&raw_gedcom);
        conn.execute(
            r#"
            UPDATE gedcom_import
            SET byte_len = ?1, line_count = ?2, file_hash = ?3
            WHERE id = ?4
            "#,
            rusqlite::params![byte_len, line_count, file_hash, id],
        )?;
    }

    Ok(())
}

#[must_use]
pub fn hash_gedcom_sha256(raw_gedcom: &str) -> String {
    let digest = Sha256::digest(raw_gedcom.as_bytes());
    format!("sha256:{digest:x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_schema_migrates_legacy_gedcom_import_table() {
        let conn = Connection::open_in_memory().expect("open in-memory sqlite");
        conn.execute_batch(
            r#"
            PRAGMA foreign_keys = ON;
            CREATE TABLE project (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE gedcom_import (
                id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL,
                filename TEXT NOT NULL,
                file_hash TEXT NOT NULL,
                imported_at TEXT NOT NULL,
                parser_version TEXT,
                raw_gedcom TEXT NOT NULL,
                FOREIGN KEY(project_id) REFERENCES project(id)
            );
            INSERT INTO project (id, name, created_at, updated_at)
            VALUES ('p1', 'Legacy', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z');
            INSERT INTO gedcom_import (
                id, project_id, filename, file_hash, imported_at, parser_version, raw_gedcom
            ) VALUES (
                'g1', 'p1', 'legacy.ged', 'siphash64:bad', '2026-01-01T00:00:00Z', NULL,
                '0 HEAD
1 SOUR legacy
0 TRLR
'
            );
            "#,
        )
        .expect("create legacy schema");

        init_schema(&conn).expect("migrate schema");

        let byte_len: i64 = conn
            .query_row(
                "SELECT byte_len FROM gedcom_import WHERE id = 'g1'",
                [],
                |row| row.get(0),
            )
            .expect("read byte_len");
        let line_count: i64 = conn
            .query_row(
                "SELECT line_count FROM gedcom_import WHERE id = 'g1'",
                [],
                |row| row.get(0),
            )
            .expect("read line_count");
        let file_hash: String = conn
            .query_row(
                "SELECT file_hash FROM gedcom_import WHERE id = 'g1'",
                [],
                |row| row.get(0),
            )
            .expect("read file_hash");

        assert_eq!(byte_len, "0 HEAD\n1 SOUR legacy\n0 TRLR\n".len() as i64);
        assert_eq!(line_count, 3);
        assert!(file_hash.starts_with("sha256:"));
    }
}
