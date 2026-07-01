use std::io;

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("time formatting error: {0}")]
    TimeFormat(#[from] time::error::Format),

    #[error("project not found: {0}")]
    ProjectNotFound(String),
}
