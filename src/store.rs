use std::collections::{BTreeSet, HashMap};

use rkyv::{rancor::Error, util::AlignedVec};

use crate::archive::view_archived_genealogy_archive;
use crate::model::{EventId, GenealogyArchive, PersonId, SearchIndex};

#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;

#[cfg(not(target_arch = "wasm32"))]
use crate::archive::load_genealogy_index_archive;

/// Errors that can occur while creating or accessing a `GenealogyStore`.
#[derive(Debug)]
pub enum GenealogyStoreError {
    Io(std::io::Error),
    Rkyv(Error),
}

impl std::fmt::Display for GenealogyStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "io error: {err}"),
            Self::Rkyv(err) => write!(f, "rkyv error: {err}"),
        }
    }
}

impl std::error::Error for GenealogyStoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            // `rkyv::rancor::Error` does not currently expose an underlying source error.
            Self::Rkyv(_err) => None,
        }
    }
}

impl From<std::io::Error> for GenealogyStoreError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<Error> for GenealogyStoreError {
    fn from(err: Error) -> Self {
        Self::Rkyv(err)
    }
}

/// Consumer-facing wrapper around a serialized `GenealogyArchive`.
///
/// The key design constraint with `rkyv` in browser/WASM contexts is that any
/// `&Archived<T>` reference must never outlive the backing bytes.
///
/// `GenealogyStore` solves this by owning the `Vec<u8>` and only producing
/// archive references that are borrowed from `&self`.
#[derive(Debug, Clone)]
pub struct GenealogyStore {
    bytes: AlignedVec,

    // Consumer-friendly, runtime indexes (derived from the archived BTreeMaps).
    person_by_id: HashMap<u64, u32>,
    event_by_id: HashMap<u64, u32>,
    note_by_id: HashMap<u64, u32>,
    events_by_year: HashMap<i32, Vec<u32>>,
}

impl GenealogyStore {
    /// Create a `GenealogyStore` from already-loaded bytes.
    ///
    /// This validates the archive and builds a small set of runtime indexes to
    /// make common lookups ergonomic.
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, GenealogyStoreError> {
        // `rkyv` requires aligned access. Bytes returned from `fetch` in the browser
        // (or other sources) are not guaranteed to be aligned, so we copy into an
        // `AlignedVec` before accessing.
        let mut aligned = AlignedVec::with_capacity(bytes.len());
        aligned.extend_from_slice(&bytes);

        let archived = view_archived_genealogy_archive(&aligned)?;

        let mut person_by_id: HashMap<u64, u32> =
            HashMap::with_capacity(archived.person_by_id.len());
        for (id, idx) in archived.person_by_id.iter() {
            person_by_id.insert(id.0.into(), (*idx).into());
        }

        let mut event_by_id: HashMap<u64, u32> = HashMap::with_capacity(archived.event_by_id.len());
        for (id, idx) in archived.event_by_id.iter() {
            event_by_id.insert(id.0.into(), (*idx).into());
        }

        let mut note_by_id: HashMap<u64, u32> = HashMap::with_capacity(archived.note_by_id.len());
        for (id, idx) in archived.note_by_id.iter() {
            note_by_id.insert(id.0.into(), (*idx).into());
        }

        let mut events_by_year: HashMap<i32, Vec<u32>> =
            HashMap::with_capacity(archived.date_index.events_by_year.len());
        for (year, postings) in archived.date_index.events_by_year.iter() {
            let postings: Vec<u32> = postings.iter().map(|idx| (*idx).into()).collect();
            events_by_year.insert((*year).into(), postings);
        }

        Ok(Self {
            bytes: aligned,
            person_by_id,
            event_by_id,
            note_by_id,
            events_by_year,
        })
    }

