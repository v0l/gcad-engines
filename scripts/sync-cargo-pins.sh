#!/bin/sh
# Point every Cargo manifest here at GCAD_REV and copy gcad's [patch.crates-io] table under it.
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$root/scripts/env.sh"
rev=$gcad_rev
[ "$(git -C "$GCAD_REPO" rev-parse HEAD)" = "$rev" ] || { echo "$GCAD_REPO is not at GCAD_REV $rev" >&2; exit 1; }
patches=$(awk '/^\[patch\.crates-io\]/{on=1;next} /^\[/{on=0} on && NF' "$GCAD_REPO/Cargo.toml")
for manifest in "$root/native-tests/Cargo.toml" "$root/bevy/Cargo.toml"; do
    [ -f "$manifest" ] || continue
    tmp="$manifest.tmp"
    awk -v rev="$rev" -v patches="$patches" '
        /^\[patch\.crates-io\]/ { print; print patches; skip=1; next }
        /^\[/ { skip=0 }
        skip { next }
        { gsub(/github\.com\/v0l\/gcad", rev = "[0-9a-f]+"/, "github.com/v0l/gcad\", rev = \"" rev "\""); print }
    ' "$manifest" >"$tmp"
    mv "$tmp" "$manifest"
    printf 'pinned %s to gcad %s\n' "${manifest#"$root"/}" "$rev"
done
