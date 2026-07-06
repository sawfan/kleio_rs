use std::error::Error;
use std::fs;
use std::path::PathBuf;

use kleio::{
    sample_timeline_document, timeline_document_to_json_pretty, timeline_document_to_toml_pretty,
};

fn main() -> Result<(), Box<dyn Error>> {
    let output_dir = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/kleio-sample-timeline-document"));
    fs::create_dir_all(&output_dir)?;

    let document = sample_timeline_document();
    let json_path = output_dir.join("timeline-document.json");
    let toml_path = output_dir.join("timeline-document.toml");

    fs::write(&json_path, timeline_document_to_json_pretty(&document)?)?;
    fs::write(&toml_path, timeline_document_to_toml_pretty(&document)?)?;

    println!("wrote {}", json_path.display());
    println!("wrote {}", toml_path.display());

    Ok(())
}
