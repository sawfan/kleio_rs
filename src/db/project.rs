use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::error::DbError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
}

pub fn create_project(conn: &Connection, name: &str) -> Result<Project, DbError> {
    let now = utc_now_rfc3339()?;
    let project = Project {
        id: Uuid::new_v4().to_string(),
        name: name.trim().to_string(),
        created_at: now.clone(),
        updated_at: now,
    };

    conn.execute(
        r#"
        INSERT INTO project (id, name, created_at, updated_at)
        VALUES (?1, ?2, ?3, ?4)
        "#,
        params![
            project.id,
            project.name,
            project.created_at,
            project.updated_at,
        ],
    )?;

    Ok(project)
}

pub(crate) fn project_exists(conn: &Connection, project_id: &str) -> Result<bool, DbError> {
    let exists = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM project WHERE id = ?1)",
        params![project_id],
        |row| row.get::<_, bool>(0),
    )?;
    Ok(exists)
}

pub(crate) fn utc_now_rfc3339() -> Result<String, DbError> {
    Ok(time::OffsetDateTime::now_utc().format(&time::format_description::well_known::Rfc3339)?)
}
