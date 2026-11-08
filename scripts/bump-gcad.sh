#!/bin/sh
# Move GCAD_REV to the latest gcad master, repin the Cargo manifests here, and run the checks
# that run on this machine. Leaves the change uncommitted for review.
set -eu
root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
url=${GCAD_URL:-https://github.com/v0l/gcad}
rev=${1:-$(git ls-remote "$url" refs/heads/master | cut -f1)}
[ -n "$rev" ] || { echo "Could not read gcad master from $url" >&2; exit 1; }
old=$(cat "$root/GCAD_REV")
printf '%s\n' "$rev" >"$root/GCAD_REV"
export GCAD_REPO=${GCAD_REPO:-$root/target/gcad}
sh "$root/scripts/fetch-gcad.sh" --build
sh "$root/scripts/sync-cargo-pins.sh"
cp "$GCAD_REPO/Cargo.lock" "$root/native-tests/Cargo.lock"
export GCAD="${CARGO_TARGET_DIR:-$GCAD_REPO/target}/release/gcad"
(cd "$GCAD_REPO" && cargo build --release -p gcad2glb)
sh "$root/scripts/check.sh"
printf 'GCAD_REV %s -> %s; checks pass. Review and commit GCAD_REV, */Cargo.toml and native-tests/Cargo.lock.\n' "$old" "$rev"
