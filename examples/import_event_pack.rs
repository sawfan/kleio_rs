use std::error::Error;
use std::fs;
use std::path::Path;

use kleio::{
    ImportBatch, ImportCandidateItem, ImportCandidateStatus, import_event_pack_json,
    import_event_pack_toml, mark_import_batch_validation,
};

fn main() -> Result<(), Box<dyn Error>> {
    let Some(path) = std::env::args_os().nth(1) else {
        eprintln!("Usage: cargo run -p kleio --example import_event_pack -- <pack.json|pack.toml>");
        std::process::exit(2);
    };
    let path = Path::new(&path);
    let text = fs::read_to_string(path)?;
    let source_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("event-pack");

    let mut batch = match path.extension().and_then(|extension| extension.to_str()) {
        Some("json") => import_event_pack_json(source_name, &text)?.batch,
        Some("toml") => import_event_pack_toml(source_name, &text)?.batch,
        _ => {
            eprintln!("unsupported pack extension; expected .json or .toml");
            std::process::exit(2);
        }
    };

    let validations = mark_import_batch_validation(&mut batch);
    print_batch_report(&batch, validations.len());

    Ok(())
}

fn print_batch_report(batch: &ImportBatch, validation_count: usize) {
    println!("import_batch_id={}", batch.id.as_str());
    println!("source_name={}", batch.source_name);
    println!("records={}", batch.records.len());
    println!("candidates={}", batch.candidates.len());
    println!("accepted={}", batch.accepted_count());
    println!("conflicts={}", batch.conflict_count());
    println!("candidates_with_validation_issues={validation_count}");

    let mut domain_profiles = 0;
    let mut entities = 0;
    let mut events = 0;
    let mut event_collections = 0;
    let mut event_relations = 0;
    let mut sources = 0;
    let mut tags = 0;

    for candidate in &batch.candidates {
        match &candidate.item {
            ImportCandidateItem::DomainProfile(_) => domain_profiles += 1,
            ImportCandidateItem::Entity(_) => entities += 1,
            ImportCandidateItem::Event(_) => events += 1,
            ImportCandidateItem::EventCollection(_) => event_collections += 1,
            ImportCandidateItem::EventRelation(_) => event_relations += 1,
            ImportCandidateItem::Source(_) => sources += 1,
            ImportCandidateItem::TagValue(_) => tags += 1,
        }
    }

    println!("domain_profiles={domain_profiles}");
    println!("entities={entities}");
    println!("events={events}");
    println!("event_collections={event_collections}");
    println!("event_relations={event_relations}");
    println!("sources={sources}");
    println!("tags={tags}");

    for candidate in &batch.candidates {
        if candidate.status == ImportCandidateStatus::Conflict || !candidate.messages.is_empty() {
            println!("candidate {} {:?}", candidate.id.as_str(), candidate.status);
            for message in &candidate.messages {
                println!("  - {message}");
            }
        }
    }
}
