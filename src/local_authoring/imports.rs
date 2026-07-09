use std::fs;
use std::path::{Path, PathBuf};

use super::{LocalAuthoringError, WorldPaths};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalImportKind {
    Gedcom,
    Wikidata,
    Csv,
}

impl LocalImportKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Gedcom => "gedcom",
            Self::Wikidata => "wikidata",
            Self::Csv => "csv",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalImportReportOptions {
    pub import_slug: String,
    pub kind: LocalImportKind,
    pub title: String,
    pub source_path: Option<String>,
    pub force: bool,
}

impl LocalImportReportOptions {
    pub fn id(&self) -> String {
        format!("import:{}:{}", self.kind.as_str(), self.import_slug)
    }
}

pub fn create_local_import_report(
    world_root: impl AsRef<Path>,
    options: &LocalImportReportOptions,
) -> Result<PathBuf, LocalAuthoringError> {
    validate_slug(&options.import_slug, "import slug")?;
    let world_root = world_root.as_ref();
    let paths = WorldPaths::new(world_root);
    let dir = match options.kind {
        LocalImportKind::Gedcom => paths.gedcom_imports_dir(),
        LocalImportKind::Wikidata => paths.wikidata_imports_dir(),
        LocalImportKind::Csv => paths.csv_imports_dir(),
    };
    create_dir(world_root, &dir)?;
    let path = dir.join(format!("{}-report.toml", options.import_slug));
    write_new_file(
        world_root,
        &path,
        &import_report_toml(options),
        options.force,
    )?;
    Ok(path)
}

fn import_report_toml(options: &LocalImportReportOptions) -> String {
    let source_path = options
        .source_path
        .as_deref()
        .map(|path| format!("source_path = \"{}\"\n", escape_toml_basic(path)))
        .unwrap_or_default();

    format!(
        r#"schema_version = 1
id = "{}"
kind = "{}-import-report"
title = "{}"
{}strategy = "link"
status = "planned"

[created]
entities = 0
events = 0
assertions = 0
sources = 0

[updated]
entities = 0
events = 0
assertions = 0
sources = 0
"#,
        escape_toml_basic(&options.id()),
        options.kind.as_str(),
        escape_toml_basic(&options.title),
        source_path
    )
}

fn create_dir(root: &Path, path: &Path) -> Result<(), LocalAuthoringError> {
    fs::create_dir_all(path).map_err(|source| LocalAuthoringError::Io {
        path: display_path(root, path),
        source,
    })
}

fn write_new_file(
    root: &Path,
    path: &Path,
    content: &str,
    force: bool,
) -> Result<(), LocalAuthoringError> {
    if path.exists() && !force {
        return Ok(());
    }

    fs::write(path, content).map_err(|source| LocalAuthoringError::Io {
        path: display_path(root, path),
        source,
    })
}

fn display_path(root: &Path, path: &Path) -> PathBuf {
    path.strip_prefix(root).unwrap_or(path).to_path_buf()
}

fn validate_slug(value: &str, label: &str) -> Result<(), LocalAuthoringError> {
    if value.trim().is_empty() {
        return Err(LocalAuthoringError::Validation {
            message: format!("{label} cannot be empty"),
        });
    }

    if value
        .chars()
        .any(|ch| ch.is_whitespace() || matches!(ch, '/' | '\\' | ':'))
    {
        return Err(LocalAuthoringError::Validation {
            message: format!("{label} `{value}` may not contain whitespace, slashes, or colons"),
        });
    }

    Ok(())
}

fn escape_toml_basic(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::local_authoring::{LocalSkeletonOptions, create_workspace_skeleton};

    #[test]
    fn creates_import_report() {
        let temp_dir = std::env::temp_dir().join(format!(
            "kleio-import-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time")
                .as_nanos()
        ));
        create_workspace_skeleton(&temp_dir, &LocalSkeletonOptions::default()).expect("skeleton");
        let world_root = temp_dir.join("worlds/default");

        let path = create_local_import_report(
            &world_root,
            &LocalImportReportOptions {
                import_slug: "example".to_string(),
                kind: LocalImportKind::Gedcom,
                title: "Example Import".to_string(),
                source_path: Some("imports/gedcom/example.ged".to_string()),
                force: false,
            },
        )
        .expect("import report");

        assert_eq!(
            path.strip_prefix(&world_root).unwrap(),
            Path::new("imports/gedcom/example-report.toml")
        );

        fs::remove_dir_all(temp_dir).expect("remove temp dir");
    }
}
