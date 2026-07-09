use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputKind {
    Bundle,
    Tree,
    TreesDocument,
}

fn main() -> ExitCode {
    let mut check = false;
    let mut init_example = false;
    let mut output_kind = OutputKind::Tree;
    let mut positional = Vec::new();

    for arg in env::args().skip(1) {
        match arg.as_str() {
            "--check" => check = true,
            "--init-example" => init_example = true,
            "--bundle" => output_kind = OutputKind::Bundle,
            "--tree" => output_kind = OutputKind::Tree,
            "--trees-document" => output_kind = OutputKind::TreesDocument,
            "--help" | "-h" => {
                print_usage();
                return ExitCode::SUCCESS;
            }
            _ => positional.push(arg),
        }
    }

    if positional.len() > 2 {
        print_usage();
        return ExitCode::FAILURE;
    }

    let source_root = positional
        .first()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("local-data"));

    if init_example {
        return init_example_local_data(source_root);
    }

    let output_path = positional
        .get(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| default_output_path(&source_root, output_kind));

    match (check, output_kind) {
        (true, OutputKind::Bundle) => check_local_data_bundle_json(source_root, output_path),
        (true, OutputKind::Tree) => check_local_tree_json(source_root, output_path),
        (true, OutputKind::TreesDocument) => {
            check_local_trees_document_json(source_root, output_path)
        }
        (false, OutputKind::Bundle) => write_local_data_bundle_json(source_root, output_path),
        (false, OutputKind::Tree) => write_local_tree_json(source_root, output_path),
        (false, OutputKind::TreesDocument) => {
            write_local_trees_document_json(source_root, output_path)
        }
    }
}

fn default_output_path(source_root: &std::path::Path, output_kind: OutputKind) -> PathBuf {
    match output_kind {
        OutputKind::Bundle => source_root.join("compiled/kleio-local-data.json"),
        OutputKind::Tree => source_root.join("compiled/kleio-tree.json"),
        OutputKind::TreesDocument => source_root.join("compiled/ourania-trees-document.json"),
    }
}

fn write_local_data_bundle_json(source_root: PathBuf, output_path: PathBuf) -> ExitCode {
    match kleio::write_local_data_json(&source_root, &output_path) {
        Ok(bundle) => {
            println!(
                "compiled {} Markdown records and {} TOML documents into {}",
                bundle.markdown_records.len(),
                bundle.toml_documents.len(),
                output_path.display()
            );
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("failed to compile local Kleio data: {err}");
            ExitCode::FAILURE
        }
    }
}

fn init_example_local_data(source_root: PathBuf) -> ExitCode {
    let files = [
        (
            "registry.toml",
            "id = \"registry_private_tree_example\"\nkind = \"registry\"\ntitle = \"Private example tree registry\"\n\n[tree]\nid = \"private-example-tree\"\ntitle = \"Private Example Tree\"\ndescription = \"Private ignored example tree compiled from local-data files.\"\nmain_person = \"person_alex_example\"\n",
        ),
        (
            "records/person_alex_example.md",
            "+++\nid = \"person_alex_example\"\nkind = \"person\"\ntitle = \"Alex Example\"\ndate = 1900-01-01\nsummary = \"Fictional placeholder person for private local-data authoring.\"\ntags = [\"example\", \"fictional\"]\nrelated = [\"person_morgan_example\"]\ngiven = \"Alex\"\nsurname = \"Example\"\nsex = \"unknown\"\nx = 0\ny = 0\n+++\n\n# Alex Example\n\nThis ignored private example record is fictional placeholder data only.\n",
        ),
        (
            "records/person_morgan_example.md",
            "+++\nid = \"person_morgan_example\"\nkind = \"person\"\ntitle = \"Morgan Example\"\ndate = 1900-01-01\nsummary = \"Second fictional placeholder person for private relationship tests.\"\ntags = [\"example\", \"fictional\"]\nrelated = [\"person_alex_example\"]\ngiven = \"Morgan\"\nsurname = \"Example\"\nsex = \"unknown\"\nx = 180\ny = 0\n+++\n\n# Morgan Example\n\nThis ignored private example record is fictional placeholder data only.\n",
        ),
        (
            "relationships/alex_morgan_example.toml",
            "id = \"relationship_alex_morgan_example\"\nkind = \"relationship\"\ntitle = \"Example association\"\nrelationship = \"associate\"\nsource = \"person_alex_example\"\ntarget = \"person_morgan_example\"\n",
        ),
    ];

    for (relative_path, contents) in files {
        let path = source_root.join(relative_path);
        if path.exists() {
            eprintln!("left existing {} unchanged", path.display());
            continue;
        }

        if let Some(parent) = path.parent() {
            if let Err(err) = fs::create_dir_all(parent) {
                eprintln!("failed to create {}: {err}", parent.display());
                return ExitCode::FAILURE;
            }
        }

        if let Err(err) = fs::write(&path, contents) {
            eprintln!("failed to write {}: {err}", path.display());
            return ExitCode::FAILURE;
        }

        println!("wrote {}", path.display());
    }

    println!(
        "initialized ignored private local-data example under {}",
        source_root.display()
    );
    ExitCode::SUCCESS
}

