# Changelog

## Unreleased

- Pin gcad 5a5fde3: `gcad2glb` has a content-addressed asset cache (`Sources::key` over the
  source, includes, named files, options and gcad version; `--cache`, `key`, `bake`,
  `bake --check`, `cache prune`). See `docs/caching.md`.
- Godot addon: imports go through `gcad2glb --cache` into `.godot/gcad-cache`; an edited include
  re-imports the models that include it.
- Unity package: `.gcad`/`.gasm` in `Assets/` become `model.gcad.glb` through `gcad2glb --cache`
  in `Library/gcad-cache` (`GcadCache`, `GcadSourceSync`).
- `gcad-bevy`: the shared key; a shipped bake (`gcad-bevy-bake`, `assets/gcad-baked/`) read
  through the asset reader; disk cache on Android and iOS; cleanup by age and size; corrupt files
  rebuilt; `GcadStats` hit counters and bytes on disk.
- Pin gcad 75e25dd (was the pre-split cc734a0). Scripts moved to `scripts/` and read gcad from
  `GCAD_REPO`; `fetch-gcad.sh`, `doctor.sh`, `check.sh`, `bump-gcad.sh`, `sync-cargo-pins.sh`.
- `gcad-bevy`: Bevy 0.20 asset loader for `.gcad` and `.gasm` on gcad's `gcad2glb` library, with
  `GcadPlugin`, settings for fit, detail, LODs and overrides, a memory and disk build cache, hot
  reload, the `hero` and `props` examples, headless tests, wasm32 and Android builds.
- `gcad-bevy`: Bevy's `multi_threaded` is a default feature instead of always on, so a game
  that runs Bevy single-threaded (kids-free-games) keeps its task pools as they are.
- `gcad-bevy`: `load_gcad_clip` loads the scene with the clip's settings first and reuses the
  clip handle once it exists, so a single-threaded app no longer replaces the fitted scene with
  an unfitted one.
- Godot: `addons/gcad` import plugin for `.gcad`/`.gasm`, a portable headless Godot for checks,
  and the GLB import check now runs (Godot 4.7.2).
- Unity: runtime scripts, editor adapters and shaders are a package, `io.v0l.gcad`, with asmdefs
  and an importer (menu, settings, optional auto-adapt).
- FBX helper: builds and passes on Linux. Fixed a crash on every conversion: the helper compiled
  rapidjson without Assimp's `RAPIDJSON_NOMEMBERITERATORCLASS`, so the two disagreed on
  `FindMember`. `native-tests/` runs the process tests with `cargo test`.
- CI: script lint, gcad build at the pin, fixtures, Godot, FBX helper, Bevy (with wasm32 and
  Android), renderer patch reproduction, Windows helpers; weekly against gcad master.
- `docs/status.md` records what ran where.
