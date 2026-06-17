use std::env;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::process::ExitCode;

use bzip2::read::BzDecoder;
use kleio::import::wikidata::{
    DEFAULT_CLOSURE_OUTPUT_PATH, DEFAULT_DRAFT_OUTPUT_PATH, DEFAULT_DUMP_PATH,
    DEFAULT_KLEIO_ARCHIVE_PATH, DEFAULT_LABEL_SEEDS_PATH, DEFAULT_MAX_FACTS, DEFAULT_MAX_LINES,
    DEFAULT_OUTPUT_PATH, DEFAULT_PROGRESS_EVERY, TruthyImportOptions, WikidataClosureOptions,
    WikidataDraftOptions, WikidataKleioOptions, build_person_drafts, inspect_kleio_archive,
    run_truthy_closure_import_from_reader, run_truthy_import_from_reader, summarize_person_drafts,
    write_kleio_archive_from_drafts, write_label_seeds_from_facts,
};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) if err == "help requested" => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("wikidata_import: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    match (args.next().as_deref(), args.next().as_deref()) {
        (Some("import"), Some("wikidata-truthy")) => run_wikidata_truthy(args.collect()),
        (Some("import"), Some("wikidata-closure")) => run_wikidata_closure(args.collect()),
        (Some("import"), Some("wikidata-label-seeds")) => run_wikidata_label_seeds(args.collect()),
        (Some("import"), Some("wikidata-drafts")) => run_wikidata_drafts(args.collect()),
        (Some("import"), Some("wikidata-drafts-summary")) => {
            run_wikidata_drafts_summary(args.collect())
        }
        (Some("import"), Some("wikidata-kleio")) => run_wikidata_kleio(args.collect()),
        (Some("import"), Some("wikidata-kleio-inspect")) => {
            run_wikidata_kleio_inspect(args.collect())
        }
        (Some("help" | "--help" | "-h"), _) | (None, _) => {
            print_help();
            Ok(())
        }
        (Some(other), subcommand) => Err(format!(
            "unknown command `{}`{}\n\nRun `cargo run -p kleio --example wikidata_import -- --help` for usage.",
            other,
            subcommand.map(|s| format!(" {s}")).unwrap_or_default()
        )),
    }
}

fn run_wikidata_truthy(args: Vec<String>) -> Result<(), String> {
    let options = parse_wikidata_truthy_options(args)?;
    let dump = File::open(&options.dump_path).map_err(|err| {
        format!(
            "wikidata-truthy import failed to open dump `{}`: {err}",
            options.dump_path.display()
        )
    })?;
    let decoder = BzDecoder::new(dump);
    let reader = BufReader::with_capacity(1024 * 1024, decoder);
    let report = run_truthy_import_from_reader(reader, &options).map_err(|err| {
        format!(
            "wikidata-truthy import failed for dump `{}`: {err}",
            options.dump_path.display()
        )
    })?;

    eprintln!(
        "wikidata_import: wrote {} relevant Wikidata facts from {} lines to {} (humans_detected={})",
        report.facts_written,
        report.lines_read,
        report.output_path.display(),
        report.humans_detected
    );
    Ok(())
}

fn run_wikidata_closure(args: Vec<String>) -> Result<(), String> {
    let options = parse_wikidata_closure_options(args)?;
    let dump = File::open(&options.dump_path).map_err(|err| {
        format!(
            "wikidata-closure import failed to open dump `{}`: {err}",
            options.dump_path.display()
        )
    })?;
    let decoder = BzDecoder::new(dump);
    let reader = BufReader::with_capacity(1024 * 1024, decoder);
    let report = run_truthy_closure_import_from_reader(reader, &options).map_err(|err| {
        format!(
            "wikidata-closure import failed for dump `{}` and seeds `{}`: {err}",
            options.dump_path.display(),
            options.seed_path.display()
        )
    })?;

    eprintln!(
        "wikidata_import: wrote {} one-hop closure facts from {} lines to {} (seed_qids={}, seed_subjects_seen={}, humans_detected={})",
        report.facts_written,
        report.lines_read,
        report.output_path.display(),
        report.seed_qids,
        report.seed_subjects_seen,
        report.humans_detected
    );
    Ok(())
}

fn run_wikidata_label_seeds(args: Vec<String>) -> Result<(), String> {
    let (input_path, output_path) = parse_wikidata_label_seeds_options(args)?;
    let count = write_label_seeds_from_facts(&input_path, &output_path).map_err(|err| {
        format!(
            "wikidata-label-seeds failed for input `{}`: {err}",
            input_path.display()
        )
    })?;

    eprintln!(
        "wikidata_import: wrote {count} label seed QIDs from {} to {}",
        input_path.display(),
        output_path.display()
    );
    Ok(())
}

