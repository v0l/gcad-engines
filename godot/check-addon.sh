#!/bin/sh
# Headless check of the Godot addon: builds scenes through addons/gcad/gcad.gd, then runs the
# editor's own import (--headless --import) on .gcad files so the EditorImportPlugin is exercised.
set -eu
root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
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
printf 'include included_body.gcad\n' >"$project/included.gcad"
printf 'blob 0,0,0 r=2,2,2\nremesh 0.6\n' >"$project/included_body.gcad"
cp "$project/creature.gcad" "$project/models/creature.gcad"
log="$project/gcad2glb-calls.log"
wrapper="$project/gcad2glb-logged"
printf '#!/bin/sh\nprintf "%%s\\n" "$*" >>"%s"\nexec "%s" "$@"\n' "$log" "$tool" >"$wrapper"
chmod +x "$wrapper"
cat >"$project/project.godot" <<EOF
config_version=5
[application]
config/name="gcad addon check"
[editor_plugins]
enabled=PackedStringArray("res://addons/gcad/plugin.cfg")
[gcad]
gcad2glb_path="$wrapper"
[rendering]
renderer/rendering_method="gl_compatibility"
EOF
GCAD2GLB=$tool "$godot" --headless --path "$project" --script res://check_addon.gd >"$project/addon.log" 2>&1 || true
grep -q '"passed": true' "$project/addon-report.json" || { cat "$project/addon-report.json" >&2; exit 1; }
echo 'godot addon: scenes built through gcad.gd; a second build of an included model was a cache hit'
rm -r "$project/broken.gcad" "$project/creature.gcad" "$project/rigged_creature.gcad" "$project/bracket.gcad" \
    "$project/included.gcad" "$project/included_body.gcad" "$project/cache-check"
"$godot" --headless --path "$project" --import >"$project/import.log" 2>&1 || true
imported=$(find "$project/.godot/imported" -name 'creature.gcad-*.scn' | head -1)
[ -n "$imported" ] || { tail -20 "$project/import.log" >&2; echo 'the editor did not import models/creature.gcad' >&2; exit 1; }
grep -q 'importer="gcad.scene"' "$project/models/creature.gcad.import"
echo "godot addon: editor imported models/creature.gcad to $(basename "$imported")"
cache="$project/.godot/gcad-cache"
key=$("$tool" key "$project/models/creature.gcad")
[ -f "$cache/$key.glb" ] || { ls -la "$cache" >&2; echo "no $key.glb in .godot/gcad-cache" >&2; exit 1; }
grep -q 'models/creature.gcad built' "$project/import.log" || { tail -20 "$project/import.log" >&2; exit 1; }
rm -rf "$project/.godot/imported" "$project/models/creature.gcad.import"
"$godot" --headless --path "$project" --import >"$project/import2.log" 2>&1 || true
[ -n "$(find "$project/.godot/imported" -name 'creature.gcad-*.scn' | head -1)" ] || { tail -20 "$project/import2.log" >&2; exit 1; }
grep -q "models/creature.gcad cached $key" "$project/import2.log" || { tail -20 "$project/import2.log" >&2; echo 'the second import built again' >&2; exit 1; }
echo "godot addon: a clean re-import of models/creature.gcad used .godot/gcad-cache/$key.glb, no build"
printf 'blob 0,0,0 r=2,2,5\nremesh 0.6\n' >"$project/models/creature.gcad"
"$godot" --headless --path "$project" --import >"$project/import3.log" 2>&1 || true
edited=$("$tool" key "$project/models/creature.gcad")
[ "$edited" != "$key" ] && grep -q "models/creature.gcad built $edited" "$project/import3.log" \
    || { tail -20 "$project/import3.log" >&2; echo 'an edited source was not rebuilt' >&2; exit 1; }
echo "godot addon: an edit rebuilt models/creature.gcad as $edited"
printf 'include body/shape.gcad\n' >"$project/models/hero.gcad"
mkdir -p "$project/models/body"
printf 'blob 0,0,0 r=2,2,2\nremesh 0.6\n' >"$project/models/body/shape.gcad"
"$godot" --headless --path "$project" --import >"$project/import4.log" 2>&1 || true
hero=$("$tool" key "$project/models/hero.gcad")
grep -q "models/hero.gcad built $hero" "$project/import4.log" || { tail -20 "$project/import4.log" >&2; exit 1; }
printf 'blob 0,0,0 r=2,2,6\nremesh 0.6\n' >"$project/models/body/shape.gcad"
"$godot" --headless --editor --quit-after 30 --path "$project" >"$project/editor.log" 2>&1 || true
reheroed=$("$tool" key "$project/models/hero.gcad")
grep -q "models/hero.gcad built $reheroed" "$project/editor.log" \
    || { tail -30 "$project/editor.log" >&2; echo 'editing an include did not re-import the model that includes it' >&2; exit 1; }
echo "godot addon: editing models/body/shape.gcad re-imported models/hero.gcad as $reheroed"
printf 'gcad2glb calls: %s\n' "$(grep -c . "$log")"
