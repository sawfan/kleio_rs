//! Import batch and pack-creation primitives.
//!
//! Imports are modeled as provenance-preserving batches of raw records plus
//! normalized candidates. Applying a batch can materialize an `EventPack`
//! without mutating the original source data.

use rkyv::{Archive, Deserialize, Serialize};

use crate::attribution::Provenance;
use crate::entity::Entity;
use crate::event::{EventRelation, TimelineEvent};
use crate::event_type::DomainProfile;
use crate::pack::{EventPack, PackKind, PackMetadata, SourceRecord};

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Archive,
    Serialize,
    Deserialize,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct ImportBatchId(pub String);

impl ImportBatchId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Archive,
    Serialize,
    Deserialize,
    serde::Serialize,
    serde::Deserialize,
)]
pub enum ImportSourceKind {
    Gedcom,
    Csv,
    Json,
    Toml,
    Ics,
    Markdown,
    Wikidata,
    Manual,
    Custom(String),
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Archive,
    Serialize,
    Deserialize,
    serde::Serialize,
    serde::Deserialize,
)]
pub enum ImportStatus {
    Draft,
    Parsed,
    Previewed,
    Applied,
    Failed,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Archive,
    Serialize,
    Deserialize,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct ImportRecord {
    pub id: String,
    pub source_record_id: Option<String>,
    pub summary: Option<String>,
    pub raw: Option<String>,
    pub provenance: Provenance,
}

impl ImportRecord {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            source_record_id: None,
            summary: None,
            raw: None,
            provenance: Provenance::default(),
        }
    }
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Archive,
    Serialize,
    Deserialize,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct ImportCandidateId(pub String);

impl ImportCandidateId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

// Keep candidate items stored inline so serde/rkyv representations and the public
// enum API remain stable across import-batch persistence boundaries.
#[allow(clippy::large_enum_variant)]
#[derive(
    Debug, Clone, PartialEq, Archive, Serialize, Deserialize, serde::Serialize, serde::Deserialize,
)]
pub enum ImportCandidateItem {
    DomainProfile(DomainProfile),
    Entity(Entity),
    Event(TimelineEvent),
    EventRelation(EventRelation),
    Source(SourceRecord),
    TagValue(crate::attribution::Tag),
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Archive,
    Serialize,
    Deserialize,
    serde::Serialize,
    serde::Deserialize,
)]
pub enum ImportAction {
    Add,
    Update,
    Link,
    Ignore,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Archive,
    Serialize,
    Deserialize,
    serde::Serialize,
    serde::Deserialize,
)]
pub enum ImportCandidateStatus {
    Pending,
    Accepted,
    Rejected,
    Conflict,
}

#[derive(
    Debug, Clone, PartialEq, Archive, Serialize, Deserialize, serde::Serialize, serde::Deserialize,
)]
pub struct ImportCandidate {
    pub id: ImportCandidateId,
    pub record_id: Option<String>,
    pub item: ImportCandidateItem,
    pub action: ImportAction,
    pub status: ImportCandidateStatus,
    pub messages: Vec<String>,
    pub provenance: Provenance,
}

impl ImportCandidate {
    pub fn add(id: ImportCandidateId, item: ImportCandidateItem) -> Self {
        Self {
            id,
            record_id: None,
            item,
            action: ImportAction::Add,
            status: ImportCandidateStatus::Pending,
            messages: Vec::new(),
            provenance: Provenance::default(),
        }
    }

    pub fn accepted(mut self) -> Self {
        self.status = ImportCandidateStatus::Accepted;
        self
    }

    pub fn rejected(mut self, message: impl Into<String>) -> Self {
        self.status = ImportCandidateStatus::Rejected;
        self.messages.push(message.into());
        self
    }

    pub fn conflict(mut self, message: impl Into<String>) -> Self {
        self.status = ImportCandidateStatus::Conflict;
        self.messages.push(message.into());
        self
    }

    pub fn is_accepted(&self) -> bool {
        self.status == ImportCandidateStatus::Accepted
    }
}

