use std::process::ExitCode;

fn main() -> ExitCode {
    eprintln!(
        "kleio: the experimental Wikidata bzip2 importer is development-only.\n\n\
Run it with:\n    cargo run -p kleio --example wikidata_import -- import wikidata-truthy [OPTIONS]\n\n\
Or run:\n    cargo run -p kleio --example wikidata_import -- --help"
    );
    ExitCode::SUCCESS
}
