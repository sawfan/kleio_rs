# kleio (kleio_rs)

Source-agnostic people and event primitives for Rust.

`kleio` is intended to be a **core model crate** that multiple importers,
exporters, and applications can target. In this workspace, Ourania can use it as
the shared representation for people, life events, places, families, notes, and
provenance.

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

## Experimental Wikidata truthy import

`kleio` includes a small experimental ETL command for sampling a bounded slice of
Wikidata's truthy N-Triples dump into a compact newline-delimited JSON
intermediate format. It streams directly from `vendor/latest-truthy.nt.bz2`; do
not decompress the full dump to disk.

The importer currently keeps only a small whitelist of person/genealogy-adjacent
properties (`P31`, `P569`, `P570`, `P19`, `P20`, `P22`, `P25`, `P26`, `P40`,
`P3373`, `P735`, `P734`, `P106`, `P21`, `P27`, `P625`). Human detection is based
on `P31 = Q5`, but facts are written as an intermediate source model rather than
being merged directly into Kleio's permanent genealogy structs.

Safe defaults are bounded (`--max-lines 100000`, `--max-facts 10000`) and write
to `target/wikidata-sample.ndjson`.

Examples:

- Import the first 1 million decompressed lines:

  `cargo run -p kleio -- import wikidata-truthy --dump-path vendor/latest-truthy.nt.bz2 --max-lines 1000000 --progress-every 100000`

- Stop after 10,000 relevant facts:

  `cargo run -p kleio -- import wikidata-truthy --max-facts 10000`

- Sample facts for one subject while scanning a bounded prefix of the dump:

  `cargo run -p kleio -- import wikidata-truthy --subject Q42 --max-lines 5000000`

  If you are sampling one subject from the subject-grouped truthy dump, you can
  usually stop as soon as the first later relevant subject is seen:

  `cargo run -p kleio -- import wikidata-truthy --subject Q42 --stop-after-subject --max-lines 5000000`

- Build a one-hop closure from a sampled fact set. This re-streams the dump and
  imports relevant facts whose subjects are either original subjects or QID
  entity values referenced by the seed file:

  `cargo run -p kleio -- import wikidata-closure --seed-path target/wikidata-sample.ndjson --max-lines 1000000`

- Generate a sorted QID seed list for building a small external label cache:

  `cargo run -p kleio -- import wikidata-label-seeds --input-path target/wikidata-closure.ndjson`

  Draft generation can then apply an optional JSON label cache shaped like
  `{ "Q42": "Douglas Adams", "Q350": "Cambridge" }`:

  `cargo run -p kleio -- import wikidata-drafts --input-path target/wikidata-closure.ndjson --label-cache target/wikidata-labels.json`

- Build experimental Kleio-oriented person drafts from sampled facts:

  `cargo run -p kleio -- import wikidata-drafts --input-path target/wikidata-sample.ndjson`

- Summarize draft completeness before building an archive:

  `cargo run -p kleio -- import wikidata-drafts-summary --input-path target/wikidata-person-drafts.ndjson`

- Convert draft NDJSON into a tiny experimental Kleio `.rkyv` archive:

  `cargo run -p kleio -- import wikidata-kleio --input-path target/wikidata-person-drafts.ndjson`

  This prototype projection creates people, birth/death/occupation events,
  minimal parent/spouse families when the related person is present in the same
  draft set, and places for birth/death place QIDs. It preserves Wikidata QIDs in
  provenance rather than treating the result as authoritative genealogy data.

- Inspect/validate the generated archive:

  `cargo run -p kleio -- import wikidata-kleio-inspect --path target/wikidata-kleio.rkyv`

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
for people/events data that Ourania and external importers can share.
