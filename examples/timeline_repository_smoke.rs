use std::error::Error;
use std::path::PathBuf;

use kleio::{
    TimelineDocumentRepository, file::JsonTimelineDocumentFileRepository,
    file::TomlTimelineDocumentFileRepository, sample_timeline_document,
};

fn main() -> Result<(), Box<dyn Error>> {
    let output_dir = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/kleio-timeline-repository-smoke"));
    std::fs::create_dir_all(&output_dir)?;

    let document = sample_timeline_document();

    let json_repo =
        JsonTimelineDocumentFileRepository::new(output_dir.join("timeline-document.json"));
    json_repo.save(&document)?;
    let json_loaded = json_repo.load()?;

    let toml_repo =
        TomlTimelineDocumentFileRepository::new(output_dir.join("timeline-document.toml"));
    toml_repo.save(&document)?;
    let toml_loaded = toml_repo.load()?;

    println!("json_path={}", json_repo.path().display());
    println!("json_packs={}", json_loaded.packs.len());
    println!("json_active_packs={}", json_loaded.active_pack_ids.len());
    println!("toml_path={}", toml_repo.path().display());
    println!("toml_packs={}", toml_loaded.packs.len());
    println!("toml_active_packs={}", toml_loaded.active_pack_ids.len());

    Ok(())
}
