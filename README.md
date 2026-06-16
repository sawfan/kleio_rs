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
