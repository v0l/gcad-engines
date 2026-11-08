#!/bin/sh
# Headless check of the Godot addon: builds scenes through addons/gcad/gcad.gd, then runs the
# editor's own import (--headless --import) on .gcad files so the EditorImportPlugin is exercised.
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$root/scripts/env.sh"
godot=${GODOT:-$(sh "$root/godot/fetch-godot.sh")}
tool=${GCAD2GLB:-}
if [ -z "$tool" ]; then
    for tool in "${CARGO_TARGET_DIR:-$GCAD_REPO/target}/release/gcad2glb" "$GCAD_REPO/target/release/gcad2glb"; do
        [ -x "$tool" ] && break
    done
fi
[ -x "$tool" ] || { echo 'Build gcad2glb (cargo build --release -p gcad2glb in the gcad checkout) or set GCAD2GLB' >&2; exit 1; }
project="$root/target/godot-addon"
rm -rf "$project"
mkdir -p "$project/addons" "$project/models"
cp -R "$root/godot/addons/gcad" "$project/addons/"
cp "$root/godot/check_addon.gd" "$project/"
cp "$fixtures/../mesh/rigged_creature.gcad" "$fixtures/../mesh/creature.gcad" "$fixtures/../parts/bracket.gcad" "$project/"
printf 'blob 0,0,0 r=2,2,2\nfrobnicate 3\n' >"$project/broken.gcad"
cp "$project/creature.gcad" "$project/models/creature.gcad"
cat >"$project/project.godot" <<EOF
config_version=5
[application]
config/name="gcad addon check"
[editor_plugins]
enabled=PackedStringArray("res://addons/gcad/plugin.cfg")
[gcad]
gcad2glb_path="$tool"
[rendering]
renderer/rendering_method="gl_compatibility"
EOF
GCAD2GLB=$tool "$godot" --headless --path "$project" --script res://check_addon.gd >"$project/addon.log" 2>&1 || true
grep -q '"passed": true' "$project/addon-report.json" || { cat "$project/addon-report.json" >&2; exit 1; }
echo 'godot addon: scenes built through gcad.gd'
rm "$project/broken.gcad" "$project/creature.gcad" "$project/rigged_creature.gcad" "$project/bracket.gcad"
"$godot" --headless --path "$project" --import >"$project/import.log" 2>&1 || true
imported=$(find "$project/.godot/imported" -name 'creature.gcad-*.scn' | head -1)
[ -n "$imported" ] || { tail -20 "$project/import.log" >&2; echo 'the editor did not import models/creature.gcad' >&2; exit 1; }
grep -q 'importer="gcad.scene"' "$project/models/creature.gcad.import"
echo "godot addon: editor imported models/creature.gcad to $(basename "$imported")"
