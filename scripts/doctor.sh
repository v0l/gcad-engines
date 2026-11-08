#!/bin/sh
# Report which integrations can run on this machine. Exits non-zero only when a part named
# on the command line is missing: sh scripts/doctor.sh gcad godot fbx
set -u
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
want=" $* "
missing=0
report() {
    part=$1; ok=$2; detail=$3
    if [ "$ok" = yes ]; then mark=ok; else mark=no; fi
    printf '%-24s %-3s %s\n' "$part" "$mark" "$detail"
    case "$want" in *" $part "*) [ "$ok" = yes ] || missing=1 ;; esac
}
have() { command -v "$1" >/dev/null 2>&1; }

rev=$(cat "$root/GCAD_REV")
repo=${GCAD_REPO:-}
[ -n "$repo" ] || { [ -d "$root/../gcad/examples" ] && repo="$root/../gcad"; } || true
[ -n "$repo" ] || { [ -d "$root/target/gcad/examples" ] && repo="$root/target/gcad"; } || true
if [ -n "$repo" ] && [ -d "$repo/examples/game-assets" ]; then
    at=$(git -C "$repo" rev-parse HEAD 2>/dev/null || echo unknown)
    if [ "$at" = "$rev" ]; then note="at GCAD_REV"; else note="at $at, GCAD_REV is $rev"; fi
    report repo yes "$repo ($note)"
else
    report repo no "no gcad checkout: set GCAD_REPO or run sh scripts/fetch-gcad.sh"
fi

gcad=${GCAD:-}
if [ -z "$gcad" ] && [ -n "$repo" ]; then
    for candidate in "${CARGO_TARGET_DIR:-$repo/target}/release/gcad" "$repo/target/release/gcad" "$repo/target/debug/gcad"; do
        [ -x "$candidate" ] && { gcad=$candidate; break; }
    done
fi
[ -n "$gcad" ] || gcad=$(command -v gcad 2>/dev/null || true)
if [ -n "$gcad" ] && [ -x "$gcad" ]; then
    version=$("$gcad" --version 2>/dev/null)
    if "$gcad" asset-capabilities --help >/dev/null 2>&1; then
        report gcad yes "$gcad ($version, game-assets)"
    else
        report gcad no "$gcad ($version) lacks --features game-assets"
    fi
else
    report gcad no "no gcad binary: set GCAD or sh scripts/fetch-gcad.sh --build"
fi

if have cargo; then report cargo yes "$(cargo --version)"; else report cargo no "cargo not on PATH"; fi

for t in wasm32-unknown-unknown aarch64-linux-android; do
    if have rustup && rustup target list --installed 2>/dev/null | grep -qx "$t"; then
        report "$t" yes "rust target installed"
    else
        report "$t" no "rustup target add $t"
    fi
done
ndk=${ANDROID_NDK_HOME:-}
if [ -z "$ndk" ] && [ -d "${ANDROID_HOME:-$HOME/Android/Sdk}/ndk" ]; then
    ndk=$(ls -d "${ANDROID_HOME:-$HOME/Android/Sdk}"/ndk/* 2>/dev/null | tail -1)
fi
if [ -n "$ndk" ] && [ -d "$ndk" ] && have cargo-ndk; then report android yes "NDK $ndk with cargo-ndk"; else report android no "needs an Android NDK (ANDROID_NDK_HOME) and cargo-ndk"; fi

if have "${CMAKE:-cmake}"; then report cmake yes "$("${CMAKE:-cmake}" --version | head -1)"; else report cmake no "cmake not on PATH (set CMAKE)"; fi
tools="$root/target/game-assets-tools"
assimp=${ASSIMP_SOURCE:-$tools/assimp}; ufbx=${UFBX_SOURCE:-$tools/ufbx}
if [ -d "$assimp/.git" ] && [ -d "$ufbx/.git" ]; then
    report fbx yes "Assimp $(git -C "$assimp" rev-parse --short HEAD), ufbx $(git -C "$ufbx" rev-parse --short HEAD)"
else
    report fbx no "no Assimp/ufbx sources: sh scripts/game-assets.sh fetch-sources"
fi
helper=${GCAD_FBX_HELPER:-}
[ -n "$helper" ] || for candidate in "$tools/material-extension-v12-build/gcad_fbx_helper" "$tools/material-extension-v12-build/Release/gcad_fbx_helper.exe"; do
    [ -f "$candidate" ] && { helper=$candidate; break; }
done
if [ -n "$helper" ] && [ -f "$helper" ]; then report helper yes "$helper"; else report helper no "not built: sh scripts/game-assets.sh build-probes Release v12"; fi

godot=${GODOT:-}
[ -n "$godot" ] || godot=$(command -v godot4 2>/dev/null || command -v godot 2>/dev/null || true)
[ -n "$godot" ] || for candidate in "$root"/target/godot/Godot_v4*; do [ -x "$candidate" ] && godot=$candidate; done
if [ -n "$godot" ] && [ -x "$godot" ]; then report godot yes "$godot ($("$godot" --headless --version 2>/dev/null | tail -1))"; else report godot no "set GODOT, or sh godot/fetch-godot.sh for a portable copy in target/godot"; fi

unity=${UNITY:-}
if [ -z "$unity" ]; then
    for candidate in "$HOME"/Unity/Hub/Editor/*/Editor/Unity "/c/Program Files/Unity/Hub/Editor"/*/Editor/Unity.exe /Applications/Unity/Hub/Editor/*/Unity.app/Contents/MacOS/Unity; do
        [ -x "$candidate" ] && unity=$candidate
    done
fi
if [ -n "$unity" ] && [ -x "$unity" ]; then report unity yes "$unity"; else report unity no "no Unity editor: set UNITY (Unity 6000.4 or 6000.5)"; fi

case "$(uname -s)" in
MINGW*|MSYS*|CYGWIN*) report windows yes "$(uname -s)" ;;
*) if have pwsh; then report windows no "not Windows; pwsh is present for parsing the .ps1 helpers"; else report windows no "not Windows ($(uname -s))"; fi ;;
esac
if have shellcheck; then report shellcheck yes "$(shellcheck --version | sed -n 2p)"; else report shellcheck no "shellcheck not on PATH"; fi
exit "$missing"
