#!/bin/sh
# Run a windowed example inside a private headless sway and save a screenshot. Never touches the
# user's display: env -i, a private XDG_RUNTIME_DIR, the headless wlroots backend, no Xwayland.
set -eu
[ "$#" -ge 2 ] || { echo 'Usage: sh scripts/headless-shot.sh OUTPUT.png COMMAND [ARGS]' >&2; exit 1; }
out=$1; shift
rt=$(mktemp -d /tmp/agent-wl.XXXX); chmod 700 "$rt"
printf 'xwayland disable\noutput HEADLESS-1 resolution 1280x720\ndefault_border none\n' >"$rt/config"
E="env -i HOME=$HOME PATH=$PATH XDG_RUNTIME_DIR=$rt GCAD_REPO=${GCAD_REPO:-} RUST_LOG=${RUST_LOG:-warn}"
$E WLR_BACKENDS=headless WLR_LIBINPUT_NO_DEVICES=1 sway -c "$rt/config" >"$rt/sway.log" 2>&1 &
sway_pid=$!
sleep 2
$E WAYLAND_DISPLAY=wayland-1 "$@" >"$rt/app.log" 2>&1 &
app_pid=$!
sleep "${SHOT_WAIT:-12}"
$E WAYLAND_DISPLAY=wayland-1 grim "$out" || true
kill "$app_pid" 2>/dev/null || true
kill "$sway_pid" 2>/dev/null || true
wait "$app_pid" 2>/dev/null || true
cp "$rt/app.log" "$out.log"
rm -rf "$rt"
[ -s "$out" ]
