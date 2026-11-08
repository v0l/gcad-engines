#!/bin/sh
# Prepare both bindings of the skinned consumer fixture without requiring either to exist already.
set -eu
root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
. "$root/scripts/env.sh"
find_gcad
cd "$GCAD_REPO"
fixture=examples/game-assets/plate-consumer-skinned.gasset
mkdir -p target
for level in high low; do
    output="target/consumer-skinned-$level.weights.json"
    if [ ! -e "$output" ]; then
        "$gcad" asset-bind "$fixture" "detail_$level" "$output" --joints base,tip --blend y,-15,15
    fi
done
"$gcad" check "$fixture"
