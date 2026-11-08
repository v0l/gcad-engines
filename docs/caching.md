# Caching gcad builds in every engine

A build runs only when the source (with its includes), a file it names, the settings or the gcad
version changed. Every engine gets this from one key, computed by gcad's `gcad2glb` library, and
a release build can ship the built models so a player's first start builds nothing either.

## The key

`gcad2glb::Sources` reads a model the way gcad does: it follows `include` lines (relative to the
file that has them, up to 16 deep, 256 files) and records every file the text names (`part
x.gcad`, `.png` stamps, `.svg`, `.dxf`, `.step`, `.stl`, `.ttf`). `Sources::key(&options)` is a
64-bit FNV-1a hash of:

- the gcad version (`gcad2glb::VERSION`: package version plus the last code commit),
- the source with includes expanded,
- every option (`detail`, `y_up`, `--set` variables, triangles, max error, fit height or
  scale, centre, LODs, texture size),
- the path and bytes of each named file (a missing one counts as missing, so adding it later
  changes the key).

Any engine can ask for it without building: `gcad2glb key [OPTIONS] model.gcad` prints it as 16
hex digits. Engines that read through their own asset system (Bevy on Android and the web) use
`Sources::read(name, text, read)` with their reader and get the same key.

## The CLI

```sh
gcad2glb --cache DIR [OPTIONS] model.gcad out.glb   # hit: copy DIR/<key>.glb; miss: build, store, copy
gcad2glb --cache DIR [OPTIONS] model.gcad           # same, prints the cached GLB's path
gcad2glb key [OPTIONS] model.gcad                   # the key, no build
gcad2glb bake [OPTIONS] SRC_DIR OUT_DIR             # every .gcad/.gasm under SRC_DIR into OUT_DIR/<key>.glb + index.json
gcad2glb bake --check [OPTIONS] SRC_DIR OUT_DIR     # exit 1 if any source has no entry or a stale one
gcad2glb cache prune DIR [--max-bytes N] [--max-age DAYS] [--keep index.json]...
```

A cache directory holds `<key>.glb` (what `Assets::to_glb` writes) and a `<key>.json` sidecar
(gcad version, source path, options hash, the hash of each read file). Writes go to a temp file
and are renamed, so processes racing on one key leave one whole file. A file that is not a whole
GLB is deleted and rebuilt. Reading an entry touches its mtime. `prune` drops entries untouched
for `--max-age` (30 days) and the oldest past `--max-bytes` (64 MB), never one named in a
`--keep` index. `index.json` maps each baked asset path to its key and options hash, with the gcad
version and format. Files reached only through `include` are not baked on their own.

## Which engine uses what

| engine | cache | where it lives | shipped in a release | built on first run |
|---|---|---|---|---|
| Godot (`godot/addons/gcad`) | `gcad2glb --cache` GLBs | `res://.godot/gcad-cache` (`gcad/cache_dir` setting, empty turns it off) | the imported `.scn` (Godot's own import output) | nothing: the game ships imported scenes |
| Unity (`io.v0l.gcad`) | `gcad2glb --cache` GLBs, written next to the source as `model.gcad.glb` | `Library/gcad-cache` | the imported GLB asset | nothing |
| Bevy (`gcad-bevy`) | bitcode of the built scene model, keyed on `Sources::key` | desktop `$XDG_CACHE_HOME/gcad/bevy`, Android `<app>/cache/gcad/bevy`, iOS `Library/Caches/gcad/bevy`, web none | `assets/gcad-baked/<key>.bin` + `index.json` from `gcad-bevy-bake` | nothing when baked; otherwise once, then the disk cache |

Godot and Unity build in the editor, so their cache saves editor imports: re-importing (a clean
`.godot`, another branch, a settings change back and forth) reuses the GLB. The Godot plugin
also records each import's key in `.godot/gcad-cache/imported.json` and, when the editor's file
system changes, re-imports a model whose key moved (an edited include, which Godot does not track
itself). The Unity package needs a glTF importer for the GLB: glTFast
(`com.unity.cloud.gltfast`), Unity's own package, which imports `.glb` as a model asset with
skins and clips; the FBX helper path stays for gcad game-asset packages.

Bevy loads models while the game runs, so its cache is the one players hit. It keeps bitcode of
its own scene model (`Built`) rather than GLB because that skips the GLB parse and the scene
conversion; the key is the shared one, folded with its `centre` mode and format tag.

## Checks

- `cargo test -p gcad2glb --test cache` in gcad: hit builds nothing, include, part file,
  setting and version changes miss, threads and processes racing on one key, bake and
  `--check`, prune with `--keep`, corrupt files.
- `sh godot/check-addon.sh`: headless Godot imports twice (the clean re-import is a cache hit),
  an edit rebuilds, an edited include re-imports the model that includes it.
- `sh unity/check-cache.sh`: compiles `GcadCache.cs` with .NET 8 and runs it against
  `gcad2glb` (hit, deleted GLB restored from the cache, flags and include edits miss). The
  editor side (`GcadSourceSync.cs`, the postprocessor and glTFast import) needs a Unity editor.
- `cd bevy && cargo test --release --test cache`: see [bevy.md](bevy.md).
