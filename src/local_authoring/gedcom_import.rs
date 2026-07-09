use std::fs;
use std::path::{Path, PathBuf};

use super::LocalAuthoringError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrimaryGedcomImportOptions {
    pub path: String,
    pub strategy: String,
    pub allow_missing: bool,
}

impl Default for PrimaryGedcomImportOptions {
    fn default() -> Self {
        Self {
            path: "imports/gedcom/family.ged".to_string(),
            strategy: "link".to_string(),
            allow_missing: false,
        }
    }
}

pub fn set_primary_gedcom_import(
    root: impl AsRef<Path>,
    options: &PrimaryGedcomImportOptions,
) -> Result<(), LocalAuthoringError> {
    let root = root.as_ref();
    let config_path = root.join("kleio.toml");
    let normalized_path = normalize_import_path(root, &options.path)?;

    if options.strategy.trim().is_empty() {
        return Err(LocalAuthoringError::Validation {
            message: "GEDCOM import strategy cannot be empty".to_string(),
        });
    }

    if !options.allow_missing && !root.join(&normalized_path).exists() {
        return Err(LocalAuthoringError::Validation {
            message: format!(
                "GEDCOM import `{}` does not exist; pass --allow-missing to link it anyway",
                normalized_path.display()
            ),
        });
    }

    let text = fs::read_to_string(&config_path).map_err(|source| LocalAuthoringError::Io {
        path: config_path.clone(),
        source,
    })?;
    let mut table = text
        .parse::<toml::Table>()
        .map_err(|source| LocalAuthoringError::Toml {
            path: config_path.clone(),
            source,
        })?;

    set_nested_string(
        &mut table,
        &["imports", "gedcom", "primary", "path"],
        &relative_path_to_string(&normalized_path),
    );
    set_nested_string(
        &mut table,
        &["imports", "gedcom", "primary", "strategy"],
        options.strategy.trim(),
    );

    let toml =
        toml::to_string_pretty(&table).map_err(|source| LocalAuthoringError::TomlSerialize {
            path: config_path.clone(),
            source,
        })?;
    fs::write(&config_path, format!("{toml}\n")).map_err(|source| LocalAuthoringError::Io {
        path: config_path,
        source,
    })?;

    Ok(())
}

fn normalize_import_path(root: &Path, path: &str) -> Result<PathBuf, LocalAuthoringError> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err(LocalAuthoringError::Validation {
            message: "GEDCOM import path cannot be empty".to_string(),
        });
    }

    let path = Path::new(trimmed);
    let relative = if path.is_absolute() {
        path.strip_prefix(root)
            .map_err(|_| LocalAuthoringError::Validation {
                message: format!(
                    "absolute GEDCOM path `{}` is not under local data root `{}`",
                    path.display(),
                    root.display()
                ),
            })?
    } else {
        path
    };

    if relative.components().any(|component| {
        matches!(
            component,
            std::path::Component::ParentDir | std::path::Component::RootDir
        )
    }) {
        return Err(LocalAuthoringError::Validation {
            message: format!(
                "GEDCOM import path `{}` must stay inside the local data root",
                relative.display()
            ),
        });
    }

    Ok(relative.to_path_buf())
}

fn set_nested_string(table: &mut toml::Table, path: &[&str], value: &str) {
    let Some((last, parents)) = path.split_last() else {
        return;
    };

    let mut current = table;
    for key in parents {
        if !current.contains_key(*key) {
            current.insert((*key).to_string(), toml::Value::Table(toml::Table::new()));
        }
        if !current.get(*key).is_some_and(toml::Value::is_table) {
            current.insert((*key).to_string(), toml::Value::Table(toml::Table::new()));
        }
        current = current
            .get_mut(*key)
            .and_then(toml::Value::as_table_mut)
            .expect("table inserted above");
    }

    current.insert((*last).to_string(), toml::Value::String(value.to_string()));
}

fn relative_path_to_string(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::local_authoring::{LocalSkeletonOptions, create_local_skeleton};

    #[test]
    fn links_primary_gedcom_in_project_config() {
        let temp_dir = std::env::temp_dir().join(format!(
            "kleio-gedcom-link-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time")
                .as_nanos()
        ));
        create_local_skeleton(&temp_dir, &LocalSkeletonOptions::default()).expect("skeleton");
        fs::write(
            temp_dir.join("imports/gedcom/family.ged"),
            "0 HEAD\n0 TRLR\n",
        )
        .expect("gedcom");

        set_primary_gedcom_import(
            &temp_dir,
            &PrimaryGedcomImportOptions {
                path: "imports/gedcom/family.ged".to_string(),
                strategy: "link".to_string(),
                allow_missing: false,
            },
        )
        .expect("set primary gedcom");

        let updated = fs::read_to_string(temp_dir.join("kleio.toml")).expect("config");
        assert!(updated.contains("path = \"imports/gedcom/family.ged\""));
        assert!(updated.contains("strategy = \"link\""));

        fs::remove_dir_all(temp_dir).expect("remove temp dir");
    }
}
