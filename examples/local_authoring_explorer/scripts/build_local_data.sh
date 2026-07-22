#!/usr/bin/env sh
set -eu

cargo run -q --manifest-path ../../../kleio-cli/Cargo.toml --bin kleio-cli -- \
  build data \
  --timeline-view example-life \
  --tree-view main-family-tree
