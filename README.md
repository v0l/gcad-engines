# gcad-engines

Game-engine integrations for [gcad](https://github.com/v0l/gcad): load gcad models in Bevy, Godot
and Unity, plus the native FBX helper and renderer patches behind gcad's experimental
game-asset pipeline. gcad itself (the `.gcad`/`.gasm`/`.gasset` formats, `gcad2glb`, the
`gcad asset-*` commands) stays in the gcad repo; this repo adapts its output to engines.

What has been run, where, and with what result: [docs/status.md](docs/status.md).

| integration | what it does | how to run it |
|---|---|---|
| [Bevy](docs/bevy.md) (`bevy/`, crate `gcad-bevy`) | Asset loader: `.gcad`/`.gasm` load as scenes with part materials, skins, clips, morph targets, LODs and a build cache; hot reload; wasm32 and Android | `cd bevy && cargo test --release` |
| [Godot](godot/addons/gcad/README.md) (`godot/addons/gcad`) | Editor import plugin: `.gcad`/`.gasm` import as scenes through `gcad2glb` | `sh godot/check-addon.sh` |
| Godot GLB check (`godot/check-import.sh`) | Structural import check of gcad GLBs in a headless Godot 4 | `sh scripts/check-godot.sh` |
| [Unity](unity/Packages/io.v0l.gcad/README.md) (`unity/Packages/io.v0l.gcad`) | Package: adapts gcad FBX packages into prefabs with gcad materials, sockets, colliders, LOD groups, root motion, UV scroll | needs a Unity editor; `unity/check-import.sh`, see [unity/README.md](unity/README.md) |
| [FBX helper](fbx-helper/README.md) (`fbx-helper/`) | Assimp/ufbx process that turns gcad GLBs into FBX packages, plus probes | `sh scripts/check-fbx.sh` |
| [Renderer patches](renderer-patches/README.md) | History of the egui_bench renderer changes and their tests | `sh scripts/game-assets.sh reproduce-renderer ...`, `verify-renderer ...` |
| `windows/` | Build and debug gcad's game-asset tests on Windows | `windows/build-tests.ps1` |

## Getting gcad

Everything is pinned to the gcad commit in `GCAD_REV`. The scripts read gcad's examples from
`GCAD_REPO` (default `../gcad`, then `target/gcad`) and run the binary in `GCAD`.

```sh
sh scripts/fetch-gcad.sh --build     # clone gcad at GCAD_REV into target/gcad and build it
# or: cargo install --git https://github.com/v0l/gcad --rev $(cat GCAD_REV) gcad --features game-assets
sh scripts/doctor.sh                 # what can run on this machine
sh scripts/doctor.sh godot fbx       # exit non-zero if those parts cannot run
sh scripts/check.sh                  # every check that runs on Linux
```

## Keeping it current

`sh scripts/bump-gcad.sh` moves `GCAD_REV` to the latest gcad master, repins the Cargo manifests
(`bevy/`, `native-tests/`) and runs `scripts/check.sh`; it leaves the change for review. Run it
after a gcad change that touches `gcad2glb`, `gcad-mesh`, the `.gasset` format or the
`asset-*` commands, and before a game updates its gcad pin. CI runs every Monday against gcad
master without bumping anything and fails if master breaks something here.

## License

GPL-3.0-or-later, like gcad. The Unity, Godot, FBX and renderer work came from Wixely's
game-assets fork of gcad and keeps its authorship in this repo's history.
