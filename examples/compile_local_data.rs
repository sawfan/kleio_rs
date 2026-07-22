use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use kleio::{DEFAULT_WORLD_SLUG, LocalSkeletonOptions, WorkspacePaths, create_workspace_skeleton};

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

    let workspace_root = positional
        .first()
        .map(PathBuf::from)
        .unwrap_or_else(default_data_root);
    let source_root = WorkspacePaths::new(&workspace_root)
        .world(DEFAULT_WORLD_SLUG)
        .root()
        .to_path_buf();

    if init_example {
        return init_example_local_data(workspace_root);
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
        OutputKind::Bundle => source_root.join("build/kleio.compiled.json"),
        OutputKind::Tree => source_root.join("build/kleio-tree.json"),
        OutputKind::TreesDocument => source_root.join("build/kleio-trees-document.json"),
    }
}

fn default_data_root() -> PathBuf {
    if let Some(path) = std::env::var_os("KLEIO_DATA_DIR").filter(|value| !value.is_empty()) {
        return PathBuf::from(path);
    }

    if let Some(path) = std::env::var_os("XDG_DATA_HOME").filter(|value| !value.is_empty()) {
        return Path::new(&path).join("kleio");
    }

    if let Some(home) = std::env::var_os("HOME").filter(|value| !value.is_empty()) {
        return Path::new(&home).join(".local/share/kleio");
    }

    PathBuf::from(".kleio-data")
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

fn init_example_local_data(workspace_root: PathBuf) -> ExitCode {
    let options = LocalSkeletonOptions {
        birth_date: Some("1900-01-01".to_string()),
        force: false,
        ..LocalSkeletonOptions::default()
    };

    match create_workspace_skeleton(&workspace_root, &options) {
        Ok(()) => {
            println!(
                "initialized ignored private Kleio workspace example under {}",
                workspace_root.display()
            );
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("failed to initialize Kleio workspace example: {err}");
            ExitCode::FAILURE
        }
    }
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
            eprintln!("failed to compile local Kleio trees document: {err}");
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
            eprintln!("failed to compile local Kleio trees document: {err}");
            return ExitCode::FAILURE;
        }
    };

    let expected = match serde_json::to_string_pretty(&document) {
        Ok(json) => format!("{json}\n"),
        Err(err) => {
            eprintln!("failed to serialize local Kleio trees document: {err}");
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
        "Usage: cargo run -p kleio --example compile_local_data -- [--tree|--trees-document|--bundle] [--check] [--init-example] [workspace-root] [output-json]\n\n\
  workspace-root: $KLEIO_DATA_DIR, $XDG_DATA_HOME/kleio, or ~/.local/share/kleio\n  compiles the default world at <workspace-root>/worlds/default\n  tree output: <world-root>/build/kleio-tree.json\n  trees document output: <world-root>/build/kleio-trees-document.json\n  bundle output: <world-root>/build/kleio.compiled.json"
    );
}
