# gcad-engines

Engine adapters and native tooling for gcad's game-asset pipeline, split out of the
`game-assets` fork by Wixely (history and authorship kept).

- `unity/`: Unity import, consumer and render checks
- `godot/`: Godot import check
- `fbx-helper/`: Assimp/ufbx based FBX helper, probes and material patches
- `renderer-patches/`: egui_bench renderer patches and their tests
- `windows/`: Windows build and test helpers
- `docs/fbx-backend-proposal.md`, `game-assets.sh`, `consumer-fixture.sh`, `mvp.sh`

These scripts need a gcad binary built with `--features game-assets`. Point `GCAD` at it
(`GCAD=/path/to/gcad`), or build the revision pinned in `GCAD_REV` from github.com/v0l/gcad.
Nothing here runs without Unity, Godot, the FBX native libraries (Assimp, ufbx) or Windows,
depending on the part.

The Rust side (the `.gasset` format, `gcad asset-*` commands, viewer, exporters) stays in gcad.
