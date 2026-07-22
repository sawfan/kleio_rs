use std::path::{Path, PathBuf};

pub const WORKSPACE_CONFIG_FILE: &str = "kleio.toml";
pub const WORLD_CONFIG_FILE: &str = "world.toml";
pub const DEFAULT_WORLD_SLUG: &str = "default";

#[derive(Debug, Clone)]
pub struct WorkspacePaths {
    root: PathBuf,
}

impl WorkspacePaths {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn config(&self) -> PathBuf {
        self.root.join(WORKSPACE_CONFIG_FILE)
    }

    pub fn worlds_dir(&self) -> PathBuf {
        self.root.join("worlds")
    }

    pub fn world(&self, world_slug: &str) -> WorldPaths {
        WorldPaths::new(self.worlds_dir().join(world_slug))
    }
}

#[derive(Debug, Clone)]
pub struct WorldPaths {
    root: PathBuf,
}

impl WorldPaths {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn config(&self) -> PathBuf {
        self.root.join(WORLD_CONFIG_FILE)
    }

    pub fn entities_dir(&self) -> PathBuf {
        self.root.join("entities")
    }

    pub fn people_dir(&self) -> PathBuf {
        self.entities_dir().join("people")
    }

    pub fn places_dir(&self) -> PathBuf {
        self.entities_dir().join("places")
    }

    pub fn organizations_dir(&self) -> PathBuf {
        self.entities_dir().join("organizations")
    }

    pub fn objects_dir(&self) -> PathBuf {
        self.entities_dir().join("objects")
    }

    pub fn concepts_dir(&self) -> PathBuf {
        self.entities_dir().join("concepts")
    }

    pub fn events_dir(&self) -> PathBuf {
        self.root.join("events")
    }

    pub fn collections_dir(&self) -> PathBuf {
        self.root.join("collections")
    }

    pub fn event_kind_dir(&self, event_kind: &str) -> PathBuf {
        self.events_dir().join(event_kind)
    }

    pub fn births_dir(&self) -> PathBuf {
        self.event_kind_dir("births")
    }

    pub fn deaths_dir(&self) -> PathBuf {
        self.event_kind_dir("deaths")
    }

    pub fn residences_dir(&self) -> PathBuf {
        self.event_kind_dir("residences")
    }

    pub fn marriages_dir(&self) -> PathBuf {
        self.event_kind_dir("marriages")
    }

    pub fn migrations_dir(&self) -> PathBuf {
        self.event_kind_dir("migrations")
    }

    pub fn observations_dir(&self) -> PathBuf {
        self.event_kind_dir("observations")
    }

    pub fn moments_dir(&self) -> PathBuf {
        self.event_kind_dir("moments")
    }

    pub fn other_events_dir(&self) -> PathBuf {
        self.event_kind_dir("other")
    }

    pub fn assertions_dir(&self) -> PathBuf {
        self.root.join("assertions")
    }

    pub fn relationships_dir(&self) -> PathBuf {
        self.root.join("relationships")
    }

    pub fn sources_dir(&self) -> PathBuf {
        self.root.join("sources")
    }

    pub fn imports_dir(&self) -> PathBuf {
        self.root.join("imports")
    }

    pub fn gedcom_imports_dir(&self) -> PathBuf {
        self.imports_dir().join("gedcom")
    }

    pub fn wikidata_imports_dir(&self) -> PathBuf {
        self.imports_dir().join("wikidata")
    }

    pub fn csv_imports_dir(&self) -> PathBuf {
        self.imports_dir().join("csv")
    }

    pub fn media_dir(&self) -> PathBuf {
        self.root.join("media")
    }

    pub fn media_people_dir(&self) -> PathBuf {
        self.media_dir().join("people")
    }

    pub fn media_places_dir(&self) -> PathBuf {
        self.media_dir().join("places")
    }

    pub fn media_sources_dir(&self) -> PathBuf {
        self.media_dir().join("sources")
    }

    pub fn media_events_dir(&self) -> PathBuf {
        self.media_dir().join("events")
    }

    pub fn views_dir(&self) -> PathBuf {
        self.root.join("views")
    }

    pub fn timeline_views_dir(&self) -> PathBuf {
        self.views_dir().join("timelines")
    }

    pub fn tree_views_dir(&self) -> PathBuf {
        self.views_dir().join("trees")
    }

    pub fn map_views_dir(&self) -> PathBuf {
        self.views_dir().join("maps")
    }

    pub fn calendar_views_dir(&self) -> PathBuf {
        self.views_dir().join("calendars")
    }

