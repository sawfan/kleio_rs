use std::error::Error;
use std::fs;
use std::path::PathBuf;

use kleio::{
    event_pack_to_json_pretty, event_pack_to_toml_pretty, sample_biography_pack,
    sample_history_pack, sample_journal_pack,
};

fn main() -> Result<(), Box<dyn Error>> {
    let output_dir = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/kleio-sample-packs"));
    fs::create_dir_all(&output_dir)?;

    let packs = [
        ("journal", sample_journal_pack()),
        ("biography", sample_biography_pack()),
        ("history", sample_history_pack()),
    ];

    for (name, pack) in packs {
        let json_path = output_dir.join(format!("{name}.json"));
        let toml_path = output_dir.join(format!("{name}.toml"));
        fs::write(&json_path, event_pack_to_json_pretty(&pack)?)?;
        fs::write(&toml_path, event_pack_to_toml_pretty(&pack)?)?;
        println!("wrote {}", json_path.display());
        println!("wrote {}", toml_path.display());
    }

    Ok(())
}