fn run_wikidata_drafts(args: Vec<String>) -> Result<(), String> {
    let options = parse_wikidata_draft_options(args)?;
    let report = build_person_drafts(&options).map_err(|err| {
        format!(
            "wikidata-drafts failed for input `{}`: {err}",
            options.input_path.display()
        )
    })?;

    eprintln!(
        "wikidata_import: wrote {} Wikidata person drafts from {} facts to {} (humans_written={}, labels_loaded={})",
        report.drafts_written,
        report.facts_read,
        report.output_path.display(),
        report.humans_written,
        report.labels_loaded
    );
    Ok(())
}

fn run_wikidata_kleio(args: Vec<String>) -> Result<(), String> {
    let options = parse_wikidata_kleio_options(args)?;
    let report = write_kleio_archive_from_drafts(&options).map_err(|err| {
        format!(
            "wikidata-kleio failed for input `{}`: {err}",
            options.input_path.display()
        )
    })?;

    eprintln!(
        "wikidata_import: wrote archive {} from {} drafts (people={}, events={}, families={}, places={})",
        report.output_path.display(),
        report.drafts_read,
        report.people_written,
        report.events_written,
        report.families_written,
        report.places_written
    );
    Ok(())
}

fn run_wikidata_kleio_inspect(args: Vec<String>) -> Result<(), String> {
    let path = parse_wikidata_kleio_inspect_options(args)?;
    let report = inspect_kleio_archive(&path).map_err(|err| {
        format!(
            "wikidata-kleio-inspect failed for `{}`: {err}",
            path.display()
        )
    })?;

    eprintln!(
        "wikidata_import: inspected {} (people={}, events={}, families={}, places={}, notes={})",
        report.path.display(),
        report.people,
        report.events,
        report.families,
        report.places,
        report.notes
    );
    Ok(())
}

fn run_wikidata_drafts_summary(args: Vec<String>) -> Result<(), String> {
    let (path, limit) = parse_wikidata_drafts_summary_options(args)?;
    let report = summarize_person_drafts(&path, limit).map_err(|err| {
        format!(
            "wikidata-drafts-summary failed for `{}`: {err}",
            path.display()
        )
    })?;

    eprintln!(
        "wikidata_import: summarized {} (drafts_read={}, humans={})",
        report.input_path.display(),
        report.drafts_read,
        report.humans
    );
    Ok(())
}

fn parse_wikidata_truthy_options(args: Vec<String>) -> Result<TruthyImportOptions, String> {
    let mut options = TruthyImportOptions::default();
    let mut iter = args.into_iter();

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--dump-path" => {
                options.dump_path = PathBuf::from(next_value(&mut iter, "--dump-path")?);
            }
            "--output-path" => {
                options.output_path = PathBuf::from(next_value(&mut iter, "--output-path")?);
            }
            "--max-lines" => {
                options.max_lines = parse_u64_flag(&mut iter, "--max-lines")?;
            }
            "--max-facts" => {
                options.max_facts = parse_u64_flag(&mut iter, "--max-facts")?;
            }
            "--progress-every" => {
                options.progress_every = parse_u64_flag(&mut iter, "--progress-every")?;
            }
            "--subject" => {
                let subject = next_value(&mut iter, "--subject")?;
                options.subject = Some(normalize_qid(&subject)?);
            }
            "--stop-after-subject" => {
                options.stop_after_subject = true;
            }
            "--help" | "-h" => {
                print_wikidata_truthy_help();
                return Err("help requested".to_string());
            }
            unknown => return Err(format!("unknown wikidata-truthy flag `{unknown}`")),
        }
    }

    if options.stop_after_subject && options.subject.is_none() {
        return Err("--stop-after-subject requires --subject <QID>".to_string());
    }

    Ok(options)
}

