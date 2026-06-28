use rkyv::{
    api::high::{from_bytes, to_bytes},
    rancor::Error,
};

use crate::model::GenealogyArchive;

#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;

/// Serialize a `GenealogyArchive` to bytes using `rkyv`.
///
/// This is designed for fast load times in local apps (including WASM).
pub fn archive_genealogy_archive(archive: &GenealogyArchive) -> Result<Vec<u8>, Error> {
    Ok(to_bytes::<Error>(archive)?.into_vec())
}

/// Deserialize and validate a `GenealogyArchive` from bytes.
pub fn deserialize_genealogy_archive(bytes: &[u8]) -> Result<GenealogyArchive, Error> {
    from_bytes::<GenealogyArchive, Error>(bytes)
}

/// Validate and view an archived `GenealogyArchive` from bytes.
///
/// Note: This returns a reference into the provided byte slice.
pub fn view_archived_genealogy_archive(
    bytes: &[u8],
) -> Result<&rkyv::Archived<GenealogyArchive>, Error> {
    rkyv::access::<rkyv::Archived<GenealogyArchive>, Error>(bytes)
}

/// Convenience helper for reading and validating a serialized genealogy archive.
#[cfg(not(target_arch = "wasm32"))]
pub fn load_genealogy_index_archive(path: impl AsRef<Path>) -> Result<Vec<u8>, std::io::Error> {
    std::fs::read(path)
}

/// Convenience helper for writing a serialized genealogy archive.
#[cfg(not(target_arch = "wasm32"))]
pub fn save_genealogy_index_archive(
    path: impl AsRef<Path>,
    bytes: &[u8],
) -> Result<(), std::io::Error> {
    std::fs::write(path, bytes)
}