    /// Read bytes from a `.rkyv` file and create a validated `GenealogyStore`.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, GenealogyStoreError> {
        let bytes = load_genealogy_index_archive(path)?;
        Self::from_bytes(bytes)
    }

    /// Access the archived snapshot.
    ///
    /// Note: the returned reference is borrowed from `&self` and is only valid
    /// as long as `self` (and therefore `self.bytes`) is alive.
    pub fn archived(&self) -> Result<&rkyv::Archived<GenealogyArchive>, GenealogyStoreError> {
        Ok(view_archived_genealogy_archive(&self.bytes)?)
    }

    /// Execute a closure with a temporary borrow of the archived snapshot.
    ///
    /// The higher-ranked trait bound (`for<'a>`) prevents the returned value
    /// from containing references into the archive. This is a convenient pattern
    /// for browser/WASM callers that want to avoid accidentally leaking
    /// `&Archived<_>` references.
    pub fn with_archived<R>(
        &self,
        f: impl for<'a> FnOnce(&'a rkyv::Archived<GenealogyArchive>) -> R,
    ) -> Result<R, GenealogyStoreError> {
        let archived = self.archived()?;
        Ok(f(archived))
    }

    /// Borrow the raw bytes (useful for WASM apps that need to keep the bytes alive
    /// while passing them across layers).
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Resolve a `PersonId` to a person record.
    pub fn person(
        &self,
        id: PersonId,
    ) -> Result<Option<&rkyv::Archived<crate::model::Person>>, GenealogyStoreError> {
        let Some(&idx) = self.person_by_id.get(&id.0) else {
            return Ok(None);
        };

        let archived = self.archived()?;
        Ok(archived.people.get(idx as usize))
    }

    /// Resolve an `EventId` to an event record.
    pub fn event(
        &self,
        id: EventId,
    ) -> Result<Option<&rkyv::Archived<crate::GenealogyEvent>>, GenealogyStoreError> {
        let Some(&idx) = self.event_by_id.get(&id.0) else {
            return Ok(None);
        };

        let archived = self.archived()?;
        Ok(archived.events.get(idx as usize))
    }

    /// Resolve a `NoteId` to a note record.
    pub fn note(
        &self,
        id: crate::model::NoteId,
    ) -> Result<Option<&rkyv::Archived<crate::model::Note>>, GenealogyStoreError> {
        let Some(&idx) = self.note_by_id.get(&id.0) else {
            return Ok(None);
        };

        let archived = self.archived()?;
        Ok(archived.notes.get(idx as usize))
    }

    /// Return notes for a person.
    pub fn notes_for_person(
        &self,
        person: &rkyv::Archived<crate::model::Person>,
        limit: usize,
    ) -> Result<Vec<&rkyv::Archived<crate::model::Note>>, GenealogyStoreError> {
        let archived = self.archived()?;

        let mut out = Vec::new();
        for note_id in person.notes.iter().take(limit) {
            if let Some(&idx) = self.note_by_id.get(&note_id.0.into())
                && let Some(note) = archived.notes.get(idx as usize)
            {
                out.push(note);
            }
        }

        Ok(out)
    }

    /// Extract simple http(s) links from a blob of text.
    pub fn extract_links(text: &str, limit: usize) -> Vec<String> {
        let mut out = Vec::new();

        for token in text.split_whitespace() {
            let token = token.trim_matches(|c: char| {
                matches!(
                    c,
                    '(' | ')'
                        | '['
                        | ']'
                        | '{'
                        | '}'
                        | '<'
                        | '>'
                        | '"'
                        | '\''
                        | ','
                        | '.'
                        | ';'
                        | ':'
                )
            });

            if token.starts_with("http://") || token.starts_with("https://") {
                out.push(token.to_owned());
                if out.len() >= limit {
                    break;
                }
            }
        }

        out
    }

    /// Search people by a free-form query string.
    ///
    /// This uses the archived `name_index` and returns matching person records.
    ///
    /// Matching strategy:
    /// - Tokenize the query (same rules as during index construction)
    /// - Intersect the postings lists for each token
    /// - Return results in ascending record index order (stable)
    pub fn search_people_by_name(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<&rkyv::Archived<crate::model::Person>>, GenealogyStoreError> {
        let archived = self.archived()?;

        let mut hits: Option<BTreeSet<u32>> = None;
        let mut tokens = SearchIndex::tokenize(query).peekable();
        if tokens.peek().is_none() {
            return Ok(Vec::new());
        }

        for token in tokens {
            let postings = archived.name_index.postings.get(token.as_str());
            let Some(postings) = postings else {
                return Ok(Vec::new());
            };

            let set: BTreeSet<u32> = postings.iter().map(|i| (*i).into()).collect();
            hits = match hits {
                None => Some(set),
                Some(prev) => Some(prev.intersection(&set).copied().collect()),
            };

            if hits.as_ref().is_some_and(|s| s.is_empty()) {
                return Ok(Vec::new());
            }
        }

        let Some(hits) = hits else {
            return Ok(Vec::new());
        };

        let mut out = Vec::new();
        for idx in hits.into_iter().take(limit) {
            if let Some(person) = archived.people.get(idx as usize) {
                out.push(person);
            }
        }

        Ok(out)
    }

    /// Convenience helper: resolve a place by its raw u64 id.
    pub fn place_by_u64(
        &self,
        place_id: u64,
    ) -> Result<Option<&rkyv::Archived<crate::model::Place>>, GenealogyStoreError> {
        let archived = self.archived()?;
        for p in archived.places.iter() {
            if p.id.0 == place_id {
                return Ok(Some(p));
            }
        }
        Ok(None)
    }

    /// Return all events in the given year.
    pub fn events_in_year(
        &self,
        year: i32,
        limit: usize,
    ) -> Result<Vec<&rkyv::Archived<crate::GenealogyEvent>>, GenealogyStoreError> {
        let Some(postings) = self.events_by_year.get(&year) else {
            return Ok(Vec::new());
        };

        let archived = self.archived()?;
        let mut out = Vec::new();
        for idx in postings.iter().copied().take(limit) {
            if let Some(ev) = archived.events.get(idx as usize) {
                out.push(ev);
            }
        }

        Ok(out)
    }
}
