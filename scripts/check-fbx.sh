#!/bin/sh
# Build the FBX helper, run its CMake tests, convert the reference fixtures and run the Rust
# process tests in native-tests/ against it. Needs cmake, a C++17 compiler, git and cargo.
set -eu
root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
. "$root/scripts/env.sh"
find_gcad
tools="$root/target/game-assets-tools"
build="$tools/material-extension-v12-build"
[ -d "$tools/assimp/.git" ] && [ -d "$tools/ufbx/.git" ] || sh "$root/scripts/game-assets.sh" fetch-sources
GCAD_HELPER_PROTOCOL_TESTS=ON GCAD_FIXTURES="$fixtures" sh "$root/scripts/game-assets.sh" build-probes Release v12
helper="$build/gcad_fbx_helper"
[ -f "$helper" ] || helper="$build/Release/gcad_fbx_helper.exe"
fixture="$build/gcad_fbx_protocol_fixture"
[ -f "$fixture" ] || fixture="$build/Release/gcad_fbx_protocol_fixture.exe"
(cd "$build" && ctest --output-on-failure -C Release)
"$gcad" asset-fbx-helper --helper "$helper" >/dev/null
work="$root/target/fbx-check"
rm -rf "$work"
mkdir -p "$work"
for name in plate plate-consumer-skinned plate-scene-refined; do
    (cd "$GCAD_REPO" && "$gcad" export "examples/game-assets/$name.gasset" "$work/$name.glb" >/dev/null)
    "$gcad" asset-fbx-experimental "$work/$name.glb" "$work/$name" --helper "$helper" --timeout-seconds 120 >/dev/null
    printf 'converted %s: %s\n' "$name" "$(ls "$work/$name" | tr '\n' ' ')"
done
cd "$root/native-tests"
GCAD_FBX_HELPER=$helper GCAD_FBX_PROTOCOL_FIXTURE=$fixture \
    GCAD_FBX_INPUT="$work/plate-scene-refined.glb" GCAD_FBX_MODEL="$work/plate-scene-refined/model.fbx" \
    cargo test --release -- --ignored --test-threads=1
