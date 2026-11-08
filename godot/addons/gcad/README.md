# gcad for Godot (addons/gcad)

An editor import plugin for Godot 4: drop a `.gcad` or `.gasm` file into the project and it
imports as a scene (meshes, part materials, skeleton, skin, one animation per `clip`). It runs
gcad's `gcad2glb` command and loads the GLB it writes with Godot's `GLTFDocument`.

## Install

1. Copy `godot/addons/gcad` from this repo into your project's `addons/`.
2. Build `gcad2glb` from gcad at `GCAD_REV`:
   `cargo install --git https://github.com/v0l/gcad --rev $(cat GCAD_REV) gcad2glb`
   (or `cargo build --release -p gcad2glb` in a gcad checkout).
3. Project Settings > Plugins: enable gcad. Set `gcad/gcad2glb_path` in Project Settings if
   `gcad2glb` is not on `PATH`.

## Import options

| option | `gcad2glb` flag |
|---|---|
| `height` | `--height`: scale to this height, lowest point at 0 |
| `scale` | `--scale` (used when `height` is 0) |
| `centre` | `--centre` |
| `detail` | `--detail` (mesh-only files) |
| `triangles` | `--tris` |

Edit them in the Import dock and press Reimport. A build error (with the gcad line number)
shows in the Output panel.

Builds go through `gcad2glb --cache`, into `res://.godot/gcad-cache` (Project Settings >
`gcad/cache_dir`; empty turns the cache off). A re-import with the same source, includes, options
and gcad version reuses the GLB and builds nothing. The plugin records each import's key and,
when the file system changes, re-imports a model whose key moved, so editing an included file
re-imports the models that include it. See [docs/caching.md](../../../docs/caching.md).

`addons/gcad/gcad.gd` has `build(tool, path, options)`, which returns the scene without the
editor, for runtime use or tests.

## Checks

`sh godot/check-addon.sh` (headless, needs `gcad2glb`): builds `rigged_creature.gcad` at height
1 (one skin, clips `idle` and `walk`), `creature.gcad`, a solid part at height 2, refuses a file
with a bad line, then runs `godot --headless --import` on a project with the plugin on and checks
the editor imported `models/creature.gcad` with the `gcad.scene` importer, that a clean
re-import used `.godot/gcad-cache/<key>.glb` with no build, that an edit rebuilds, and that
editing an included file re-imports the model that includes it.