#[derive(
    Debug, Clone, PartialEq, Archive, Serialize, Deserialize, serde::Serialize, serde::Deserialize,
)]
pub struct ImportBatch {
    pub id: ImportBatchId,
    pub source_name: String,
    pub source_kind: ImportSourceKind,
    pub imported_at: Option<String>,
    pub status: ImportStatus,
    pub records: Vec<ImportRecord>,
    pub candidates: Vec<ImportCandidate>,
    pub provenance: Provenance,
}

impl ImportBatch {
    pub fn new(
        id: ImportBatchId,
        source_name: impl Into<String>,
        source_kind: ImportSourceKind,
    ) -> Self {
        Self {
            id,
            source_name: source_name.into(),
            source_kind,
            imported_at: None,
            status: ImportStatus::Draft,
            records: Vec::new(),
            candidates: Vec::new(),
            provenance: Provenance::default(),
        }
    }

    pub fn accepted_candidates(&self) -> impl Iterator<Item = &ImportCandidate> {
        self.candidates
            .iter()
            .filter(|candidate| candidate.is_accepted())
    }

    pub fn conflict_count(&self) -> usize {
        self.candidates
            .iter()
            .filter(|candidate| candidate.status == ImportCandidateStatus::Conflict)
            .count()
    }

    pub fn accepted_count(&self) -> usize {
        self.accepted_candidates().count()
    }

    pub fn materialize_event_pack(&self, metadata: PackMetadata, kind: PackKind) -> EventPack {
        let mut pack = EventPack::empty(metadata, kind);
        pack.provenance = self.provenance.clone();

        for candidate in self.accepted_candidates() {
            match &candidate.item {
                ImportCandidateItem::DomainProfile(profile) => {
                    pack.domain_profiles.push(profile.clone())
                }
                ImportCandidateItem::Entity(entity) => pack.entities.push(entity.clone()),
                ImportCandidateItem::Event(event) => pack.events.push(event.clone()),
                ImportCandidateItem::EventRelation(relation) => {
                    pack.event_relations.push(relation.clone())
                }
                ImportCandidateItem::Source(source) => pack.sources.push(source.clone()),
                ImportCandidateItem::TagValue(tag) => pack.tags.push(tag.clone()),
            }
        }

        pack
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EventId, EventTypeId, PackId};

    #[test]
    fn batch_materializes_only_accepted_candidates_into_pack() {
        let mut batch = ImportBatch::new(
            ImportBatchId::new("import:1"),
            "journal.json",
            ImportSourceKind::Json,
        );
        batch.candidates.push(
            ImportCandidate::add(
                ImportCandidateId::new("candidate:event:1"),
                ImportCandidateItem::Event(TimelineEvent::new(
                    EventId(1),
                    EventTypeId::new("journal.entry"),
                    "Accepted",
                )),
            )
            .accepted(),
        );
        batch.candidates.push(
            ImportCandidate::add(
                ImportCandidateId::new("candidate:event:2"),
                ImportCandidateItem::Event(TimelineEvent::new(
                    EventId(2),
                    EventTypeId::new("journal.entry"),
                    "Rejected",
                )),
            )
            .rejected("duplicate"),
        );

        let pack = batch.materialize_event_pack(
            PackMetadata::new(PackId::new("pack:import:1"), "Imported Journal"),
            PackKind::ImportedDataset,
        );

        assert_eq!(batch.accepted_count(), 1);
        assert_eq!(pack.events.len(), 1);
        assert_eq!(pack.events[0].id, EventId(1));
    }

    #[test]
    fn batch_counts_conflicts_for_preview() {
        let mut batch = ImportBatch::new(
            ImportBatchId::new("import:2"),
            "events.csv",
            ImportSourceKind::Csv,
        );
        batch.candidates.push(
            ImportCandidate::add(
                ImportCandidateId::new("candidate:event:1"),
                ImportCandidateItem::Event(TimelineEvent::new(
                    EventId(1),
                    EventTypeId::new("custom.event"),
                    "Possible dupe",
                )),
            )
            .conflict("possible duplicate event"),
        );

        assert_eq!(batch.conflict_count(), 1);
        assert_eq!(batch.accepted_count(), 0);
    }
}
