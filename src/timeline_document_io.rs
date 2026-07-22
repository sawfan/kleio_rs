//! Import/export helpers for `TimelineDocument`.
//!
//! Packs are the main exchange unit, but a document captures which packs are
//! attached and active for a working timeline context.

use crate::pack::TimelineDocument;
use crate::pack_samples::sample_timeline_packs;

pub fn timeline_document_to_json_pretty(
    document: &TimelineDocument,
) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(document)
}

pub fn timeline_document_from_json(json: &str) -> Result<TimelineDocument, serde_json::Error> {
    serde_json::from_str(json)
}

pub fn timeline_document_to_toml_pretty(
    document: &TimelineDocument,
) -> Result<String, toml::ser::Error> {
    toml::to_string_pretty(document)
}

pub fn timeline_document_from_toml(toml_text: &str) -> Result<TimelineDocument, toml::de::Error> {
    toml::from_str(toml_text)
}

pub fn sample_timeline_document() -> TimelineDocument {
    let mut document = TimelineDocument::empty();
    for pack in sample_timeline_packs() {
        document.add_pack(pack, true);
    }
    document
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PersonId, YearSpan};

    #[test]
    fn sample_timeline_document_contains_active_sample_packs() {
        let document = sample_timeline_document();

        assert_eq!(document.active_packs().count(), 4);
        assert!(
            !document
                .active_events_in_year_span(YearSpan::exact(1942))
                .is_empty()
        );
        assert!(!document.active_events_for_entity(PersonId(1)).is_empty());
    }

    #[test]
    fn timeline_document_json_round_trips() {
        let document = sample_timeline_document();
        let json = timeline_document_to_json_pretty(&document).expect("serialize document json");
        let parsed = timeline_document_from_json(&json).expect("parse document json");

        assert_eq!(parsed.version, TimelineDocument::CURRENT_VERSION);
        assert_eq!(parsed.packs.len(), 4);
        assert_eq!(parsed.active_pack_ids.len(), 4);
        assert!(
            parsed
                .active_event_collection(&crate::EventCollectionId::new(
                    "collection:sample-biography-sequence"
                ))
                .is_some()
        );
    }

    #[test]
    fn timeline_document_toml_round_trips() {
        let document = sample_timeline_document();
        let toml_text =
            timeline_document_to_toml_pretty(&document).expect("serialize document toml");
        let parsed = timeline_document_from_toml(&toml_text).expect("parse document toml");

        assert_eq!(parsed.version, TimelineDocument::CURRENT_VERSION);
        assert_eq!(parsed.packs.len(), 4);
        assert_eq!(parsed.active_pack_ids.len(), 4);
        assert!(
            parsed
                .active_event_collection(&crate::EventCollectionId::new(
                    "collection:sample-biography-sequence"
                ))
                .is_some()
        );
    }
}