fn write_local_tree_json(source_root: PathBuf, output_path: PathBuf) -> ExitCode {
    match kleio::write_local_tree_json(&source_root, &output_path) {
        Ok(tree) => {
            println!(
                "compiled tree {} with {} people and {} relationships into {}",
                tree.metadata.id.0,
                tree.people.len(),
                tree.relationships.len(),
                output_path.display()
            );
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("failed to compile local Kleio tree: {err}");
            ExitCode::FAILURE
        }
    }
}

fn write_local_trees_document_json(source_root: PathBuf, output_path: PathBuf) -> ExitCode {
    match kleio::write_local_trees_document_json(&source_root, &output_path) {
        Ok(document) => {
            let people = document
                .trees
                .iter()
                .map(|tree| tree.people.len())
                .sum::<usize>();
            let relationships = document
                .trees
                .iter()
                .map(|tree| tree.relationships.len())
                .sum::<usize>();
            println!(
                "compiled trees document with {} tree(s), {people} people, and {relationships} relationships into {}",
                document.trees.len(),
                output_path.display()
            );
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("failed to compile local Ourania trees document: {err}");
            ExitCode::FAILURE
        }
    }
}

fn check_local_data_bundle_json(source_root: PathBuf, output_path: PathBuf) -> ExitCode {
    let bundle = match kleio::compile_local_data(&source_root) {
        Ok(bundle) => bundle,
        Err(err) => {
            eprintln!("failed to compile local Kleio data: {err}");
            return ExitCode::FAILURE;
        }
    };

    let expected = match serde_json::to_string_pretty(&bundle) {
        Ok(json) => format!("{json}\n"),
        Err(err) => {
            eprintln!("failed to serialize local Kleio data: {err}");
            return ExitCode::FAILURE;
        }
    };

    check_expected_json(&output_path, &source_root, expected, "--bundle")
}

fn check_local_tree_json(source_root: PathBuf, output_path: PathBuf) -> ExitCode {
    let tree = match kleio::compile_local_tree(&source_root) {
        Ok(tree) => tree,
        Err(err) => {
            eprintln!("failed to compile local Kleio tree: {err}");
            return ExitCode::FAILURE;
        }
    };

    let expected = match serde_json::to_string_pretty(&tree) {
        Ok(json) => format!("{json}\n"),
        Err(err) => {
            eprintln!("failed to serialize local Kleio tree: {err}");
            return ExitCode::FAILURE;
        }
    };

    check_expected_json(&output_path, &source_root, expected, "--tree")
}

fn check_local_trees_document_json(source_root: PathBuf, output_path: PathBuf) -> ExitCode {
    let document = match kleio::compile_local_trees_document(&source_root) {
        Ok(document) => document,
        Err(err) => {
            eprintln!("failed to compile local Ourania trees document: {err}");
            return ExitCode::FAILURE;
        }
    };

    let expected = match serde_json::to_string_pretty(&document) {
        Ok(json) => format!("{json}\n"),
        Err(err) => {
            eprintln!("failed to serialize local Ourania trees document: {err}");
            return ExitCode::FAILURE;
        }
    };

    check_expected_json(&output_path, &source_root, expected, "--trees-document")
}

fn check_expected_json(
    output_path: &std::path::Path,
    source_root: &std::path::Path,
    expected: String,
    flag: &str,
) -> ExitCode {
    let actual = match fs::read_to_string(output_path) {
        Ok(actual) => actual,
        Err(err) => {
            eprintln!(
                "{} is missing or unreadable: {err}\nrun without --check to regenerate it",
                output_path.display()
            );
            return ExitCode::FAILURE;
        }
    };

    if actual == expected {
        println!("{} is up to date", output_path.display());
        ExitCode::SUCCESS
    } else {
        eprintln!(
            "{} is stale\nrun: cargo run -p kleio --example compile_local_data -- {flag} {} {}",
            output_path.display(),
            source_root.display(),
            output_path.display()
        );
        ExitCode::FAILURE
    }
}

fn print_usage() {
    eprintln!(
        "Usage: cargo run -p kleio --example compile_local_data -- [--tree|--trees-document|--bundle] [--check] [--init-example] [source-root] [output-json]\n\n\
         Defaults:\n  mode: --tree\n  source-root: local-data\n  tree output: local-data/compiled/kleio-tree.json\n  trees document output: local-data/compiled/ourania-trees-document.json\n  bundle output: local-data/compiled/kleio-local-data.json"
    );
}
