#!/bin/sh
# Compile the Unity package's GcadCache.cs (plain .NET, no UnityEditor) with a .NET SDK and run it
# against gcad2glb: the cache, a hit, a deleted GLB, flags and an edited include. No Unity needed.
set -eu
root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
. "$root/scripts/env.sh"
dotnet=${DOTNET:-$(command -v dotnet || echo "$root/target/dotnet/dotnet")}
[ -x "$dotnet" ] || { echo 'needs dotnet 8 (DOTNET=..., or dotnet-install.sh --install-dir target/dotnet)' >&2; exit 1; }
tool=${GCAD2GLB:-"${CARGO_TARGET_DIR:-$GCAD_REPO/target}/release/gcad2glb"}
[ -x "$tool" ] || { echo 'Build gcad2glb or set GCAD2GLB' >&2; exit 1; }
project="$root/target/unity-cache-check"
build="$root/target/unity-cache-check-build"
rm -rf "$project" "$build"
mkdir -p "$project" "$build"
cp "$root/unity/cache-check/Check.cs" "$root/unity/Packages/io.v0l.gcad/Editor/GcadCache.cs" "$build/"
sed 's#../Packages/io.v0l.gcad/Editor/GcadCache.cs#GcadCache.cs#' "$root/unity/cache-check/cache-check.csproj" >"$build/cache-check.csproj"
DOTNET_CLI_TELEMETRY_OPTOUT=1 DOTNET_NOLOGO=1 "$dotnet" run --project "$build" -- "$tool" "$project"
