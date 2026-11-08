#!/bin/sh
# Check out gcad at GCAD_REV into target/gcad and, with --build, build it with game assets.
set -eu
root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
rev=$(cat "$root/GCAD_REV")
url=${GCAD_URL:-https://github.com/v0l/gcad}
dest=${GCAD_REPO:-"$root/target/gcad"}
if [ ! -d "$dest/.git" ]; then
    mkdir -p "$(dirname -- "$dest")"
    git clone --quiet --filter=blob:none --no-checkout "$url" "$dest"
fi
git -C "$dest" cat-file -e "$rev^{commit}" 2>/dev/null || git -C "$dest" fetch --quiet origin
git -C "$dest" -c advice.detachedHead=false checkout --quiet "$rev"
printf 'gcad %s at %s\n' "$rev" "$dest"
if [ "${1:-}" = --build ]; then
    (cd "$dest" && cargo build --locked --release --features game-assets -p gcad)
    printf 'binary: %s/release/gcad\n' "${CARGO_TARGET_DIR:-$dest/target}"
fi
