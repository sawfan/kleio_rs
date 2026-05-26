# kleio (kleio_rs)

Source-agnostic genealogy primitives for Rust.

`kleio` is intended to be a **core model crate** that multiple importers/exporters can target.
In this workspace, `adbimport` becomes an adapter that converts Astrodatabank XML into `kleio` types.

## Goals

- Provide a **normalized people/events/families/places/notes** model suitable for:
  - Astrodatabank imports (including astro-specific computed/recorded fields)
  - GEDCOM 7 import/export
  - other sources (custom apps, APIs, etc.)
- Preserve as much upstream data as possible without hard-tying the core to any one source.
- Enable **fast load times** via `rkyv` archived snapshots (`GenealogyArchive`) with a runtime wrapper (`GenealogyStore`).

## What lives where

- `kleio`:
  - Core structs: `Person`, `Event`, `Family`, `Place`, `Note`
  - IDs: `PersonId`, `EventId`, ...
  - Generic provenance: `Provenance`, `Tag`, `Attribute`, `Citation`, `SourceRef`
  - Derived indexes + archived snapshot types: `SearchIndexArchive`, `DateIndexArchive`, `GenealogyArchive`
  - Runtime access wrapper over archived bytes: `GenealogyStore`

- `adbimport`:
  - ADB XML serde structs (`PublicData`, `ResearchData`, etc.)
  - Conversion/parsing logic (`parse_astrodatabank`) that produces `kleio` records

## Notes on flexibility / lossless import

Real-world genealogy data has:
- multiple competing assertions (two birth times)
- varying confidence / evidence
- source-specific classifications (e.g. ADB categories, Rodden rating)

The core approach in `kleio` is:
- keep common genealogical concepts first-class (Birth/Death/Marriage/Baptism/etc.)
- keep uncommon or source-specific concepts as `EventKind::Other(String)`
- attach extra source-specific metadata as generic `Provenance` (attributes/tags/citations)

### ADB specifics

ADB fields like `roddenrating`, `datatype`, `categories`, etc. are expected to be retained via `Provenance.attributes` / `Provenance.tags` rather than requiring ADB-specific struct fields in the core model.

ADB’s `positions` are modeled as `Event.positions: Option<AstroPositions>` on the relevant event (typically Birth). This keeps the model useful for non-astrology sources while still supporting ADB.

## GEDCOM 7 (planned)

A future `kleio_gedcom7` (or similar) crate can:
- parse GEDCOM 7 into `kleio` (preserving original IDs in `SourceRef`)
- emit GEDCOM 7 from `kleio`
- maintain round-trip safety using:
  - `SourceRef` for original record identifiers
  - `Attribute`/`Tag` for extensions
  - `Citation` for evidence pointers

## Status

This crate is under active development. The current focus is establishing the core types and ensuring `adbimport` can translate ADB XML into `kleio` archives.

