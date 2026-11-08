#!/bin/sh
# Open a .gasset in the gcad game-asset viewer, with the FBX helper built here when there is one.
set -eu
root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
if [ "$#" -ne 1 ]; then
    printf '%s\n' 'Usage: sh scripts/mvp.sh ASSET.gasset' >&2
    exit 1
fi
. "$root/scripts/env.sh"
find_gcad
if [ -z "${GCAD_FBX_HELPER:-}" ]; then
    for helper in "$root/target/game-assets-tools/material-extension-v12-build/Release/gcad_fbx_helper.exe" \
        "$root/target/game-assets-tools/material-extension-v12-build/gcad_fbx_helper"; do
        if [ -f "$helper" ]; then
            if command -v cygpath >/dev/null 2>&1; then helper=$(cygpath -am "$helper"); fi
            export GCAD_FBX_HELPER="$helper"
            break
        fi
    done
fi
asset=$1
[ -f "$asset" ] || asset="$fixtures/$1"
[ -f "$asset" ] || { echo "No such asset: $1" >&2; exit 1; }
if [ -n "${GCAD_MVP_CHECK:-}" ]; then
    exec "$gcad" check "$asset"
fi
exec "$gcad" view "$asset"
