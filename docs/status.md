# Status

What runs where, against gcad `GCAD_REV` (888da75). Recorded 2026-11-08 on Linux x86_64
(Debian, 48 cores, NVIDIA RTX PRO 6000, cmake 3.31.6, g++ 14, Rust 1.99). `sh scripts/doctor.sh`
prints what the current machine can run.

| integration | run here | result | not run here, and why |
|---|---|---|---|
| gcad build | `cargo build --release --features game-assets -p gcad` at GCAD_REV (`scripts/fetch-gcad.sh --build`) | builds in 2 min 4 s; `--locked` fails because gcad's Cargo.lock at 888da75 is stale | |
| fixtures | `scripts/game-assets.sh prepare-fixtures`, `scripts/consumer-fixture.sh`, `GCAD_MVP_CHECK=1 scripts/mvp.sh plate.gasset` | pass: all 69 `.gasset` fixtures check | `mvp.sh` without `GCAD_MVP_CHECK` opens the viewer window, so it was not opened here |
| FBX helper | `scripts/check-fbx.sh`: Assimp a4643cb with the v12 patch, ufbx 5955c5c, Release build, ctest, `gcad asset-fbx-helper` handshake, three conversions | pass: 2/2 ctest, handshake reports v12 diff `783bf12b...`, `plate`, `plate-consumer-skinned`, `plate-scene-refined` convert | `examples/mesh/rigged_creature.gcad` is refused (skin playback check, error 1.2e-6 on clip `idle`); packaging (`package-helper`) not run |
| FBX process tests | `native-tests` (`cargo test --release -- --ignored`), driven by `scripts/check-fbx.sh` | pass: 9/9 | |
| Godot | `scripts/check-godot.sh`: portable Godot 4.7.2 in `target/godot`, `--headless` | pass: `plate-consumer-profile`, `plate-consumer-skinned`, `creature`, `rigged_creature` import with the expected mesh, surface, triangle, skin and clip counts; a truncated GLB is refused | |
| renderer patches | `scripts/game-assets.sh reproduce-renderer` on egui_bench 0.9.2 from crates.io; `verify-renderer ... vulkan` on v0l/egui_bench e37138d | reproduce passes (5 patches apply, checksums match). verify passes 10/10 on Vulkan with the atlas fixture cut to 3 slots; the fixture gcad writes now has 5 slots and fails `gcad_explicit_real_atlas_matches_separate_chart_references` (gcad #313) | dx12 needs Windows; `verify-gpu` (gcad-view GPU tests) not run |
| Bevy (`bevy/`) | see the Bevy rows below | | |
| Unity | `sh -n` on `unity/*.sh` | parses | needs a Unity editor (6000.4.7f1 or 6000.5.4f1) and, for the render and player checks, Windows with D3D11. C# cannot be compile-checked without Unity |
| Windows helpers | `pwsh` not installed here | not run | `windows/*.ps1` need Windows with Visual Studio; CI runs them on `windows-latest` |

## Running Unity on a machine that has it

Build the FBX package first (here or there), then from this repo:

```sh
gcad export $GCAD_REPO/examples/game-assets/plate-consumer-skinned.gasset /tmp/skinned.glb
gcad asset-fbx-experimental /tmp/skinned.glb /tmp/skinned-fbx --helper "$GCAD_FBX_HELPER"
UNITY=/path/to/Unity UNITY_VERSION=6000.5.4f1 sh unity/check-import.sh /tmp/skinned-fbx /tmp/unity-import
```

Success is exit 0 with `/tmp/unity-import/import-report.json` containing `"passed": true`. The
consumer, root-motion, UV, LOD and player modes are switched on with the `GCAD_*` variables
listed in `unity/README.md`; each writes its own `*-report.json` and the script fails unless that
report says `"passed": true`.
