# Kleio SQLite project database

This module is the initial SQLite-backed local project format for Kleio.

Current scope:

- `project` stores project identity and timestamps.
- `gedcom_import` stores raw GEDCOM uploads as immutable datasource rows.
- GEDCOM text is preserved exactly as imported.
- No parsing is performed during insertion yet.

Planned evolution:

- Keep `gedcom_import` immutable and add parsed tables keyed back to an import/source row.
- Add normalized `person`, `family`, `event`, `source`, and `place` tables.
- Add `user_override` for display names, corrected facts, hidden facts, preferred roots, and UI state.
- Add `date_assertion` for GEDCOM-provided, Wikidata-derived, and user-provided date claims.
- Add source/citation tables so assertions can cite GEDCOM records, Wikidata, documents, or user notes.
- Treat the SQLite file itself as the exportable project file when the schema stabilizes.

The module is gated behind the `sqlite` Cargo feature so browser/WASM builds do not pull in SQLite.
