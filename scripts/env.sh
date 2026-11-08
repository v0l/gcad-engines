#!/bin/sh
# shellcheck shell=sh disable=SC2034,SC2154
# Shared paths for the scripts in this repo. Source it after setting `root`.
gcad_rev=$(cat "$root/GCAD_REV")
if [ -z "${GCAD_REPO:-}" ]; then
    if [ -d "$root/../gcad/examples" ]; then
        GCAD_REPO="$root/../gcad"
    else
        GCAD_REPO="$root/target/gcad"
    fi
fi
[ -d "$GCAD_REPO/examples" ] || { echo "No gcad checkout at $GCAD_REPO: set GCAD_REPO or run sh scripts/fetch-gcad.sh" >&2; exit 1; }
GCAD_REPO=$(CDPATH='' cd -- "$GCAD_REPO" && pwd)
export GCAD_REPO
fixtures="$GCAD_REPO/examples/game-assets"
find_gcad() {
    if [ -n "${GCAD:-}" ]; then
        gcad=$GCAD
        [ -f "$gcad" ] || gcad="$gcad.exe"
    else
        for gcad in "${CARGO_TARGET_DIR:-$GCAD_REPO/target}/release/gcad" "$GCAD_REPO/target/release/gcad" "$GCAD_REPO/target/debug/gcad"; do
            [ -f "$gcad" ] && break
        done
    fi
    [ -f "$gcad" ] || { echo 'Build gcad with --features game-assets (sh scripts/fetch-gcad.sh --build), or set GCAD' >&2; exit 1; }
    "$gcad" asset-capabilities --help >/dev/null 2>&1 || { echo "$gcad was built without --features game-assets" >&2; exit 1; }
}
