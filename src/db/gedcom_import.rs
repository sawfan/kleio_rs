use std::{collections::hash_map::DefaultHasher, fs, hash::Hasher, path::Path};

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{
    error::DbError,
    project::{project_exists, utc_now_rfc3339},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GedcomImport {
    pub id: String,
    pub project_id: String,
    pub filename: String,
    pub file_hash: String,
    pub imported_at: String,
    pub parser_version: Option<String>,
    pub raw_gedcom: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GedcomImportSummary {
    pub id: String,
    pub project_id: String,
    pub filename: String,
    pub file_hash: String,
    pub imported_at: String,
    pub parser_version: Option<String>,
}

pub fn import_gedcom_path(
    conn: &mut Connection,
    project_id: &str,
    path: impl AsRef<Path>,
) -> Result<GedcomImport, DbError> {
    let path = path.as_ref();
    let gedcom_text = fs::read_to_string(path)?;
    let filename = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("import.ged");
    import_gedcom_file(conn, project_id, filename, &gedcom_text)
}

pub fn import_gedcom_file(
    conn: &mut Connection,
    project_id: &str,
    filename: &str,
    gedcom_text: &str,
) -> Result<GedcomImport, DbError> {
    let tx = conn.transaction()?;

    if !project_exists(&tx, project_id)? {
        return Err(DbError::ProjectNotFound(project_id.to_string()));
    }

    let imported_at = utc_now_rfc3339()?;
    let import = GedcomImport {
        id: Uuid::new_v4().to_string(),
        project_id: project_id.to_string(),
        filename: filename.to_string(),
        file_hash: hash_gedcom_text(gedcom_text),
        imported_at,
        parser_version: None,
        raw_gedcom: gedcom_text.to_string(),
    };

    tx.execute(
        r#"
        INSERT INTO gedcom_import (
            id,
            project_id,
            filename,
            file_hash,
            imported_at,
            parser_version,
            raw_gedcom
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        "#,
        params![
            import.id,
            import.project_id,
            import.filename,
            import.file_hash,
            import.imported_at,
            import.parser_version,
            import.raw_gedcom,
        ],
    )?;

    tx.commit()?;
    Ok(import)
}

pub fn list_gedcom_imports(
    conn: &Connection,
    project_id: &str,
) -> Result<Vec<GedcomImportSummary>, DbError> {
    let mut stmt = conn.prepare(
        r#"
        SELECT id, project_id, filename, file_hash, imported_at, parser_version
        FROM gedcom_import
        WHERE project_id = ?1
        ORDER BY imported_at DESC, id DESC
        "#,
    )?;

    let rows = stmt.query_map(params![project_id], |row| {
        Ok(GedcomImportSummary {
            id: row.get(0)?,
            project_id: row.get(1)?,
            filename: row.get(2)?,
            file_hash: row.get(3)?,
            imported_at: row.get(4)?,
            parser_version: row.get(5)?,
        })
    })?;

    rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
}

pub fn read_gedcom_import(
    conn: &Connection,
    import_id: &str,
) -> Result<Option<GedcomImport>, DbError> {
    conn.query_row(
        r#"
        SELECT id, project_id, filename, file_hash, imported_at, parser_version, raw_gedcom
        FROM gedcom_import
        WHERE id = ?1
        "#,
        params![import_id],
        |row| {
            Ok(GedcomImport {
                id: row.get(0)?,
                project_id: row.get(1)?,
                filename: row.get(2)?,
                file_hash: row.get(3)?,
                imported_at: row.get(4)?,
                parser_version: row.get(5)?,
                raw_gedcom: row.get(6)?,
            })
        },
    )
    .optional()
    .map_err(DbError::from)
}

pub fn hash_gedcom_text(gedcom_text: &str) -> String {
    let mut hasher = DefaultHasher::new();
    hasher.write(gedcom_text.as_bytes());
    format!("siphash64:{:016x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{create_project, init_schema};

    #[test]
    fn imports_and_reads_raw_gedcom() {
        let mut conn = Connection::open_in_memory().expect("open in-memory sqlite");
        init_schema(&conn).expect("init schema");
        let project = create_project(&conn, "Family Tree").expect("create project");
        let raw = "0 HEAD\n1 SOUR kleio-test\n0 TRLR\n";

        let import =
            import_gedcom_file(&mut conn, &project.id, "family.ged", raw).expect("import GEDCOM");
        assert_eq!(import.filename, "family.ged");
        assert_eq!(import.raw_gedcom, raw);
        assert_eq!(import.file_hash, hash_gedcom_text(raw));

        let summaries = list_gedcom_imports(&conn, &project.id).expect("list imports");
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].id, import.id);

        let read = read_gedcom_import(&conn, &import.id)
            .expect("read import")
            .expect("import row exists");
        assert_eq!(read.raw_gedcom, raw);
    }
}
