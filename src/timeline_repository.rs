//! Repository abstractions for loading/saving `TimelineDocument` values.
//!
//! This is intentionally small and synchronous for now. Browser/worker/SQLite
//! backends can implement the same conceptual boundary later without making UI
//! code depend directly on localStorage, OPFS, or filesystem details.

use crate::pack::TimelineDocument;
use crate::timeline_document_io::{
    timeline_document_from_json, timeline_document_from_toml, timeline_document_to_json_pretty,
    timeline_document_to_toml_pretty,
};

#[derive(Debug)]
pub enum TimelineRepositoryError {
    Io(std::io::Error),
    JsonSerialize(serde_json::Error),
    JsonDeserialize(serde_json::Error),
    TomlSerialize(toml::ser::Error),
    TomlDeserialize(toml::de::Error),
}

impl std::fmt::Display for TimelineRepositoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "io error: {err}"),
            Self::JsonSerialize(err) => write!(f, "json serialize error: {err}"),
            Self::JsonDeserialize(err) => write!(f, "json deserialize error: {err}"),
            Self::TomlSerialize(err) => write!(f, "toml serialize error: {err}"),
            Self::TomlDeserialize(err) => write!(f, "toml deserialize error: {err}"),
        }
    }
}

impl std::error::Error for TimelineRepositoryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::JsonSerialize(err) | Self::JsonDeserialize(err) => Some(err),
            Self::TomlSerialize(err) => Some(err),
            Self::TomlDeserialize(err) => Some(err),
        }
    }
}

impl From<std::io::Error> for TimelineRepositoryError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

pub trait TimelineDocumentRepository {
    fn load(&self) -> Result<TimelineDocument, TimelineRepositoryError>;
    fn save(&self, document: &TimelineDocument) -> Result<(), TimelineRepositoryError>;
}

#[cfg(not(target_arch = "wasm32"))]
pub mod file {
    use std::path::{Path, PathBuf};

    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct JsonTimelineDocumentFileRepository {
        path: PathBuf,
    }

    impl JsonTimelineDocumentFileRepository {
        pub fn new(path: impl Into<PathBuf>) -> Self {
            Self { path: path.into() }
        }

        pub fn path(&self) -> &Path {
            self.path.as_path()
        }
    }

    impl TimelineDocumentRepository for JsonTimelineDocumentFileRepository {
        fn load(&self) -> Result<TimelineDocument, TimelineRepositoryError> {
            let json = std::fs::read_to_string(&self.path)?;
            timeline_document_from_json(&json).map_err(TimelineRepositoryError::JsonDeserialize)
        }

        fn save(&self, document: &TimelineDocument) -> Result<(), TimelineRepositoryError> {
            if let Some(parent) = self.path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let json = timeline_document_to_json_pretty(document)
                .map_err(TimelineRepositoryError::JsonSerialize)?;
            std::fs::write(&self.path, json)?;
            Ok(())
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct TomlTimelineDocumentFileRepository {
        path: PathBuf,
    }

    impl TomlTimelineDocumentFileRepository {
        pub fn new(path: impl Into<PathBuf>) -> Self {
            Self { path: path.into() }
        }

        pub fn path(&self) -> &Path {
            self.path.as_path()
        }
    }

    impl TimelineDocumentRepository for TomlTimelineDocumentFileRepository {
        fn load(&self) -> Result<TimelineDocument, TimelineRepositoryError> {
            let toml_text = std::fs::read_to_string(&self.path)?;
            timeline_document_from_toml(&toml_text)
                .map_err(TimelineRepositoryError::TomlDeserialize)
        }

        fn save(&self, document: &TimelineDocument) -> Result<(), TimelineRepositoryError> {
            if let Some(parent) = self.path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let toml_text = timeline_document_to_toml_pretty(document)
                .map_err(TimelineRepositoryError::TomlSerialize)?;
            std::fs::write(&self.path, toml_text)?;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sample_timeline_document;

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn json_file_repository_saves_and_loads_document() {
        let path = std::env::temp_dir().join(format!(
            "kleio-timeline-document-{}.json",
            std::process::id()
        ));
        let repo = file::JsonTimelineDocumentFileRepository::new(&path);
        let document = sample_timeline_document();

        repo.save(&document).expect("save json document");
        let loaded = repo.load().expect("load json document");
        let _ = std::fs::remove_file(&path);

        assert_eq!(loaded.packs.len(), document.packs.len());
        assert_eq!(loaded.active_pack_ids, document.active_pack_ids);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn toml_file_repository_saves_and_loads_document() {
        let path = std::env::temp_dir().join(format!(
            "kleio-timeline-document-{}.toml",
            std::process::id()
        ));
        let repo = file::TomlTimelineDocumentFileRepository::new(&path);
        let document = sample_timeline_document();

        repo.save(&document).expect("save toml document");
        let loaded = repo.load().expect("load toml document");
        let _ = std::fs::remove_file(&path);

        assert_eq!(loaded.packs.len(), document.packs.len());
        assert_eq!(loaded.active_pack_ids, document.active_pack_ids);
    }
}
