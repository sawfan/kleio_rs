use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{
    error::DbError,
    project::{project_exists, utc_now_rfc3339},
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlaceResolution {
    pub id: String,
    pub project_id: String,
    pub place_key: String,
    pub lookup_key: Option<String>,
    pub query: Option<String>,
    pub selected_label: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub provider: Option<String>,
    pub provider_id: Option<String>,
    pub confidence: Option<String>,
    pub candidates_json: Option<String>,
    pub note: Option<String>,
    pub updated_at: String,
}

pub fn upsert_place_resolution(
    conn: &Connection,
    mut resolution: PlaceResolution,
) -> Result<PlaceResolution, DbError> {
    if !project_exists(conn, &resolution.project_id)? {
        return Err(DbError::ProjectNotFound(resolution.project_id));
    }
    if resolution.id.trim().is_empty() {
        resolution.id = Uuid::new_v4().to_string();
    }
    resolution.updated_at = utc_now_rfc3339()?;

    conn.execute(
        r#"
        INSERT INTO place_resolution (
          id, project_id, place_key, lookup_key, query, selected_label,
          latitude, longitude, provider, provider_id, confidence,
          candidates_json, note, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
        ON CONFLICT(project_id, place_key) DO UPDATE SET
          lookup_key = excluded.lookup_key,
          query = excluded.query,
          selected_label = excluded.selected_label,
          latitude = excluded.latitude,
          longitude = excluded.longitude,
          provider = excluded.provider,
          provider_id = excluded.provider_id,
          confidence = excluded.confidence,
          candidates_json = excluded.candidates_json,
          note = excluded.note,
          updated_at = excluded.updated_at
        "#,
        params![
            resolution.id,
            resolution.project_id,
            resolution.place_key,
            resolution.lookup_key,
            resolution.query,
            resolution.selected_label,
            resolution.latitude,
            resolution.longitude,
            resolution.provider,
            resolution.provider_id,
            resolution.confidence,
            resolution.candidates_json,
            resolution.note,
            resolution.updated_at,
        ],
    )?;

    Ok(resolution)
}

pub fn list_place_resolutions(
    conn: &Connection,
    project_id: &str,
) -> Result<Vec<PlaceResolution>, DbError> {
    let mut stmt = conn.prepare(
        r#"
        SELECT id, project_id, place_key, lookup_key, query, selected_label,
               latitude, longitude, provider, provider_id, confidence,
               candidates_json, note, updated_at
        FROM place_resolution
        WHERE project_id = ?1
        ORDER BY place_key ASC
        "#,
    )?;

    let rows = stmt.query_map(params![project_id], |row| {
        Ok(PlaceResolution {
            id: row.get(0)?,
            project_id: row.get(1)?,
            place_key: row.get(2)?,
            lookup_key: row.get(3)?,
            query: row.get(4)?,
            selected_label: row.get(5)?,
            latitude: row.get(6)?,
            longitude: row.get(7)?,
            provider: row.get(8)?,
            provider_id: row.get(9)?,
            confidence: row.get(10)?,
            candidates_json: row.get(11)?,
            note: row.get(12)?,
            updated_at: row.get(13)?,
        })
    })?;

    rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use super::*;
    use crate::db::{create_project, init_schema};

    #[test]
    fn upserts_place_resolution_by_project_and_place_key() {
        let conn = Connection::open_in_memory().expect("open in-memory sqlite");
        init_schema(&conn).expect("init schema");
        let project = create_project(&conn, "Family Tree").expect("create project");

        let first = upsert_place_resolution(
            &conn,
            PlaceResolution {
                id: String::new(),
                project_id: project.id.clone(),
                place_key: "place:london england".to_string(),
                lookup_key: Some("lookup:london:country:gb:admin:".to_string()),
                query: Some("London".to_string()),
                selected_label: Some("London, England".to_string()),
                latitude: Some(51.5074),
                longitude: Some(-0.1278),
                provider: Some("geosuggest".to_string()),
                provider_id: Some("test:london".to_string()),
                confidence: Some("first_pass_suggestion".to_string()),
                candidates_json: Some("[]".to_string()),
                note: None,
                updated_at: String::new(),
            },
        )
        .expect("insert place resolution");
        assert!(!first.id.is_empty());

        upsert_place_resolution(
            &conn,
            PlaceResolution {
                selected_label: Some("Greater London, England".to_string()),
                note: Some("manual correction".to_string()),
                ..first.clone()
            },
        )
        .expect("update place resolution");

        let rows = list_place_resolutions(&conn, &project.id).expect("list place resolutions");
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0].selected_label.as_deref(),
            Some("Greater London, England")
        );
        assert_eq!(rows[0].note.as_deref(), Some("manual correction"));
    }
}
