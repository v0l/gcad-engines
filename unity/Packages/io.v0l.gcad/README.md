# gcad for Unity (io.v0l.gcad)

Imports the FBX packages `gcad asset-fbx-experimental` writes (a folder with `model.fbx`,
`textures/` and the `gcad-*.json` receipts) into the Built-In render pipeline with gcad's
materials, sockets, colliders, LOD groups, root motion and UV scroll.

## Install

Add it by git URL in Package Manager (Window > Package Manager > + > Add package from git URL),
or in `Packages/manifest.json`:

```json
"io.v0l.gcad": "https://github.com/v0l/gcad-engines.git?path=/unity/Packages/io.v0l.gcad#master"
```

The repo is private: Unity uses your git credentials for it.

Requirements: Unity 6000.4 or 6000.5, Built-In render pipeline, Linear colour space, and a
`gcad_fbx_helper` built from `fbx-helper/` (`sh scripts/game-assets.sh build-probes Release v12`).

## Use

1. Make a package: `gcad export asset.gasset asset.glb`, then
   `gcad asset-fbx-experimental asset.glb Assets/Models/asset --helper /abs/gcad_fbx_helper`.
2. Project Settings > gcad: set the absolute helper path (or `GCAD_FBX_HELPER`), and tick
   "Adapt gcad packages on import" to adapt every package as it is imported.
3. Or right-click `model.fbx` > gcad > Build Adapted Prefab. The prefab goes into the sibling
   folder `<package>-gcad`; the FBX and its textures are not changed.

## Plain `.gcad` and `.gasm` models

Drop a `.gcad` or `.gasm` into `Assets/` and the package writes `model.gcad.glb` beside it with
`gcad2glb --cache Library/gcad-cache` (on editor load, when a source is imported, and from
Assets > gcad > Rebuild gcad Models). Import the GLB with glTFast (`com.unity.cloud.gltfast`,
Unity's own glTF package: it imports `.glb` as a model asset with meshes, skins and clips and
needs no FBX helper). The GLB is rewritten only when its bytes change, so Unity re-imports it
only then, and a build runs only when the key misses (source, includes, flags, gcad version).
Put `gcad2glb` flags for one model in its importer's `userData` (`--height 1 --centre`); set
the tool path with the `GCAD2GLB` variable or the `io.v0l.gcad.gcad2glb` editor pref. See
[docs/caching.md](../../../docs/caching.md).

## What is in it

| folder | |
|---|---|
| `Runtime/` (`Gcad.Runtime`) | `GcadDeformation` (CPU skinning fallback that keeps gcad's blended deformation basis), `GcadMotionTrack`, `GcadRootMotionPlayer` (absolute-time root motion), `GcadSockets` (named attachment points), `GcadUvScroll`, `GcadUvScrollAnimation` |
| `Editor/` (`Gcad.Editor`) | `GcadImporter` (menu, settings, postprocessor), `GcadCache` and `GcadSourceSync` (`.gcad`/`.gasm` to GLB through the `gcad2glb` cache), `GcadMaterialAdapter` (materials from the helper's material contract), `GcadConsumerAdapter` (the `consumer unity version=1` profile: sockets, colliders, LOD groups), `GcadRootMotionAdapter`, `GcadUvScrollAdapter`, `GcadFbxMaterialContract`, `GcadFbxSkinWeights`, `GcadNormalTextureImport` |
| `Shaders/` | `gcad/PBR Opaque Mask`, `gcad/PBR Blend` and their includes |

## Status

Nothing in this package has been compiled or run in the gcad-engines repo: there is no Unity
editor on the machine that put it together. The scripts are the ones the acceptance checks in
`unity/check-import.sh` ran on Unity 6000.4.7f1 and 6000.5.4f1 under Windows (see
`unity/README.md`); the package layout, the asmdefs and `GcadImporter.cs` are new and unverified.
To check them on a machine with Unity, see `docs/status.md`.
