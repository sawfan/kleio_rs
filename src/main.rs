use std::process::ExitCode;

#[cfg(feature = "sqlite")]
mod sqlite_example {
    use std::{env, fs};

    pub fn run() -> Result<(), kleio::db::DbError> {
        let mut args = env::args().skip(1);
        let Some(gedcom_path) = args.next() else {
            eprintln!(
                "Usage: cargo run -p kleio --features sqlite -- <path-to-file.ged> [project-name]\n\n\
Creates or opens kleio.sqlite, initializes the schema, creates a project, imports the GEDCOM, and prints the import id/hash.\n\n\
Kleio CLI local authoring:\n    cargo run -p kleio-cli_rs --bin kleio-cli -- init"
            );
            return Ok(());
        };
        let project_name = args.next().unwrap_or_else(|| "Kleio Project".to_string());
        run_sqlite_example(&gedcom_path, &project_name)
    }

    fn run_sqlite_example(gedcom_path: &str, project_name: &str) -> Result<(), kleio::db::DbError> {
        let mut conn = kleio::db::open_database("kleio.sqlite")?;
        kleio::db::init_schema(&conn)?;

        let project = kleio::db::create_project(&conn, project_name)?;
        let gedcom_text = fs::read_to_string(gedcom_path)?;
        let filename = std::path::Path::new(gedcom_path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("import.ged");
        let import = kleio::db::import_gedcom_file(&mut conn, &project.id, filename, &gedcom_text)?;

        println!("project_id={}", project.id);
        println!("gedcom_import_id={}", import.id);
        println!("gedcom_file_hash={}", import.file_hash);

        Ok(())
    }
}

#[cfg(feature = "sqlite")]
fn main() -> ExitCode {
    match sqlite_example::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("kleio sqlite import failed: {err}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(not(feature = "sqlite"))]
fn main() -> ExitCode {
    eprintln!(
        "Kleio is primarily a library crate.\n\n\
Kleio CLI local authoring:\n    cargo run -p kleio-cli_rs --bin kleio-cli -- init\n\n\
SQLite GEDCOM import example:\n    cargo run -p kleio --features sqlite -- path/to/family.ged [project-name]\n\n\
Experimental Wikidata bzip2 importer:\n    cargo run -p kleio --example wikidata_import -- --help"
    );
    ExitCode::SUCCESS
}
