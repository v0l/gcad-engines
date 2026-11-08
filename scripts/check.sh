#!/bin/sh
# Every check that runs on Linux, in order. Each part can be skipped with SKIP="fbx renderer ...".
set -eu
root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
. "$root/scripts/env.sh"
skip=" ${SKIP:-} "
want() { case "$skip" in *" $1 "*) echo "skip $1"; return 1;; esac; echo "== $1"; }
if want syntax; then
    for script in "$root"/scripts/*.sh "$root"/godot/*.sh "$root"/unity/*.sh "$root"/fbx-helper/*.sh; do sh -n "$script"; done
    if command -v shellcheck >/dev/null 2>&1; then shellcheck -S warning "$root"/scripts/*.sh "$root"/godot/*.sh; fi
fi
if want fixtures; then
    sh "$root/scripts/game-assets.sh" prepare-fixtures >/dev/null
    GCAD_MVP_CHECK=1 sh "$root/scripts/mvp.sh" plate.gasset
fi
if want godot; then sh "$root/scripts/check-godot.sh"; sh "$root/godot/check-addon.sh"; fi
if want fbx; then sh "$root/scripts/check-fbx.sh"; fi
if want bevy; then
    (cd "$root/bevy" && cargo test --release && cargo test --release --features brep &&
        cargo check --release --lib --target wasm32-unknown-unknown)
fi
echo 'all checks passed'
