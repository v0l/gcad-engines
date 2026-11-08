# Status

What runs where, against gcad `GCAD_REV` (5a5fde3). Recorded 2026-11-08 on Linux, and in GitHub Actions (`.github/workflows/ci.yml`, every job green except as noted), x86_64
(Debian 13, NVIDIA RTX PRO 6000, cmake 3.31.6, g++ 14.2, Rust 1.99, Android NDK 28.2).
`sh scripts/doctor.sh` prints what the current machine can run; `sh scripts/check.sh` runs all
of the Linux rows below.

| integration | run here | result | not run here, and why |
|---|---|---|---|
| gcad build | `scripts/fetch-gcad.sh --build` (`cargo build --release --features game-assets -p gcad`), `cargo build --release -p gcad2glb` | builds | |
| fixtures | `scripts/game-assets.sh prepare-fixtures`, `scripts/consumer-fixture.sh`, `GCAD_MVP_CHECK=1 scripts/mvp.sh plate.gasset` | pass: every `.gasset` in gcad's examples checks | `mvp.sh` without `GCAD_MVP_CHECK` opens the viewer window, so it was not opened |
| Bevy (`bevy/`) | `cargo test --release` (`tests/load.rs` 11 tests, `tests/cache.rs` 12, with the kids-free-games hero when `KIDS_GAMES_MODELS` is set and its bake when `KIDS_GAMES_ASSETS` is), `--features brep`, `--features hot-reload`, `--no-default-features`; `cargo check --lib --target wasm32-unknown-unknown`; `cargo ndk -t arm64-v8a check --lib --features bevy/android-game-activity`; both examples in a private headless sway (`scripts/headless-shot.sh`, screenshots in `docs/img/`). The bake and disk caches measured in kids-free-games on desktop, the `kidsgames-test` emulator, the iOS simulator and headless Chrome (`docs/bevy.md`) | pass | not run on a real phone. Bevy 0.20.1 only |
| Godot GLB check | `scripts/check-godot.sh`: portable Godot 4.7.2 `--headless` on 4 GLBs and a truncated one | pass, truncated GLB refused | other Godot versions |
| Godot addon | `godot/check-addon.sh`: scenes through `gcad.gd` (fit height, skin, clips, a solid part, a broken file), `godot --headless --import` with the plugin on, a clean re-import from `.godot/gcad-cache` with no build, an edit and an edited include rebuilding | pass | the editor UI (import dock) was not opened |
| FBX helper | `scripts/check-fbx.sh`: Assimp a4643cb + v12 patch, ufbx 5955c5c, Release build, 2 ctest, `gcad asset-fbx-helper` handshake, 3 conversions, 9 Rust process tests (`native-tests`) | pass | `package-helper` bundle not built; `examples/mesh/rigged_creature.gcad` is refused by the helper's skin playback check (1.2e-6 on clip `idle`), as the helper is meant to |
| Renderer patches | `reproduce-renderer` on egui_bench 0.9.2 from crates.io; `verify-renderer ... vulkan` on v0l/egui_bench e37138d with gcad's atlas fixture | pass, 10/10 renderer tests on Vulkan | dx12 needs Windows; CI runners have no GPU, so CI only reproduces the patches |
| Unity package | `sh -n` and shellcheck on `unity/*.sh`; `unity/check-cache.sh` compiles `GcadCache.cs` with .NET 8 and runs it against `gcad2glb` | parse; cache check pass | needs a Unity editor (6000.4.7f1 or 6000.5.4f1) and for render and player checks Windows with D3D11. The Unity-facing C# (`GcadSourceSync.cs`, `GcadImporter.cs`), the asmdefs and the glTFast import of the written GLB are unverified |
| Windows helpers | in CI on `windows-latest`: both `.ps1` parse, `build-tests.ps1` builds gcad's game-asset test binaries at GCAD_REV and copies `lib.exe` | pass | not Windows here, no `pwsh` |

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
report says `"passed": true`. To check the package itself, add it to a project by path
(`"io.v0l.gcad": "file:/path/to/gcad-engines/unity/Packages/io.v0l.gcad"`), let the editor
compile (no errors in the Console), and use Assets > gcad > Build Adapted Prefab on the
`model.fbx` above.
