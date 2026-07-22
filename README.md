# kleio (kleio_rs)

Source-agnostic people and event primitives for Rust.

`kleio` is intended to be a **core model crate** that multiple importers,
exporters, and applications can target. It provides shared representations for
people, life events, places, families, notes, provenance, and local authoring
workflows without depending on any downstream application.

## Goals

- Provide a **normalized people/events/families/places/notes** model suitable for:
  - application-owned people and event records
  - GEDCOM 7 import/export
  - other importers, APIs, and archival formats
- Preserve as much upstream data as possible without hard-tying the core to any one source.
- Enable **fast load times** via `rkyv` archived snapshots (`GenealogyArchive`) with a runtime wrapper (`GenealogyStore`).

## What lives where

- `kleio`:
  - Core structs: `Person`, `Event`, `Family`, `Place`, `Note`
  - IDs: `PersonId`, `EventId`, ...
  - Generic provenance: `Provenance`, `Tag`, `Attribute`, `Citation`, `SourceRef`
  - Derived indexes + archived snapshot types: `SearchIndexArchive`, `DateIndexArchive`, `GenealogyArchive`
  - Runtime access wrapper over archived bytes: `GenealogyStore`

- Importer crates:
  - Parse source-specific formats.
  - Convert them into `kleio` records.
  - Preserve source-specific values through `Provenance.attributes`, `Provenance.tags`, `SourceRef`, and `Citation`.

## Notes on flexibility / lossless import

Real-world person and event data has:
- multiple competing assertions (for example, two possible birth times)
- varying confidence / evidence
- source-specific classifications and fields

The core approach in `kleio` is:
- keep common concepts first-class (Birth/Death/Marriage/Baptism/etc.)
- keep uncommon or source-specific concepts as `EventKind::Other(String)`
- attach extra source-specific metadata as generic `Provenance` (attributes/tags/citations)

## Private Kleio data authoring

Kleio local authoring uses a workspace/world layout. A workspace contains one or
more worlds; each world owns semantic records (entities, events, assertions,
sources, imports, schemas) and saved views (timelines, trees, maps, calendars,
visualizations). `kleio-cli` defaults to the standard XDG data location
(`$XDG_DATA_HOME/kleio`, usually `~/.local/share/kleio`) and accepts an explicit
workspace root for scratch/local development.

- Markdown records with TOML frontmatter for prose-heavy entity/event/assertion/source records.
- Plain TOML files for workspace/world config, saved views, schemas, vocabularies, and import reports.
- Generated JSON under `worlds/<world>/build/`.
- Raw import artifacts under `worlds/<world>/imports/`.

Create or compile a workspace with:

```bash
cargo run -p kleio-cli_rs --bin kleio-cli -- init-workspace
cargo run -p kleio-cli_rs --bin kleio-cli -- compile
cargo run -p kleio-cli_rs --bin kleio-cli -- compile-ecs
cargo run -p kleio-cli_rs --bin kleio-cli -- compile-tree --view main-family-tree
```

For repo-local scratch testing:

```bash
cargo run -p kleio-cli_rs --bin kleio-cli -- init-workspace crates/kleio-cli/.kleio-data
```

See `docs/kleio-data-authoring.md` in the workspace root for the current file
shapes. SQLite output remains a future-compatible target for now.

Documentation examples must stay fictional: use IDs such as
`person:alex-example`, `person:morgan-example`, `place:example-place`, and dates
such as `1900-01-01`; do not use real personal names, real birth dates, or real
family examples.

## Experimental Wikidata truthy import

Wikidata import support has moved to the dedicated `kleio-wikidata` crate.
Run the importer with:

```sh
cargo run -p kleio-wikidata -- --help
```

## GEDCOM 7 (planned)

A future `kleio_gedcom7` (or similar) crate can:
- parse GEDCOM 7 into `kleio` (preserving original IDs in `SourceRef`)
- emit GEDCOM 7 from `kleio`
- maintain round-trip safety using:
  - `SourceRef` for original record identifiers
  - `Attribute`/`Tag` for extensions
  - `Citation` for evidence pointers

## Status

This crate is under active development. The current focus is establishing the core types
for people/events data that applications and external importers can share.
