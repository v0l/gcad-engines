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

## What is in it

| folder | |
|---|---|
| `Runtime/` (`Gcad.Runtime`) | `GcadDeformation` (CPU skinning fallback that keeps gcad's blended deformation basis), `GcadMotionTrack`, `GcadRootMotionPlayer` (absolute-time root motion), `GcadSockets` (named attachment points), `GcadUvScroll`, `GcadUvScrollAnimation` |
| `Editor/` (`Gcad.Editor`) | `GcadImporter` (menu, settings, postprocessor), `GcadMaterialAdapter` (materials from the helper's material contract), `GcadConsumerAdapter` (the `consumer unity version=1` profile: sockets, colliders, LOD groups), `GcadRootMotionAdapter`, `GcadUvScrollAdapter`, `GcadFbxMaterialContract`, `GcadFbxSkinWeights`, `GcadNormalTextureImport` |
| `Shaders/` | `gcad/PBR Opaque Mask`, `gcad/PBR Blend` and their includes |

## Status

Nothing in this package has been compiled or run in the gcad-engines repo: there is no Unity
editor on the machine that put it together. The scripts are the ones the acceptance checks in
`unity/check-import.sh` ran on Unity 6000.4.7f1 and 6000.5.4f1 under Windows (see
`unity/README.md`); the package layout, the asmdefs and `GcadImporter.cs` are new and unverified.
To check them on a machine with Unity, see `docs/status.md`.
