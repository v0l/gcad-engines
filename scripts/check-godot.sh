#!/bin/sh
# Export reference fixtures with gcad and run the headless Godot import check on each, plus a
# truncated GLB that must be refused. Uses GODOT, or a portable copy from godot/fetch-godot.sh.
set -eu
root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
. "$root/scripts/env.sh"
find_gcad
godot=${GODOT:-$(sh "$root/godot/fetch-godot.sh")}
version=$("$godot" --headless --version | tail -1)
work="$root/target/godot-check"
rm -rf "$work"
mkdir -p "$work"
export_glb() {
    (cd "$GCAD_REPO" && "$gcad" export "$1" "$work/$2.glb" >/dev/null)
}
export_glb examples/game-assets/plate-consumer-profile.gasset plate-consumer-profile
export_glb examples/game-assets/plate-consumer-skinned.gasset plate-consumer-skinned
export_glb examples/game-assets/creature.gasset creature
export_glb examples/mesh/rigged_creature.gcad rigged_creature
for name in plate-consumer-profile plate-consumer-skinned creature rigged_creature; do
    GODOT=$godot GODOT_VERSION=$version sh "$root/godot/check-import.sh" "$work/$name.glb" "$work/$name"
    printf 'godot %s: %s passed\n' "$version" "$name"
done
head -c 3000 "$work/creature.glb" >"$work/truncated.glb"
if GODOT=$godot GODOT_VERSION=$version sh "$root/godot/check-import.sh" "$work/truncated.glb" "$work/truncated" 2>/dev/null; then
    echo 'truncated GLB was accepted' >&2
    exit 1
fi
grep -q 'Invalid GLB' "$work/truncated/import-report.json"
printf 'godot %s: truncated GLB refused\n' "$version"
