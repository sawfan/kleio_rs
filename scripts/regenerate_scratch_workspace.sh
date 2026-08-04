#!/usr/bin/env sh
set -eu

KLEIO_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
WORKSPACE_ROOT=$(CDPATH= cd -- "$KLEIO_DIR/../.." && pwd)
WORKSPACE_DIR="${1:-$HOME/kleio-scratch}"
PERSON_SLUG="${KLEIO_PERSON_SLUG:-alex-example}"
PERSON_NAME="${KLEIO_PERSON_NAME:-Alex Example}"
BIRTH_DATE="${KLEIO_BIRTH_DATE:-1900-01-01 07:18}"
BIRTH_LOCATION="${KLEIO_BIRTH_LOCATION:-Example Town Birth Center}"
BIRTH_LATITUDE="${KLEIO_BIRTH_LATITUDE:-12.345}"
BIRTH_LONGITUDE="${KLEIO_BIRTH_LONGITUDE:--67.89}"

if [ -e "$WORKSPACE_DIR" ]; then
  case "$WORKSPACE_DIR" in
    "$HOME"|"$HOME/"|"/"|"$KLEIO_DIR"|"$KLEIO_DIR/"|"$WORKSPACE_ROOT"|"$WORKSPACE_ROOT/")
      echo "Refusing to remove unsafe workspace path: $WORKSPACE_DIR" >&2
      exit 1
      ;;
  esac
  rm -rf "$WORKSPACE_DIR"
fi

cargo run -q --manifest-path "$WORKSPACE_ROOT/crates/kleio-cli/Cargo.toml" --bin kleio-cli -- \
  init-workspace "$WORKSPACE_DIR" \
  --person-slug "$PERSON_SLUG" \
  --person-name "$PERSON_NAME" \
  --birth-date "$BIRTH_DATE" \
  --birth-location "$BIRTH_LOCATION" \
  --birth-latitude="$BIRTH_LATITUDE" \
  --birth-longitude="$BIRTH_LONGITUDE" \
  --force

cargo run -q --manifest-path "$WORKSPACE_ROOT/crates/kleio-cli/Cargo.toml" --bin kleio-cli -- \
  build "$WORKSPACE_DIR" \
  --timeline-view example-life \
  --tree-view main-family-tree

cat <<EOF

Regenerated scratch Kleio workspace:
  $WORKSPACE_DIR

Starter files to edit:
  $WORKSPACE_DIR/worlds/default/entities/people/$PERSON_SLUG.md
  $WORKSPACE_DIR/worlds/default/events/births/$BIRTH_DATE-birth-$PERSON_SLUG.md

Generated outputs:
  $WORKSPACE_DIR/worlds/default/build/kleio.compiled.json
  $WORKSPACE_DIR/worlds/default/build/kleio.ecs.json
  $WORKSPACE_DIR/worlds/default/build/example-life.timeline.json
  $WORKSPACE_DIR/worlds/default/build/main-family-tree.tree.json
EOF
