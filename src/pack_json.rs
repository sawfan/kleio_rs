//! JSON import/export helpers for `EventPack`.
//!
//! This is a deliberately small adapter: it parses a serialized pack, preserves
//! the raw JSON as an import record, and exposes the pack contents as accepted
//! import candidates. UI code can still preview/reject candidates before
//! materializing the pack.

use crate::import_batch::{ImportBatch, ImportSourceKind};
use crate::pack::{EventPack, PackKind, PackMetadata};
use crate::pack_import::import_batch_from_event_pack;

#[derive(Debug, Clone, PartialEq)]
pub struct ImportedEventPack {
    pub metadata: PackMetadata,
    pub kind: PackKind,
    pub batch: ImportBatch,
}

impl ImportedEventPack {
    pub fn materialize(self) -> EventPack {
        self.batch.materialize_event_pack(self.metadata, self.kind)
    }
}

pub fn event_pack_to_json_pretty(pack: &EventPack) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(pack)
}

pub fn event_pack_from_json(json: &str) -> Result<EventPack, serde_json::Error> {
    serde_json::from_str(json)
}

pub fn import_event_pack_json(
    source_name: impl Into<String>,
    json: &str,
) -> Result<ImportedEventPack, serde_json::Error> {
    let source_name = source_name.into();
    let pack = event_pack_from_json(json)?;
    let batch = import_batch_from_event_pack(
        source_name,
        ImportSourceKind::Json,
        "record:json:0",
        Some(json.to_string()),
        &pack,
    );

    Ok(ImportedEventPack {
        metadata: pack.metadata,
        kind: pack.kind,
        batch,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EventId, EventTypeId, PackId, TimeSpec, TimelineEvent};

    #[test]
    fn event_pack_json_round_trips_and_imports_as_candidates() {
        let mut pack = EventPack::empty(
            PackMetadata::new(PackId::new("pack:journal"), "Journal Pack"),
            PackKind::UserJournal,
        );
        pack.events.push(
            TimelineEvent::new(EventId(1), EventTypeId::new("journal.entry"), "First entry")
                .with_time(TimeSpec::OriginalOnly {
                    original: "2026-07-06".to_string(),
                }),
        );

        let json = event_pack_to_json_pretty(&pack).expect("serialize event pack");
        let imported = import_event_pack_json("journal.json", &json).expect("import event pack");

        assert_eq!(imported.metadata.title, "Journal Pack");
        assert_eq!(imported.batch.records.len(), 1);
        assert_eq!(imported.batch.accepted_count(), 1);

        let materialized = imported.materialize();
        assert_eq!(materialized.kind, PackKind::UserJournal);
        assert_eq!(materialized.events.len(), 1);
        assert_eq!(materialized.events[0].id, EventId(1));
    }

    #[test]
    fn json_import_uses_shared_stable_import_key() {
        let mut pack = EventPack::empty(
            PackMetadata::new(PackId::new("pack:journal"), "Journal Pack"),
            PackKind::UserJournal,
        );
        pack.events.push(TimelineEvent::new(
            EventId(1),
            EventTypeId::new("journal.entry"),
            "First entry",
        ));

        let json = event_pack_to_json_pretty(&pack).expect("serialize event pack");
        let imported =
            import_event_pack_json("Journal Pack.json", &json).expect("import event pack");

        assert_eq!(imported.batch.id.as_str(), "import:json:journal-pack-json");
    }
}
