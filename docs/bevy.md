# gcad-bevy

`bevy/` is the crate `gcad-bevy`: a Bevy asset loader that turns `.gcad` and `.gasm` files into
scenes while the game runs. It builds models with gcad's `gcad2glb` library (gcad #310), so a
game ships the gcad text instead of GLBs and never runs the gcad CLI. It grew out of
kids-free-games `crates/app/src/gcad.rs`.

## Install

```toml
gcad-bevy = { git = "https://github.com/v0l/gcad-engines", rev = "<rev>" }
```

The repo is private, so cargo needs git credentials for it (`CARGO_NET_GIT_FETCH_WITH_CLI=true`
with a working `git clone`).

Features:

| feature | what it adds |
|---|---|
| (none) | mesh files: blockout, sculpt, rigs, skins, clips, correctives, LODs; flat solids (`rect`, `extrude`, ...) through gcad-mesh. Builds for wasm32 and Android |
| `brep` | files with kernel solids (`hole`, `fillet`, `plane plate.end`, ...) and `.gasm` assemblies. Pulls in monstertruck: copy the `[patch.crates-io]` table from `bevy/Cargo.toml` into the game's root `Cargo.toml` |
| `parallel` | rayon inside gcad-mesh |
| `hot-reload` | Bevy's file watcher: saving a `.gcad`, or a file it `include`s, rebuilds the model |
| `window` | only for the examples |

Supported Bevy: 0.20 (tested with 0.20.1, the version kids-free-games locks).

## Use it

```rust
use bevy::prelude::*;
use bevy::world_serialization::WorldAssetRoot;
use gcad_bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, GcadPlugin::default()))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, assets: Res<AssetServer>, mut graphs: ResMut<Assets<AnimationGraph>>) {
    let settings = GcadSettings::fit_height(1.0);
    let hero = assets.load_gcad("models/hero.gcad", settings.clone());
    let walk = assets.load_gcad_clip("models/hero.gcad", "walk", settings);
    let (graph, node) = AnimationGraph::from_clip(walk);
    commands.insert_resource(Walk(graphs.add(graph), node));
    commands.spawn(WorldAssetRoot(hero));
}

#[derive(Resource)]
struct Walk(Handle<AnimationGraph>, AnimationNodeIndex);
```

The scene root carries the `AnimationPlayer`; give it an `AnimationGraphHandle` and call
`play(node)` (see `bevy/examples/hero.rs`). `assets.load("x.gcad")` works too and uses the
default settings.

Load clips and labelled assets with the same settings as the scene (`load_gcad_clip`, or
`load_builder().with_settings(...)`). Bevy keys an asset on its path and settings: a plain
`assets.load("hero.gcad#Animation/walk")` loads the file again with default settings and replaces
the fitted scene with an unfitted one.

Run the examples against a gcad checkout (`GCAD_REPO`, default `../../gcad`):

```sh
cd bevy
cargo run --release --example hero --features window
cargo run --release --example props --features window,brep
```

`hero` (walk clip playing) and `props` (solid parts and a skinned mascot), captured in a private
headless sway with `scripts/headless-shot.sh`:

![hero](img/bevy-hero.png)
![props](img/bevy-props.png)

## What a file becomes

- One child entity per body (named after it), and under it one entity per part with `Mesh3d`,
  a `StandardMaterial` (part colour as base colour, `roughness = 1 - 0.8 * gloss`, or white with
  vertex colours when the body is painted) and a `GcadPart { body, part, lod }`.
- Rigged bodies: a joint entity per bone with `AnimationTargetId` and `AnimatedBy`, a
  `SkinnedMesh` on every part (joint indices and weights, inverse bind matrices from the bone
  heads), `NoFrustumCulling`, and one `AnimationClip` per `clip` as `file.gcad#Animation/<name>`.
- Correctives become morph targets: a `<body>/Morphs` entity holds `MorphWeights` at their rest
  values, the parts reference it with `MeshMorphWeights`, and each clip drives the weights.
- `lod` levels spawn as more parts with `GcadPart.lod = 1, 2, ...`. Set
  `lod_distances: vec![10.0, 30.0]` to give each level a `VisibilityRange`; without it every
  level is visible and the game picks.
- The root has `GcadModel { clips, bounds_min, bounds_max, triangles }`.
- Labelled assets: `<body>/<part>/Mesh`, `<body>/<part>/Material`, `<body>/LOD<k>/<part>/...`,
  `<body>/Skin`, `Animation/<clip>`.

## Settings (`GcadSettings`)

| field | meaning |
|---|---|
| `detail` | remesh cell scale, above 1 is coarser (mesh-only files) |
| `vars` (`set(name, value)`) | override `let` values |
| `height`, `scale` | fit: scale to this height with the lowest point at 0, or by this factor |
| `centre` | `No`, `Yes`, or `Unrigged` (centre props, leave rigged models on their origin, as kids-free-games does) |
| `triangles`, `max_error` | decimate every body |
| `lods` | replace the file's `lod` levels (`Some(vec![])` drops them) |
| `lod_distances` | camera distances between LOD levels, for `VisibilityRange` |
| `cache` | use the build cache (default true) |

`GcadSettings::fit_height(1.0)` is what kids-free-games used: one unit high, feet at 0, props
centred.

## Includes, caching and threads

`include file.gcad` lines are read through Bevy's asset reader, so they work from an Android APK
and over HTTP on the web, and they are loader dependencies, so hot reload follows them.

Each model is built once per process and kept in memory (the last 16), keyed on gcad's
`cache_key` (gcad version, text with includes expanded, settings) plus the asset path and the
bytes of any file the text names (`part x.gcad`, stamps). With a cache directory (the default on
desktop: `$XDG_CACHE_HOME/gcad/bevy`, `GcadPlugin::without_disk_cache()` turns it off; never on
wasm, Android or iOS) the built model is stored as bitcode and reused on the next run.
`GcadStats::builds()` counts real builds. Builds run on Bevy's `AsyncComputeTaskPool`, off the
main thread.

## One file talks to gcad

`bevy/src/source.rs` is the only file that calls gcad: it runs `gcad2glb::Assets` and converts
the result into the crate's own `Built` model (`bevy/src/built.rs`), which the scene builder and
the cache use. A gcad API change, or a switch to another gcad entry point, is a change to that
one file.

## Limits

- `.gasset` (the game-asset pipeline) is not loaded: that pipeline lives in gcad-view behind the
  `game-assets` feature, with egui and wgpu, which a game should not link. Export the `.gasset`
  to GLB with `gcad export` and load it with `bevy_gltf`.
- `.gasm` and kernel solids that read other files (`part`, stamps) need `brep` and assets on disk
  (desktop): gcad-brep reads those files itself, not through Bevy's asset reader.
- `.gsim` files are refused.

## Tests

```sh
cd bevy
GCAD_REPO=../target/gcad cargo test --release                    # mesh only
GCAD_REPO=../target/gcad cargo test --release --features brep    # solid parts and assemblies
GCAD_REPO=../target/gcad cargo test --release --features hot-reload
cargo check --release --lib --target wasm32-unknown-unknown
cargo ndk -t arm64-v8a check --release --lib --features bevy/android-game-activity
```

`tests/load.rs` runs Bevy with `MinimalPlugins` (no window, no GPU) and loads every
`examples/mesh/*.gcad` from gcad, checking mesh, triangle, joint and clip counts against the
library, the fitted height, the morph weight curve, LOD ranges, nested includes, error lines,
the disk cache across two apps, and with `brep` a solid part and `assemblies/linkage.gasm`.