fn parse_wikidata_closure_options(args: Vec<String>) -> Result<WikidataClosureOptions, String> {
    let mut options = WikidataClosureOptions::default();
    let mut iter = args.into_iter();

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--dump-path" => {
                options.dump_path = PathBuf::from(next_value(&mut iter, "--dump-path")?);
            }
            "--seed-path" => {
                options.seed_path = PathBuf::from(next_value(&mut iter, "--seed-path")?);
            }
            "--output-path" => {
                options.output_path = PathBuf::from(next_value(&mut iter, "--output-path")?);
            }
            "--max-lines" => {
                options.max_lines = parse_u64_flag(&mut iter, "--max-lines")?;
            }
            "--max-facts" => {
                options.max_facts = parse_u64_flag(&mut iter, "--max-facts")?;
            }
            "--progress-every" => {
                options.progress_every = parse_u64_flag(&mut iter, "--progress-every")?;
            }
            "--help" | "-h" => {
                print_wikidata_closure_help();
                return Err("help requested".to_string());
            }
            unknown => return Err(format!("unknown wikidata-closure flag `{unknown}`")),
        }
    }

    Ok(options)
}

fn parse_wikidata_draft_options(args: Vec<String>) -> Result<WikidataDraftOptions, String> {
    let mut options = WikidataDraftOptions::default();
    let mut iter = args.into_iter();

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--input-path" => {
                options.input_path = PathBuf::from(next_value(&mut iter, "--input-path")?);
            }
            "--output-path" => {
                options.output_path = PathBuf::from(next_value(&mut iter, "--output-path")?);
            }
            "--label-cache" => {
                options.label_cache_path =
                    Some(PathBuf::from(next_value(&mut iter, "--label-cache")?));
            }
            "--help" | "-h" => {
                print_wikidata_drafts_help();
                return Err("help requested".to_string());
            }
            unknown => return Err(format!("unknown wikidata-drafts flag `{unknown}`")),
        }
    }

    Ok(options)
}

fn parse_wikidata_label_seeds_options(args: Vec<String>) -> Result<(PathBuf, PathBuf), String> {
    let mut input_path = PathBuf::from(DEFAULT_OUTPUT_PATH);
    let mut output_path = PathBuf::from(DEFAULT_LABEL_SEEDS_PATH);
    let mut iter = args.into_iter();

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--input-path" => {
                input_path = PathBuf::from(next_value(&mut iter, "--input-path")?);
            }
            "--output-path" => {
                output_path = PathBuf::from(next_value(&mut iter, "--output-path")?);
            }
            "--help" | "-h" => {
                print_wikidata_label_seeds_help();
                return Err("help requested".to_string());
            }
            unknown => return Err(format!("unknown wikidata-label-seeds flag `{unknown}`")),
        }
    }

    Ok((input_path, output_path))
}

fn parse_wikidata_kleio_options(args: Vec<String>) -> Result<WikidataKleioOptions, String> {
    let mut options = WikidataKleioOptions::default();
    let mut iter = args.into_iter();

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--input-path" => {
                options.input_path = PathBuf::from(next_value(&mut iter, "--input-path")?);
            }
            "--output-path" => {
                options.output_path = PathBuf::from(next_value(&mut iter, "--output-path")?);
            }
            "--include-non-humans" => {
                options.include_non_humans = true;
            }
            "--help" | "-h" => {
                print_wikidata_kleio_help();
                return Err("help requested".to_string());
            }
            unknown => return Err(format!("unknown wikidata-kleio flag `{unknown}`")),
        }
    }

    Ok(options)
}

fn parse_wikidata_kleio_inspect_options(args: Vec<String>) -> Result<PathBuf, String> {
    let mut path = PathBuf::from(DEFAULT_KLEIO_ARCHIVE_PATH);
    let mut iter = args.into_iter();

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--path" => {
                path = PathBuf::from(next_value(&mut iter, "--path")?);
            }
            "--help" | "-h" => {
                print_wikidata_kleio_inspect_help();
                return Err("help requested".to_string());
            }
            unknown => return Err(format!("unknown wikidata-kleio-inspect flag `{unknown}`")),
        }
    }

    Ok(path)
}

fn parse_wikidata_drafts_summary_options(args: Vec<String>) -> Result<(PathBuf, usize), String> {
    let mut path = PathBuf::from(DEFAULT_DRAFT_OUTPUT_PATH);
    let mut limit = 5_usize;
    let mut iter = args.into_iter();

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--input-path" => {
                path = PathBuf::from(next_value(&mut iter, "--input-path")?);
            }
            "--limit" => {
                let value = next_value(&mut iter, "--limit")?;
                limit = value
                    .parse::<usize>()
                    .map_err(|_| format!("invalid integer for `--limit`: `{value}`"))?;
            }
            "--help" | "-h" => {
                print_wikidata_drafts_summary_help();
                return Err("help requested".to_string());
            }
            unknown => return Err(format!("unknown wikidata-drafts-summary flag `{unknown}`")),
        }
    }

    Ok((path, limit))
}

