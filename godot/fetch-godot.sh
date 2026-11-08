#!/bin/sh
# Download a portable Linux Godot into target/godot. Installs nothing system-wide.
set -eu
root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
version=${GODOT_RELEASE:-4.7.2-stable}
case "$(uname -m)" in
x86_64) arch=linux.x86_64 ;;
aarch64) arch=linux.arm64 ;;
*) echo "No portable Godot for $(uname -m)" >&2; exit 1 ;;
esac
dest="$root/target/godot"
binary="$dest/Godot_v${version}_$arch"
if [ ! -x "$binary" ]; then
    mkdir -p "$dest"
    url="https://github.com/godotengine/godot/releases/download/$version/Godot_v${version}_$arch.zip"
    curl -fsSL -o "$dest/godot.zip" "$url"
    (cd "$dest" && unzip -oq godot.zip && rm godot.zip)
    chmod +x "$binary"
fi
printf '%s\n' "$binary"
