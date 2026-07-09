use std::fs;
use std::path::{Path, PathBuf};

use super::LocalAuthoringError;
use super::paths::{DEFAULT_WORLD_SLUG, WorkspacePaths, WorldPaths};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct WorkspaceConfig {
    pub schema_version: u32,
    pub workspace: WorkspaceInfo,
    pub worlds: Vec<WorkspaceWorldEntry>,
}

pub fn read_world_config(world_root: impl AsRef<Path>) -> Result<WorldConfig, LocalAuthoringError> {
    let path = world_root.as_ref().join("world.toml");
    let text = fs::read_to_string(&path).map_err(|source| LocalAuthoringError::Io {
        path: path.clone(),
        source,
    })?;
    toml::from_str(&text).map_err(|source| LocalAuthoringError::Toml { path, source })
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct WorkspaceInfo {
    pub id: String,
    pub title: String,
    pub default_world: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct WorkspaceWorldEntry {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub path: String,
}

impl WorkspaceConfig {
    pub const SCHEMA_VERSION: u32 = 1;

    pub fn with_default_world(world_slug: &str, world_title: &str) -> Self {
        let mut config = Self {
            schema_version: Self::SCHEMA_VERSION,
            workspace: WorkspaceInfo {
                id: "workspace:default".to_string(),
                title: "Kleio workspace".to_string(),
                default_world: world_slug.to_string(),
            },
            worlds: Vec::new(),
        };
        config.upsert_world(world_slug, world_title);
        config
    }

    pub fn upsert_world(&mut self, world_slug: &str, world_title: &str) {
        let entry = WorkspaceWorldEntry {
            id: format!("world:{world_slug}"),
            slug: world_slug.to_string(),
            title: world_title.to_string(),
            path: format!("worlds/{world_slug}"),
        };

        if let Some(existing) = self
            .worlds
            .iter_mut()
            .find(|existing| existing.slug == world_slug)
        {
            *existing = entry;
        } else {
            self.worlds.push(entry);
        }
    }

    pub fn world_entry(&self, world_slug: &str) -> Option<&WorkspaceWorldEntry> {
        self.worlds.iter().find(|world| world.slug == world_slug)
    }

    pub fn default_world_entry(&self) -> Option<&WorkspaceWorldEntry> {
        self.world_entry(&self.workspace.default_world)
    }

    pub fn world_path(&self, world_slug: &str) -> Option<&str> {
        self.world_entry(world_slug)
            .map(|world| world.path.as_str())
    }

    pub fn default_world_path(&self) -> Option<&str> {
        self.default_world_entry().map(|world| world.path.as_str())
    }
}

pub fn read_workspace_config(
    workspace_root: impl AsRef<Path>,
) -> Result<WorkspaceConfig, LocalAuthoringError> {
    let path = workspace_root.as_ref().join("kleio.toml");
    let text = fs::read_to_string(&path).map_err(|source| LocalAuthoringError::Io {
        path: path.clone(),
        source,
    })?;
    toml::from_str(&text).map_err(|source| LocalAuthoringError::Toml { path, source })
}

pub fn write_workspace_config(
    workspace_root: impl AsRef<Path>,
    config: &WorkspaceConfig,
) -> Result<(), LocalAuthoringError> {
    let path = workspace_root.as_ref().join("kleio.toml");
    let toml =
        toml::to_string_pretty(config).map_err(|source| LocalAuthoringError::TomlSerialize {
            path: path.clone(),
            source,
        })?;
    fs::write(&path, format!("{toml}\n")).map_err(|source| LocalAuthoringError::Io { path, source })
}

pub fn resolve_workspace_world_root(
    workspace_root: impl AsRef<Path>,
    world_slug: Option<&str>,
) -> Result<PathBuf, LocalAuthoringError> {
    let workspace_root = workspace_root.as_ref();
    let config_path = workspace_root.join("kleio.toml");
    let config = if config_path.exists() {
        Some(read_workspace_config(workspace_root)?)
    } else {
        None
    };
    let world_slug = world_slug
        .map(ToOwned::to_owned)
        .or_else(|| {
            config
                .as_ref()
                .map(|config| config.workspace.default_world.clone())
        })
        .filter(|slug| !slug.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_WORLD_SLUG.to_string());

    if let Some(config) = &config {
        if let Some(path) = config.world_path(&world_slug) {
            return normalize_workspace_relative_path(workspace_root, path);
        }

        return Err(LocalAuthoringError::Validation {
            message: format!(
                "world `{world_slug}` is not registered in {}",
                config_path.display()
            ),
        });
    }

    Ok(WorkspacePaths::new(workspace_root)
        .world(&world_slug)
        .root()
        .to_path_buf())
}

fn normalize_workspace_relative_path(
    workspace_root: &Path,
    path: &str,
) -> Result<PathBuf, LocalAuthoringError> {
    let path = Path::new(path.trim());
    if path.as_os_str().is_empty() || path.is_absolute() {
        return Err(LocalAuthoringError::Validation {
            message: format!("world path `{}` must be relative", path.display()),
        });
    }

    if path.components().any(|component| {
        matches!(
            component,
            std::path::Component::ParentDir | std::path::Component::RootDir
        )
    }) {
        return Err(LocalAuthoringError::Validation {
            message: format!(
                "world path `{}` must stay inside the workspace root",
                path.display()
            ),
        });
    }

    Ok(workspace_root.join(path))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldBuildPaths {
    pub compiled_json: PathBuf,
    pub ecs_json: PathBuf,
    pub sqlite: PathBuf,
}

pub fn resolve_world_build_paths(
    world_root: impl AsRef<Path>,
) -> Result<WorldBuildPaths, LocalAuthoringError> {
    let world_root = world_root.as_ref();
    let world_config_path = world_root.join("world.toml");
    if !world_config_path.exists() {
        let paths = WorldPaths::new(world_root);
        return Ok(WorldBuildPaths {
            compiled_json: paths.compiled_json(),
            ecs_json: paths.ecs_json(),
            sqlite: paths.sqlite(),
        });
    }

    let config = read_world_config(world_root)?;
    Ok(WorldBuildPaths {
        compiled_json: normalize_world_relative_path(world_root, &config.build.compiled_json)?,
        ecs_json: normalize_world_relative_path(world_root, &config.build.ecs_json)?,
        sqlite: normalize_world_relative_path(world_root, &config.build.sqlite)?,
    })
}

fn normalize_world_relative_path(
    world_root: &Path,
    path: &str,
) -> Result<PathBuf, LocalAuthoringError> {
    let path = Path::new(path.trim());
    if path.as_os_str().is_empty() || path.is_absolute() {
        return Err(LocalAuthoringError::Validation {
            message: format!("world build path `{}` must be relative", path.display()),
        });
    }

    if path.components().any(|component| {
        matches!(
            component,
            std::path::Component::ParentDir | std::path::Component::RootDir
        )
    }) {
        return Err(LocalAuthoringError::Validation {
            message: format!(
                "world build path `{}` must stay inside the world root",
                path.display()
            ),
        });
    }

    Ok(world_root.join(path))
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct WorldConfig {
    pub schema_version: u32,
    pub id: String,
    pub slug: String,
    pub title: String,
    pub kind: String,
    pub build: WorldBuildConfig,
    #[serde(default, skip_serializing_if = "WorldImportsConfig::is_empty")]
    pub imports: WorldImportsConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct WorldBuildConfig {
    pub compiled_json: String,
    pub ecs_json: String,
    pub sqlite: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct WorldImportsConfig {
    #[serde(default, skip_serializing_if = "GedcomImportsConfig::is_empty")]
    pub gedcom: GedcomImportsConfig,
}

impl WorldImportsConfig {
    pub fn is_empty(&self) -> bool {
        self.gedcom.is_empty()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GedcomImportsConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primary: Option<GedcomImportConfig>,
}

impl GedcomImportsConfig {
    pub fn is_empty(&self) -> bool {
        self.primary.is_none()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GedcomImportConfig {
    pub path: String,
    pub strategy: String,
}

impl WorldConfig {
    pub const SCHEMA_VERSION: u32 = 1;

    pub fn new(slug: &str, title: &str) -> Self {
        Self {
            schema_version: Self::SCHEMA_VERSION,
            id: format!("world:{slug}"),
            slug: slug.to_string(),
            title: title.to_string(),
            kind: "family-history".to_string(),
            build: WorldBuildConfig {
                compiled_json: "build/kleio.compiled.json".to_string(),
                ecs_json: "build/kleio.ecs.json".to_string(),
                sqlite: "build/kleio.sqlite".to_string(),
            },
            imports: WorldImportsConfig::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn resolves_world_path_from_workspace_config() {
        let temp_dir = test_temp_dir("resolve-world-path");
        fs::create_dir_all(temp_dir.join("custom/world-root")).expect("world dir");
        let config = WorkspaceConfig {
            schema_version: WorkspaceConfig::SCHEMA_VERSION,
            workspace: WorkspaceInfo {
                id: "workspace:default".to_string(),
                title: "Kleio workspace".to_string(),
                default_world: "custom".to_string(),
            },
            worlds: vec![WorkspaceWorldEntry {
                id: "world:custom".to_string(),
                slug: "custom".to_string(),
                title: "Custom World".to_string(),
                path: "custom/world-root".to_string(),
            }],
        };
        write_workspace_config(&temp_dir, &config).expect("write config");

        let resolved =
            resolve_workspace_world_root(&temp_dir, None).expect("resolve default world");

        assert_eq!(resolved, temp_dir.join("custom/world-root"));
        fs::remove_dir_all(temp_dir).expect("remove temp dir");
    }

    #[test]
    fn rejects_world_path_that_escapes_workspace() {
        let temp_dir = test_temp_dir("reject-world-path");
        fs::create_dir_all(&temp_dir).expect("workspace dir");
        fs::write(
            temp_dir.join("kleio.toml"),
            r#"schema_version = 1

[workspace]
id = "workspace:default"
title = "Kleio workspace"
default_world = "bad"

[[worlds]]
id = "world:bad"
slug = "bad"
title = "Bad World"
path = "../bad"
"#,
        )
        .expect("config");

        let err = resolve_workspace_world_root(&temp_dir, None).expect_err("bad path rejected");

        assert!(
            err.to_string()
                .contains("must stay inside the workspace root"),
            "unexpected error: {err}"
        );
        fs::remove_dir_all(temp_dir).expect("remove temp dir");
    }

    #[test]
    fn rejects_unregistered_world_when_workspace_config_exists() {
        let temp_dir = test_temp_dir("reject-unregistered-world");
        fs::create_dir_all(&temp_dir).expect("workspace dir");
        write_workspace_config(
            &temp_dir,
            &WorkspaceConfig::with_default_world("default", "Default world"),
        )
        .expect("config");

        let err = resolve_workspace_world_root(&temp_dir, Some("missing"))
            .expect_err("missing world rejected");

        assert!(
            err.to_string()
                .contains("world `missing` is not registered"),
            "unexpected error: {err}"
        );
        fs::remove_dir_all(temp_dir).expect("remove temp dir");
    }

    #[test]
    fn resolves_build_paths_from_world_config() {
        let temp_dir = test_temp_dir("resolve-build-paths");
        fs::create_dir_all(&temp_dir).expect("world dir");
        fs::write(
            temp_dir.join("world.toml"),
            r#"schema_version = 1
id = "world:default"
slug = "default"
title = "Default world"
kind = "family-history"

[build]
compiled_json = "out/semantic.json"
ecs_json = "out/ecs.json"
sqlite = "out/kleio.sqlite"
"#,
        )
        .expect("world config");

        let paths = resolve_world_build_paths(&temp_dir).expect("resolve build paths");

        assert_eq!(paths.compiled_json, temp_dir.join("out/semantic.json"));
        assert_eq!(paths.ecs_json, temp_dir.join("out/ecs.json"));
        assert_eq!(paths.sqlite, temp_dir.join("out/kleio.sqlite"));
        fs::remove_dir_all(temp_dir).expect("remove temp dir");
    }

    #[test]
    fn rejects_build_path_that_escapes_world() {
        let temp_dir = test_temp_dir("reject-build-path");
        fs::create_dir_all(&temp_dir).expect("world dir");
        fs::write(
            temp_dir.join("world.toml"),
            r#"schema_version = 1
id = "world:default"
slug = "default"
title = "Default world"
kind = "family-history"

[build]
compiled_json = "../semantic.json"
ecs_json = "build/ecs.json"
sqlite = "build/kleio.sqlite"
"#,
        )
        .expect("world config");

        let err = resolve_world_build_paths(&temp_dir).expect_err("bad build path rejected");

        assert!(
            err.to_string().contains("must stay inside the world root"),
            "unexpected error: {err}"
        );
        fs::remove_dir_all(temp_dir).expect("remove temp dir");
    }

    fn test_temp_dir(label: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "kleio-config-{label}-{}-{unique}",
            std::process::id()
        ))
    }
}
