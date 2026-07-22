//! Shared helpers for importing serialized `EventPack` values.
//!
//! Format-specific modules such as `pack_json` and `pack_toml` handle parsing
//! and serialization. This module owns the common conversion from a parsed pack
//! into an `ImportBatch` preview.

use crate::import_batch::{
    ImportBatch, ImportBatchId, ImportCandidate, ImportCandidateId, ImportCandidateItem,
    ImportRecord, ImportSourceKind, ImportStatus,
};
use crate::pack::EventPack;

pub fn import_batch_from_event_pack(
    source_name: impl Into<String>,
    source_kind: ImportSourceKind,
    record_id: impl Into<String>,
    raw: Option<String>,
    pack: &EventPack,
) -> ImportBatch {
    let source_name = source_name.into();
    let mut batch = ImportBatch::new(
        ImportBatchId::new(format!(
            "import:{}:{}",
            source_kind_key(&source_kind),
            stable_import_key(&source_name)
        )),
        source_name.clone(),
        source_kind,
    );
    batch.status = ImportStatus::Previewed;
    batch.provenance = pack.provenance.clone();
    batch.records.push(ImportRecord {
        id: record_id.into(),
        source_record_id: Some(source_name),
        summary: Some(format!("Event pack: {}", pack.metadata.title)),
        raw,
        provenance: pack.provenance.clone(),
    });
    append_event_pack_candidates(&mut batch, pack);
    batch
}

pub fn append_event_pack_candidates(batch: &mut ImportBatch, pack: &EventPack) {
    for (idx, profile) in pack.domain_profiles.iter().cloned().enumerate() {
        batch.candidates.push(
            ImportCandidate::add(
                ImportCandidateId::new(format!("candidate:domain-profile:{idx}")),
                ImportCandidateItem::DomainProfile(profile),
            )
            .accepted(),
        );
    }

    for (idx, entity) in pack.entities.iter().cloned().enumerate() {
        batch.candidates.push(
            ImportCandidate::add(
                ImportCandidateId::new(format!("candidate:entity:{idx}")),
                ImportCandidateItem::Entity(entity),
            )
            .accepted(),
        );
    }

    for (idx, event) in pack.events.iter().cloned().enumerate() {
        batch.candidates.push(
            ImportCandidate::add(
                ImportCandidateId::new(format!("candidate:event:{idx}")),
                ImportCandidateItem::Event(event),
            )
            .accepted(),
        );
    }

    for (idx, collection) in pack.event_collections.iter().cloned().enumerate() {
        batch.candidates.push(
            ImportCandidate::add(
                ImportCandidateId::new(format!("candidate:event-collection:{idx}")),
                ImportCandidateItem::EventCollection(collection),
            )
            .accepted(),
        );
    }

    for (idx, relation) in pack.event_relations.iter().cloned().enumerate() {
        batch.candidates.push(
            ImportCandidate::add(
                ImportCandidateId::new(format!("candidate:event-relation:{idx}")),
                ImportCandidateItem::EventRelation(relation),
            )
            .accepted(),
        );
    }

    for (idx, source) in pack.sources.iter().cloned().enumerate() {
        batch.candidates.push(
            ImportCandidate::add(
                ImportCandidateId::new(format!("candidate:source:{idx}")),
                ImportCandidateItem::Source(source),
            )
            .accepted(),
        );
    }

    for (idx, tag) in pack.tags.iter().cloned().enumerate() {
        batch.candidates.push(
            ImportCandidate::add(
                ImportCandidateId::new(format!("candidate:tag:{idx}")),
                ImportCandidateItem::TagValue(tag),
            )
            .accepted(),
        );
    }
}

pub fn stable_import_key(value: &str) -> String {
    let key: String = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let key = key.trim_matches('-');
    if key.is_empty() {
        "event-pack".to_string()
    } else {
        key.to_string()
    }
}

fn source_kind_key(source_kind: &ImportSourceKind) -> &str {
    match source_kind {
        ImportSourceKind::Gedcom => "gedcom",
        ImportSourceKind::Csv => "csv",
        ImportSourceKind::Json => "json",
        ImportSourceKind::Toml => "toml",
        ImportSourceKind::Ics => "ics",
        ImportSourceKind::Markdown => "markdown",
        ImportSourceKind::Wikidata => "wikidata",
        ImportSourceKind::Manual => "manual",
        ImportSourceKind::Custom(_) => "custom",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        EventCollection, EventCollectionId, EventCollectionKind, EventCollectionMember, EventId,
        EventTypeId, PackId, PackKind, PackMetadata, TimelineEvent,
    };

    #[test]
    fn import_batch_from_pack_extracts_all_candidate_kinds() {
        let mut pack = EventPack::empty(
            PackMetadata::new(PackId::new("pack:test"), "Test Pack"),
            PackKind::HistoricalTimeline,
        );
        pack.events.push(TimelineEvent::new(
            EventId(1),
            EventTypeId::new("history.event"),
            "Event",
        ));
        pack.event_collections.push(
            EventCollection::new(
                EventCollectionId::new("collection:test"),
                "Test Collection",
                EventCollectionKind::Set,
            )
            .with_member(EventCollectionMember::new(EventId(1))),
        );
        let collection_refs = pack.event_collections_for_event(EventId(1));
        assert_eq!(collection_refs.len(), 1);
        assert_eq!(collection_refs[0].id.as_str(), "collection:test");

        let batch = import_batch_from_event_pack(
            "test pack.json",
            ImportSourceKind::Json,
            "record:json:0",
            Some("{}".to_string()),
            &pack,
        );

        assert_eq!(batch.id.as_str(), "import:json:test-pack-json");
        assert_eq!(batch.records.len(), 1);
        assert_eq!(batch.accepted_count(), 2);
    }

    #[test]
    fn stable_import_key_uses_safe_fallback() {
        assert_eq!(stable_import_key("Journal Pack.json"), "journal-pack-json");
        assert_eq!(stable_import_key("---"), "event-pack");
    }
}
