use std::path::{Path, PathBuf};

use super::{
    LocalAuthoringError, resolve_world_build_paths, write_local_data_json, write_local_ecs_json,
    write_local_timeline_json, write_local_tree_json_with_view,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalWorldBuildOutput {
    pub semantic_json_path: PathBuf,
    pub ecs_json_path: PathBuf,
    pub timeline_json_path: Option<PathBuf>,
    pub tree_json_path: Option<PathBuf>,
    pub markdown_records: usize,
    pub toml_documents: usize,
    pub ecs_entities: usize,
    pub timeline_events: Option<usize>,
    pub tree_people: Option<usize>,
    pub tree_events: Option<usize>,
    pub tree_relationships: Option<usize>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LocalWorldBuildOptions<'a> {
    pub timeline_view: Option<&'a str>,
    pub tree_view: Option<&'a str>,
}

pub fn build_local_world(
    world_root: impl AsRef<Path>,
) -> Result<LocalWorldBuildOutput, LocalAuthoringError> {
    build_local_world_with_options(world_root, &LocalWorldBuildOptions::default())
}

pub fn build_local_world_with_options(
    world_root: impl AsRef<Path>,
    options: &LocalWorldBuildOptions<'_>,
) -> Result<LocalWorldBuildOutput, LocalAuthoringError> {
    let world_root = world_root.as_ref();
    let build_paths = resolve_world_build_paths(world_root)?;
    let semantic_json_path = build_paths.compiled_json;
    let ecs_json_path = build_paths.ecs_json;
    let semantic = write_local_data_json(world_root, &semantic_json_path)?;
    let ecs = write_local_ecs_json(world_root, &ecs_json_path)?;

    let build_dir = semantic_json_path.parent().unwrap_or(world_root);
    let (timeline_json_path, timeline_events) = if let Some(view) = options.timeline_view {
        let path = build_dir.join(format!("{view}.timeline.json"));
        let timeline = write_local_timeline_json(world_root, Some(view), &path)?;
        (Some(path), Some(timeline.events.len()))
    } else {
        (None, None)
    };

    let (tree_json_path, tree_people, tree_events, tree_relationships) =
        if let Some(view) = options.tree_view {
            let path = build_dir.join(format!("{view}.tree.json"));
            let tree = write_local_tree_json_with_view(world_root, Some(view), &path)?;
            (
                Some(path),
                Some(tree.people.len()),
                Some(tree.events.len()),
                Some(tree.relationships.len()),
            )
        } else {
            (None, None, None, None)
        };

    Ok(LocalWorldBuildOutput {
        semantic_json_path,
        ecs_json_path,
        timeline_json_path,
        tree_json_path,
        markdown_records: semantic.markdown_records.len(),
        toml_documents: semantic.toml_documents.len(),
        ecs_entities: ecs.entities.len(),
        timeline_events,
        tree_people,
        tree_events,
        tree_relationships,
    })
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::local_authoring::{LocalSkeletonOptions, create_workspace_skeleton};

    #[test]
    fn builds_standard_world_outputs() {
        let temp_dir = test_temp_dir("standard");
        create_workspace_skeleton(&temp_dir, &LocalSkeletonOptions::default()).expect("skeleton");
        let world_root = temp_dir.join("worlds/default");

        let output = build_local_world(&world_root).expect("build world");

        assert_eq!(
            output.semantic_json_path,
            world_root.join("build/kleio.compiled.json")
        );
        assert_eq!(
            output.ecs_json_path,
            world_root.join("build/kleio.ecs.json")
        );
        assert!(output.markdown_records > 0);
        assert!(output.ecs_entities > 0);
        assert!(output.timeline_json_path.is_none());
        assert!(output.tree_json_path.is_none());
        assert!(output.semantic_json_path.exists());
        assert!(output.ecs_json_path.exists());

        fs::remove_dir_all(temp_dir).expect("remove temp dir");
    }

    #[test]
    fn builds_selected_view_outputs() {
        let temp_dir = test_temp_dir("views");
        create_workspace_skeleton(
            &temp_dir,
            &LocalSkeletonOptions {
                birth_date: Some("1900-01-01".to_string()),
                ..LocalSkeletonOptions::default()
            },
        )
        .expect("skeleton");
        let world_root = temp_dir.join("worlds/default");

        let output = build_local_world_with_options(
            &world_root,
            &LocalWorldBuildOptions {
                timeline_view: Some("example-life"),
                tree_view: Some("main-family-tree"),
            },
        )
        .expect("build world views");

        assert_eq!(
            output.timeline_json_path,
            Some(world_root.join("build/example-life.timeline.json"))
        );
        assert_eq!(
            output.tree_json_path,
            Some(world_root.join("build/main-family-tree.tree.json"))
        );
        assert_eq!(output.timeline_events, Some(1));
        assert_eq!(output.tree_people, Some(1));
        assert_eq!(output.tree_events, Some(1));
        assert_eq!(output.tree_relationships, Some(0));
        assert!(
            output
                .timeline_json_path
                .as_ref()
                .is_some_and(|path| path.exists())
        );
        assert!(
            output
                .tree_json_path
                .as_ref()
                .is_some_and(|path| path.exists())
        );

        fs::remove_dir_all(temp_dir).expect("remove temp dir");
    }

    #[test]
    fn builds_selected_view_outputs_in_configured_build_dir() {
        let temp_dir = test_temp_dir("configured-views");
        create_workspace_skeleton(
            &temp_dir,
            &LocalSkeletonOptions {
                birth_date: Some("1900-01-01".to_string()),
                ..LocalSkeletonOptions::default()
            },
        )
        .expect("skeleton");
        let world_root = temp_dir.join("worlds/default");
        fs::write(
            world_root.join("world.toml"),
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

        let output = build_local_world_with_options(
            &world_root,
            &LocalWorldBuildOptions {
                timeline_view: Some("example-life"),
                tree_view: Some("main-family-tree"),
            },
        )
        .expect("build world views");

        assert_eq!(
            output.semantic_json_path,
            world_root.join("out/semantic.json")
        );
        assert_eq!(output.ecs_json_path, world_root.join("out/ecs.json"));
        assert_eq!(
            output.timeline_json_path,
            Some(world_root.join("out/example-life.timeline.json"))
        );
        assert_eq!(
            output.tree_json_path,
            Some(world_root.join("out/main-family-tree.tree.json"))
        );

        fs::remove_dir_all(temp_dir).expect("remove temp dir");
    }

    fn test_temp_dir(label: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "kleio-build-{label}-{}-{unique}",
            std::process::id()
        ))
    }
}
