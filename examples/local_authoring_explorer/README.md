# Kleio local authoring explorer

This is a small Trunk/WASM example for exploring Kleio local authoring data before wiring the model into a downstream application.

The authored data lives under `data/worlds/default` and is intentionally fictional. Edit those Markdown/TOML files, then refresh the Trunk app to inspect the generated JSON outputs.

## Run

From this directory:

```sh
trunk serve
```

The Trunk `pre_build` hook runs:

```sh
cargo run -q --manifest-path ../../../kleio-cli/Cargo.toml --bin kleio-cli -- build data --timeline-view example-life --tree-view main-family-tree
```

or equivalently:

```sh
scripts/build_local_data.sh
```

That writes generated files under `data/worlds/default/build/`, which Trunk copies into the served app as `build/`.

## Manual build

If you want to compile the authored files without starting Trunk:

```sh
cargo run -p kleio-cli_rs --bin kleio-cli -- build crates/kleio/examples/local_authoring_explorer/data --timeline-view example-life --tree-view main-family-tree
```

## Notes

A browser app cannot directly read or write the standard Kleio data path (`$XDG_DATA_HOME/kleio`, usually `~/.local/share/kleio`). This example uses a repo-local fictional workspace and treats the browser as a read-only explorer for generated JSON.