    pub fn visualization_views_dir(&self) -> PathBuf {
        self.views_dir().join("visualizations")
    }

    pub fn schemas_dir(&self) -> PathBuf {
        self.root.join("schemas")
    }

    pub fn component_schemas_dir(&self) -> PathBuf {
        self.schemas_dir().join("components")
    }

    pub fn bundle_schemas_dir(&self) -> PathBuf {
        self.schemas_dir().join("bundles")
    }

    pub fn event_schemas_dir(&self) -> PathBuf {
        self.schemas_dir().join("events")
    }

    pub fn view_schemas_dir(&self) -> PathBuf {
        self.schemas_dir().join("views")
    }

    pub fn vocab_schemas_dir(&self) -> PathBuf {
        self.schemas_dir().join("vocab")
    }

    pub fn systems_dir(&self) -> PathBuf {
        self.root.join("systems")
    }

    pub fn importer_systems_dir(&self) -> PathBuf {
        self.systems_dir().join("importers")
    }

    pub fn compiler_systems_dir(&self) -> PathBuf {
        self.systems_dir().join("compilers")
    }

    pub fn validator_systems_dir(&self) -> PathBuf {
        self.systems_dir().join("validators")
    }

    pub fn renderer_systems_dir(&self) -> PathBuf {
        self.systems_dir().join("renderers")
    }

    pub fn build_dir(&self) -> PathBuf {
        self.root.join("build")
    }

    pub fn compiled_json(&self) -> PathBuf {
        self.build_dir().join("kleio.compiled.json")
    }

    pub fn ecs_json(&self) -> PathBuf {
        self.build_dir().join("kleio.ecs.json")
    }

    pub fn sqlite(&self) -> PathBuf {
        self.build_dir().join("kleio.sqlite")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_workspace_and_world_paths() {
        let workspace = WorkspacePaths::new("/tmp/kleio-workspace");
        assert_eq!(workspace.root(), Path::new("/tmp/kleio-workspace"));
        assert_eq!(
            workspace.config(),
            PathBuf::from("/tmp/kleio-workspace/kleio.toml")
        );
        assert_eq!(
            workspace.worlds_dir(),
            PathBuf::from("/tmp/kleio-workspace/worlds")
        );
        assert_eq!(
            workspace.world("default").root(),
            Path::new("/tmp/kleio-workspace/worlds/default")
        );
    }

    #[test]
    fn exposes_world_semantic_view_schema_system_and_build_paths() {
        let world = WorldPaths::new("/tmp/kleio-workspace/worlds/default");
        assert_eq!(
            world.config(),
            PathBuf::from("/tmp/kleio-workspace/worlds/default/world.toml")
        );
        assert_eq!(
            world.people_dir(),
            PathBuf::from("/tmp/kleio-workspace/worlds/default/entities/people")
        );
        assert_eq!(
            world.places_dir(),
            PathBuf::from("/tmp/kleio-workspace/worlds/default/entities/places")
        );
        assert_eq!(
            world.assertions_dir(),
            PathBuf::from("/tmp/kleio-workspace/worlds/default/assertions")
        );
        assert_eq!(
            world.sources_dir(),
            PathBuf::from("/tmp/kleio-workspace/worlds/default/sources")
        );
        assert_eq!(
            world.gedcom_imports_dir(),
            PathBuf::from("/tmp/kleio-workspace/worlds/default/imports/gedcom")
        );
        assert_eq!(
            world.media_people_dir(),
            PathBuf::from("/tmp/kleio-workspace/worlds/default/media/people")
        );
        assert_eq!(
            world.timeline_views_dir(),
            PathBuf::from("/tmp/kleio-workspace/worlds/default/views/timelines")
        );
        assert_eq!(
            world.visualization_views_dir(),
            PathBuf::from("/tmp/kleio-workspace/worlds/default/views/visualizations")
        );
        assert_eq!(
            world.component_schemas_dir(),
            PathBuf::from("/tmp/kleio-workspace/worlds/default/schemas/components")
        );
        assert_eq!(
            world.importer_systems_dir(),
            PathBuf::from("/tmp/kleio-workspace/worlds/default/systems/importers")
        );
        assert_eq!(
            world.compiled_json(),
            PathBuf::from("/tmp/kleio-workspace/worlds/default/build/kleio.compiled.json")
        );
        assert_eq!(
            world.ecs_json(),
            PathBuf::from("/tmp/kleio-workspace/worlds/default/build/kleio.ecs.json")
        );
        assert_eq!(
            world.sqlite(),
            PathBuf::from("/tmp/kleio-workspace/worlds/default/build/kleio.sqlite")
        );
    }
}
