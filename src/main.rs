use std::process::ExitCode;

fn main() -> ExitCode {
    eprintln!(
        "Kleio is primarily a library crate.\n\n\
Kleio CLI local authoring:\n    cargo run -p kleio-cli_rs --bin kleio-cli -- init\n\n\
SQLite GEDCOM import example:\n    cargo run -p kleio-gedcom --features db -- path/to/family.ged [project-name]\n\n\
Experimental Wikidata bzip2 importer:\n    cargo run -p kleio-wikidata -- --help"
    );
    ExitCode::SUCCESS
}
