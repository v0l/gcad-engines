#!/bin/sh
# Launch the game-asset mode of the existing application .
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
cd "$root"
if [ "$#" -gt 1 ]; then
    printf '%s\n' 'Usage: sh mvp.sh [ASSET.gasset]' >&2
    exit 1
fi
gcad=${GCAD:?set GCAD to a gcad built with --features game-assets}
helper="$root/target/game-assets-tools/material-extension-v12-build/Release/gcad_fbx_helper.exe"
if [ -z "${GCAD_FBX_HELPER:-}" ] && [ -f "$helper" ]; then
    if command -v cygpath >/dev/null 2>&1; then helper=$(cygpath -am "$helper"); fi
    export GCAD_FBX_HELPER="$helper"
fi
exec "$gcad" view "${1:?pass a .gasset file}"
