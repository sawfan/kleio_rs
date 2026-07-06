//! TOML import/export helpers for `EventPack`.
//!
//! TOML is useful for smaller hand-authored packs. Like the JSON adapter, this
//! parses a serialized pack into an `ImportBatch` so UI code can preview the
//! normalized candidates before materializing them.

use crate::import_batch::{ImportBatch, ImportSourceKind};
use crate::pack::{EventPack, PackKind, PackMetadata};
use crate::pack_import::import_batch_from_event_pack;

#[derive(Debug, Clone, PartialEq)]
pub struct ImportedTomlEventPack {
    pub metadata: PackMetadata,
    pub kind: PackKind,
    pub batch: ImportBatch,
}

impl ImportedTomlEventPack {
    pub fn materialize(self) -> EventPack {
        self.batch.materialize_event_pack(self.metadata, self.kind)
    }
}

pub fn event_pack_to_toml_pretty(pack: &EventPack) -> Result<String, toml::ser::Error> {
    toml::to_string_pretty(pack)
}

pub fn event_pack_from_toml(toml_text: &str) -> Result<EventPack, toml::de::Error> {
    toml::from_str(toml_text)
}

pub fn import_event_pack_toml(
    source_name: impl Into<String>,
    toml_text: &str,
) -> Result<ImportedTomlEventPack, toml::de::Error> {
    let source_name = source_name.into();
    let pack = event_pack_from_toml(toml_text)?;
    let batch = import_batch_from_event_pack(
        source_name,
        ImportSourceKind::Toml,
        "record:toml:0",
        Some(toml_text.to_string()),
        &pack,
    );

    Ok(ImportedTomlEventPack {
        metadata: pack.metadata,
        kind: pack.kind,
        batch,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EventId, EventTypeId, PackId, TimelineEvent};

    #[test]
    fn event_pack_toml_round_trips_and_imports_as_candidates() {
        let mut pack = EventPack::empty(
            PackMetadata::new(PackId::new("pack:small-history"), "Small History"),
            PackKind::HistoricalTimeline,
        );
        pack.events.push(TimelineEvent::new(
            EventId(1),
            EventTypeId::new("history.event"),
            "A hand-authored event",
        ));

        let toml_text = event_pack_to_toml_pretty(&pack).expect("serialize event pack toml");
        let imported = import_event_pack_toml("small-history.toml", &toml_text)
            .expect("import event pack toml");

        assert_eq!(imported.metadata.title, "Small History");
        assert_eq!(imported.batch.records.len(), 1);
        assert_eq!(imported.batch.accepted_count(), 1);

        let materialized = imported.materialize();
        assert_eq!(materialized.kind, PackKind::HistoricalTimeline);
        assert_eq!(materialized.events.len(), 1);
        assert_eq!(materialized.events[0].id, EventId(1));
    }
}