fn next_value(iter: &mut impl Iterator<Item = String>, flag: &str) -> Result<String, String> {
    iter.next()
        .ok_or_else(|| format!("missing value for `{flag}`"))
}

fn parse_u64_flag(iter: &mut impl Iterator<Item = String>, flag: &str) -> Result<u64, String> {
    let value = next_value(iter, flag)?;
    value
        .parse::<u64>()
        .map_err(|_| format!("invalid integer for `{flag}`: `{value}`"))
}

fn normalize_qid(value: &str) -> Result<String, String> {
    let qid = value.trim();
    if qid
        .strip_prefix('Q')
        .is_some_and(|digits| !digits.is_empty() && digits.chars().all(|ch| ch.is_ascii_digit()))
    {
        Ok(qid.to_string())
    } else {
        Err(format!(
            "--subject expects a Wikidata QID like Q42, got `{value}`"
        ))
    }
}

fn print_help() {
    println!(
        "wikidata_import\n\nUSAGE:\n    cargo run -p kleio --example wikidata_import -- import wikidata-truthy [OPTIONS]\n    cargo run -p kleio --example wikidata_import -- import wikidata-closure [OPTIONS]\n    cargo run -p kleio --example wikidata_import -- import wikidata-label-seeds [OPTIONS]\n    cargo run -p kleio --example wikidata_import -- import wikidata-drafts [OPTIONS]\n    cargo run -p kleio --example wikidata_import -- import wikidata-drafts-summary [OPTIONS]\n    cargo run -p kleio --example wikidata_import -- import wikidata-kleio [OPTIONS]\n    cargo run -p kleio --example wikidata_import -- import wikidata-kleio-inspect [OPTIONS]\n\nRun `cargo run -p kleio --example wikidata_import -- import <command> --help` for importer options.\n\nThis development-only example uses kleio's dev-dependencies for bzip2 decompression and is not part of the released product binary."
    );
}

fn print_wikidata_closure_help() {
    println!(
        "wikidata_import import wikidata-closure\n\n\
Experimental one-hop closure import. Reads seed fact NDJSON, collects subject and entity-value QIDs, then streams the truthy dump for those subjects.\n\n\
USAGE:\n    cargo run -p kleio --example wikidata_import -- import wikidata-closure [OPTIONS]\n\n\
OPTIONS:\n    --dump-path <PATH>        Dump path [default: {DEFAULT_DUMP_PATH}]\n    --seed-path <PATH>        Seed fact NDJSON path [default: {DEFAULT_OUTPUT_PATH}]\n    --output-path <PATH>      Closure fact NDJSON output path [default: {DEFAULT_CLOSURE_OUTPUT_PATH}]\n    --max-lines <N>           Stop after N decompressed lines [default: {DEFAULT_MAX_LINES}]\n    --max-facts <N>           Stop after N relevant facts [default: {DEFAULT_MAX_FACTS}]\n    --progress-every <N>      Print progress every N lines; 0 disables [default: {DEFAULT_PROGRESS_EVERY}]\n\n\
EXAMPLES:\n    cargo run -p kleio --example wikidata_import -- import wikidata-closure --seed-path target/wikidata-sample.ndjson --max-lines 1000000\n"
    );
}

fn print_wikidata_label_seeds_help() {
    println!(
        "wikidata_import import wikidata-label-seeds\n\n\
Write a sorted list of QIDs referenced by a Wikidata fact NDJSON file. This is useful for building a small external label cache.\n\n\
USAGE:\n    cargo run -p kleio --example wikidata_import -- import wikidata-label-seeds [OPTIONS]\n\n\
OPTIONS:\n    --input-path <PATH>       Input fact NDJSON path [default: {DEFAULT_OUTPUT_PATH}]\n    --output-path <PATH>      Label seed output path [default: {DEFAULT_LABEL_SEEDS_PATH}]\n\n\
EXAMPLES:\n    cargo run -p kleio --example wikidata_import -- import wikidata-label-seeds --input-path target/wikidata-closure.ndjson\n"
    );
}

