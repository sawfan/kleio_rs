use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use super::{
    error::DbError,
    project::{project_exists, utc_now_rfc3339},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectDocument {
    pub project_id: String,
    pub document_json: String,
    pub updated_at: String,
}

pub fn save_project_document(
    conn: &Connection,
    project_id: &str,
    document_json: &str,
) -> Result<ProjectDocument, DbError> {
    if !project_exists(conn, project_id)? {
        return Err(DbError::ProjectNotFound(project_id.to_string()));
    }

    let document = ProjectDocument {
        project_id: project_id.to_string(),
        document_json: document_json.to_string(),
        updated_at: utc_now_rfc3339()?,
    };

    conn.execute(
        r#"
        INSERT INTO project_document (project_id, document_json, updated_at)
        VALUES (?1, ?2, ?3)
        ON CONFLICT(project_id) DO UPDATE SET
          document_json = excluded.document_json,
          updated_at = excluded.updated_at
        "#,
        params![
            document.project_id,
            document.document_json,
            document.updated_at
        ],
    )?;

    Ok(document)
}

pub fn get_project_document(
    conn: &Connection,
    project_id: &str,
) -> Result<Option<ProjectDocument>, DbError> {
    conn.query_row(
        r#"
        SELECT project_id, document_json, updated_at
        FROM project_document
        WHERE project_id = ?1
        "#,
        params![project_id],
        |row| {
            Ok(ProjectDocument {
                project_id: row.get(0)?,
                document_json: row.get(1)?,
                updated_at: row.get(2)?,
            })
        },
    )
    .optional()
    .map_err(DbError::from)
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use super::*;
    use crate::db::{create_project, init_schema};

    #[test]
    fn saves_and_updates_project_document() {
        let conn = Connection::open_in_memory().expect("open in-memory sqlite");
        init_schema(&conn).expect("init schema");
        let project = create_project(&conn, "Family Tree").expect("create project");

        let saved =
            save_project_document(&conn, &project.id, "{\"version\":1}").expect("save document");
        assert_eq!(saved.project_id, project.id);

        let updated =
            save_project_document(&conn, &project.id, "{\"version\":2}").expect("update document");
        assert_eq!(updated.project_id, project.id);

        let read = get_project_document(&conn, &project.id)
            .expect("read document")
            .expect("document exists");
        assert_eq!(read.document_json, "{\"version\":2}");
    }
}