fn print_wikidata_drafts_help() {
    println!(
        "wikidata_import import wikidata-drafts\n\n\
Build experimental Kleio-oriented person draft records from wikidata-truthy NDJSON facts.\n\n\
USAGE:\n    cargo run -p kleio --example wikidata_import -- import wikidata-drafts [OPTIONS]\n\n\
OPTIONS:\n    --input-path <PATH>       Input fact NDJSON path [default: {DEFAULT_OUTPUT_PATH}]\n    --output-path <PATH>      Draft NDJSON output path [default: {DEFAULT_DRAFT_OUTPUT_PATH}]\n    --label-cache <PATH>      Optional JSON object mapping QID to label\n\n\
EXAMPLES:\n    cargo run -p kleio --example wikidata_import -- import wikidata-drafts --input-path target/wikidata-sample.ndjson\n    cargo run -p kleio --example wikidata_import -- import wikidata-drafts --input-path target/wikidata-closure.ndjson --label-cache target/wikidata-labels.json\n"
    );
}

fn print_wikidata_kleio_help() {
    println!(
        "wikidata_import import wikidata-kleio\n\n\
Convert experimental Wikidata person draft NDJSON into a small Kleio GenealogyArchive `.rkyv` file. This is a prototype projection, not an authoritative import.\n\n\
USAGE:\n    cargo run -p kleio --example wikidata_import -- import wikidata-kleio [OPTIONS]\n\n\
OPTIONS:\n    --input-path <PATH>       Draft NDJSON input path [default: {DEFAULT_DRAFT_OUTPUT_PATH}]\n    --output-path <PATH>      Kleio rkyv output path [default: {DEFAULT_KLEIO_ARCHIVE_PATH}]\n    --include-non-humans      Include drafts without P31=Q5\n\n\
EXAMPLES:\n    cargo run -p kleio --example wikidata_import -- import wikidata-kleio --input-path target/wikidata-person-drafts.ndjson\n"
    );
}

fn print_wikidata_drafts_summary_help() {
    println!(
        "wikidata_import import wikidata-drafts-summary\n\n\
Summarize experimental Wikidata person draft NDJSON so you can quickly judge completeness/usefulness.\n\n\
USAGE:\n    cargo run -p kleio --example wikidata_import -- import wikidata-drafts-summary [OPTIONS]\n\n\
OPTIONS:\n    --input-path <PATH>       Draft NDJSON input path [default: {DEFAULT_DRAFT_OUTPUT_PATH}]\n    --limit <N>               Number of example drafts to print [default: 5]\n\n\
EXAMPLES:\n    cargo run -p kleio --example wikidata_import -- import wikidata-drafts-summary --input-path target/wikidata-person-drafts.ndjson\n"
    );
}

fn print_wikidata_kleio_inspect_help() {
    println!(
        "wikidata_import import wikidata-kleio-inspect\n\n\
Load and validate a generated Kleio `.rkyv` archive, then print core record counts.\n\n\
USAGE:\n    cargo run -p kleio --example wikidata_import -- import wikidata-kleio-inspect [OPTIONS]\n\n\
OPTIONS:\n    --path <PATH>             Kleio rkyv archive path [default: {DEFAULT_KLEIO_ARCHIVE_PATH}]\n\n\
EXAMPLES:\n    cargo run -p kleio --example wikidata_import -- import wikidata-kleio-inspect --path target/wikidata-kleio.rkyv\n"
    );
}

fn print_wikidata_truthy_help() {
    println!(
        "wikidata_import import wikidata-truthy\n\n\
Experimental bounded streaming import from a Wikidata truthy N-Triples bzip2 dump.\n\n\
USAGE:\n    cargo run -p kleio --example wikidata_import -- import wikidata-truthy [OPTIONS]\n\n\
OPTIONS:\n    --dump-path <PATH>        Dump path [default: {DEFAULT_DUMP_PATH}]\n    --output-path <PATH>      NDJSON output path [default: {DEFAULT_OUTPUT_PATH}]\n    --max-lines <N>           Stop after N decompressed lines [default: {DEFAULT_MAX_LINES}]\n    --max-facts <N>           Stop after N relevant facts [default: {DEFAULT_MAX_FACTS}]\n    --progress-every <N>      Print progress every N lines; 0 disables [default: {DEFAULT_PROGRESS_EVERY}]\n    --subject <QID>           Optional subject filter for testing one entity\n    --stop-after-subject      With --subject, stop after the first later relevant subject (assumes subject-grouped dump)\n\n\
EXAMPLES:\n    cargo run -p kleio --example wikidata_import -- import wikidata-truthy --dump-path vendor/latest-truthy.nt.bz2 --max-lines 1000000 --progress-every 100000\n    cargo run -p kleio --example wikidata_import -- import wikidata-truthy --max-facts 10000\n    cargo run -p kleio --example wikidata_import -- import wikidata-truthy --subject Q42 --stop-after-subject --max-lines 5000000\n"
    );
}
