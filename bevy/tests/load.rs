use bevy::asset::LoadState;
use bevy::mesh::morph::MorphWeights;
use bevy::mesh::skinning::SkinnedMesh;
use bevy::prelude::*;
use bevy::world_serialization::{WorldAsset, WorldAssetRoot, WorldSerializationPlugin};
use gcad_bevy::prelude::*;
use gcad_bevy::GcadStats;
use std::path::PathBuf;

fn examples() -> PathBuf {
    let repo = std::env::var_os("GCAD_REPO")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../gcad"));
    let dir = repo.join("examples");
    assert!(dir.is_dir(), "no gcad examples at {}: set GCAD_REPO", dir.display());
    dir.canonicalize().unwrap()
}

fn app(assets: PathBuf, plugin: GcadPlugin) -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin {
            file_path: assets.to_string_lossy().into_owned(),
            ..default()
        },
        TransformPlugin,
        bevy::mesh::MeshPlugin,
        bevy::animation::AnimationPlugin,
        WorldSerializationPlugin,
    ))
    .init_asset::<StandardMaterial>()
    .add_plugins(plugin);
    app
}

fn load(app: &mut App, path: &str, settings: GcadSettings) -> Handle<WorldAsset> {
    let handle = app
        .world()
        .resource::<AssetServer>()
        .load_gcad(path.to_string(), settings);
    wait(app, &handle, path);
    handle
}

fn wait(app: &mut App, handle: &Handle<WorldAsset>, path: &str) {
    let started = std::time::Instant::now();
    loop {
        app.update();
        let server = app.world().resource::<AssetServer>();
        match server.load_state(handle) {
            LoadState::Loaded if server.is_loaded_with_dependencies(handle) => return,
            LoadState::Failed(e) => panic!("{path}: {e}"),
            _ => {}
        }
        assert!(started.elapsed().as_secs() < 300, "{path} did not load");
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}

#[derive(Debug, Default)]
struct Spawned {
    meshes: usize,
    skinned: usize,
    joints: usize,
    morphs: usize,
    names: Vec<String>,
    model: GcadModel,
    root_scale: f32,
    root_translation: Vec3,
}

fn spawn(app: &mut App, handle: &Handle<WorldAsset>) -> Spawned {
    let root = app.world_mut().spawn(WorldAssetRoot(handle.clone())).id();
    for _ in 0..4 {
        app.update();
    }
    let mut out = Spawned::default();
    let mut stack = vec![root];
    while let Some(e) = stack.pop() {
        let entity = app.world().entity(e);
        if entity.contains::<Mesh3d>() {
            out.meshes += 1;
        }
        if let Some(skin) = entity.get::<SkinnedMesh>() {
            out.skinned += 1;
            out.joints = out.joints.max(skin.joints.len());
        }
        if entity.contains::<MorphWeights>() {
            out.morphs += 1;
        }
        if let Some(name) = entity.get::<Name>() {
            out.names.push(name.to_string());
        }
        if let Some(model) = entity.get::<GcadModel>() {
            out.model = model.clone();
            let t = entity.get::<Transform>().unwrap();
            out.root_scale = t.scale.x;
            out.root_translation = t.translation;
        }
        if let Some(children) = entity.get::<Children>() {
            stack.extend(children.iter());
        }
    }
    out
}

fn clip_by_name<'a>(app: &'a mut App, path: &str, name: &str) -> &'a AnimationClip {
    let handle: Handle<AnimationClip> = app
        .world()
        .resource::<AssetServer>()
        .load(format!("{path}#Animation/{name}"));
    let started = std::time::Instant::now();
    while started.elapsed().as_secs() < 120 {
        if app.world().resource::<Assets<AnimationClip>>().contains(&handle) {
            break;
        }
        app.update();
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let clips = app.world().resource::<Assets<AnimationClip>>();
    clips.get(&handle).unwrap_or_else(|| panic!("no {path}#Animation/{name}"))
}

fn clip_names(app: &mut App, path: &str, names: &[&str]) {
    for name in names {
        let clip = clip_by_name(app, path, name);
        assert!(clip.duration() > 0.0, "{path}#{name} has no duration");
        assert!(!clip.curves().is_empty(), "{path}#{name} has no curves");
    }
}

#[test]
fn rigged_creature_loads_skin_joints_and_clips() {
    let mut app = app(examples(), GcadPlugin::without_disk_cache());
    let handle = load(&mut app, "mesh/rigged_creature.gcad", GcadSettings::fit_height(1.0));
    let s = spawn(&mut app, &handle);
    assert_eq!(s.model.clips, ["idle", "walk"]);
    assert!(s.skinned >= 1, "{s:?}");
    assert!(s.joints >= 4, "{s:?}");
    assert!(s.names.iter().any(|n| n == "creature" || n == "main"), "{:?}", s.names);
    assert!(s.model.triangles > 1000, "{s:?}");
    let height = (s.model.bounds_max.y - s.model.bounds_min.y) * s.root_scale;
    assert!((height - 1.0).abs() < 1e-4, "fitted height {height}");
    assert!((s.model.bounds_min.y * s.root_scale + s.root_translation.y).abs() < 1e-4);
    clip_names(&mut app, "mesh/rigged_creature.gcad", &["idle", "walk"]);
}

#[test]
fn a_clip_loaded_with_the_same_settings_keeps_the_fit() {
    let mut app = app(examples(), GcadPlugin::without_disk_cache());
    let settings = GcadSettings::fit_height(1.0);
    let server = app.world().resource::<AssetServer>().clone();
    let clip = server.load_gcad_clip("mesh/rigged_creature.gcad", "walk", settings.clone());
    let handle = load(&mut app, "mesh/rigged_creature.gcad", settings);
    let s = spawn(&mut app, &handle);
    let height = (s.model.bounds_max.y - s.model.bounds_min.y) * s.root_scale;
    assert!((height - 1.0).abs() < 1e-4, "fitted height {height}");
    assert!(app.world().resource::<Assets<AnimationClip>>().contains(&clip));
}

#[test]
fn corrective_shapes_become_morph_targets_with_a_weight_curve() {
    let mut app = app(examples(), GcadPlugin::without_disk_cache());
    let handle = load(&mut app, "mesh/skin_arm_corrective.gcad", GcadSettings::default());
    let s = spawn(&mut app, &handle);
    assert_eq!(s.morphs, 1, "{s:?}");
    let meshes = app.world().resource::<Assets<Mesh>>();
    let server = app.world().resource::<AssetServer>();
    let morphed = meshes
        .iter()
        .filter(|(id, _)| server.get_path(*id).is_some_and(|p| p.to_string().contains("skin_arm_corrective")))
        .filter(|(_, m)| m.has_morph_targets())
        .count();
    assert!(morphed >= 1);
    let rom = clip_by_name(&mut app, "mesh/skin_arm_corrective.gcad", "rom").clone();
    let holders: Vec<&String> = s.names.iter().filter(|n| n.ends_with("/Morphs")).collect();
    assert_eq!(holders.len(), 1, "{:?}", s.names);
    let target = bevy::animation::AnimationTargetId::from_name(&Name::new(holders[0].clone()));
    assert!(rom.curves().contains_key(&target), "rom clip drives the corrective weights");
}

#[test]
fn every_mesh_example_loads() {
    let dir = examples();
    let mut app = app(dir.clone(), GcadPlugin::without_disk_cache());
    let mut seen = 0;
    for entry in std::fs::read_dir(dir.join("mesh")).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|e| e != "gcad") {
            continue;
        }
        let rel = format!("mesh/{}", path.file_name().unwrap().to_string_lossy());
        let expected = match gcad_bevy::build_file(&path, &GcadSettings::default()) {
            Err(e) if !cfg!(feature = "brep") && e.0.contains("without the brep feature") => continue,
            other => other.unwrap(),
        };
        let handle = load(&mut app, &rel, GcadSettings::default());
        let s = spawn(&mut app, &handle);
        let parts: usize = expected.bodies.iter().map(|b| b.parts.len() * (1 + b.lods.len())).sum();
        assert_eq!(s.meshes, parts, "{rel}");
        assert_eq!(s.model.triangles, expected.triangles(), "{rel}");
        assert_eq!(s.model.clips, expected.clip_names(), "{rel}");
        let joints = expected.bodies.iter().map(|b| b.joints.len()).max().unwrap_or(0);
        assert_eq!(s.joints, joints, "{rel}");
        seen += 1;
    }
    assert!(seen >= if cfg!(feature = "brep") { 8 } else { 7 }, "only {seen} mesh examples");
}

#[test]
fn includes_resolve_through_the_asset_reader() {
    let dir = tempdir("includes");
    std::fs::create_dir_all(dir.join("parts")).unwrap();
    std::fs::write(dir.join("model.gcad"), "include parts/body.gcad\nremesh 0.5\n").unwrap();
    std::fs::write(dir.join("parts/body.gcad"), "blob 0,0,0 r=2,2,2\ninclude cap.gcad\n").unwrap();
    std::fs::write(dir.join("parts/cap.gcad"), "blob 0,0,3 r=1,1,1 blend=0.5\n").unwrap();
    let mut app = app(dir.clone(), GcadPlugin::without_disk_cache());
    let handle = load(&mut app, "model.gcad", GcadSettings::default());
    let s = spawn(&mut app, &handle);
    assert_eq!(s.meshes, 2, "the blend seam splits the remesh into two parts");
    assert!(s.model.bounds_max.y > 3.5, "cap from the nested include is there: {:?}", s.model);
}

#[test]
fn a_broken_file_fails_with_its_line() {
    let dir = tempdir("broken");
    std::fs::write(dir.join("bad.gcad"), "blob 0,0,0 r=2,2,2\nfrobnicate 3\n").unwrap();
    let mut app = app(dir, GcadPlugin::without_disk_cache());
    let handle: Handle<WorldAsset> = app.world().resource::<AssetServer>().load("bad.gcad");
    let started = std::time::Instant::now();
    loop {
        app.update();
        if let LoadState::Failed(e) = app.world().resource::<AssetServer>().load_state(&handle) {
            let text = e.to_string();
            assert!(text.contains("line 2"), "{text}");
            assert!(text.contains("frobnicate"), "{text}");
            break;
        }
        assert!(started.elapsed().as_secs() < 60);
    }
}

#[test]
fn the_disk_cache_reuses_a_model_built_in_an_earlier_run() {
    let assets = tempdir("cache-assets");
    let cache = tempdir("cache-store");
    std::fs::copy(examples().join("mesh/creature.gcad"), assets.join("creature.gcad")).unwrap();
    let plugin = GcadPlugin {
        cache_dir: Some(cache.clone()),
        asset_root: None,
    };
    let mut first = app(assets.clone(), plugin.clone());
    let handle = load(&mut first, "creature.gcad", GcadSettings::default());
    let one = spawn(&mut first, &handle);
    assert_eq!(first.world().resource::<GcadStats>().builds(), 1);
    assert_eq!(std::fs::read_dir(&cache).unwrap().count(), 1);
    let mut second = app(assets.clone(), plugin.clone());
    let handle = load(&mut second, "creature.gcad", GcadSettings::default());
    let two = spawn(&mut second, &handle);
    assert_eq!(second.world().resource::<GcadStats>().builds(), 0, "second run read the cache");
    assert_eq!(one.model, two.model);
    let mut third = app(assets, plugin);
    load(&mut third, "creature.gcad", GcadSettings::default().set("unused", 2.0));
    assert_eq!(third.world().resource::<GcadStats>().builds(), 1, "other settings build again");
}

#[test]
fn lods_spawn_as_extra_levels_with_visibility_ranges() {
    let dir = tempdir("lods");
    std::fs::write(
        dir.join("rock.gcad"),
        "blob 0,0,0 r=4,3,3\nremesh 0.3\nlod 50% 10%\n",
    )
    .unwrap();
    let mut app = app(dir, GcadPlugin::without_disk_cache());
    let settings = GcadSettings {
        lod_distances: vec![10.0, 30.0],
        ..default()
    };
    let handle = load(&mut app, "rock.gcad", settings);
    let root = app.world_mut().spawn(WorldAssetRoot(handle)).id();
    for _ in 0..4 {
        app.update();
    }
    let mut levels = Vec::new();
    let mut q = app.world_mut().query::<(&GcadPart, &bevy::camera::visibility::VisibilityRange)>();
    for (part, range) in q.iter(app.world()) {
        levels.push((part.lod, range.start_margin.start, range.end_margin.end));
    }
    levels.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(levels.len(), 3, "{levels:?}");
    assert_eq!(levels[0], (0, 0.0, 10.0));
    assert_eq!(levels[1], (1, 10.0, 30.0));
    assert_eq!(levels[2].0, 2);
    let _ = root;
}

#[cfg(feature = "brep")]
#[test]
fn solid_parts_and_assemblies_load_with_brep() {
    let mut app = app(examples(), GcadPlugin::without_disk_cache());
    let bracket = load(&mut app, "parts/bracket.gcad", GcadSettings::fit_height(1.0));
    let b = spawn(&mut app, &bracket);
    assert_eq!(b.meshes, 1, "{b:?}");
    assert!(b.model.triangles > 50);
    let linkage = load(&mut app, "assemblies/linkage.gasm", GcadSettings::default());
    let l = spawn(&mut app, &linkage);
    assert!(l.meshes >= 4, "one mesh per placed part: {l:?}");
}

fn tempdir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "gcad-bevy-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[cfg(feature = "hot-reload")]
#[test]
fn editing_an_included_file_reloads_the_model() {
    let dir = tempdir("reload");
    std::fs::write(dir.join("model.gcad"), "include body.gcad\nremesh 0.5\n").unwrap();
    std::fs::write(dir.join("body.gcad"), "blob 0,0,0 r=2,2,2\n").unwrap();
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin {
            file_path: dir.to_string_lossy().into_owned(),
            watch_for_changes_override: Some(true),
            ..default()
        },
        TransformPlugin,
        bevy::mesh::MeshPlugin,
        bevy::animation::AnimationPlugin,
        WorldSerializationPlugin,
    ))
    .init_asset::<StandardMaterial>()
    .add_plugins(GcadPlugin::without_disk_cache());
    let handle = load(&mut app, "model.gcad", GcadSettings::default());
    let before = spawn(&mut app, &handle).model.bounds_max.y;
    std::thread::sleep(std::time::Duration::from_millis(300));
    std::fs::write(dir.join("body.gcad"), "blob 0,0,0 r=2,2,6\n").unwrap();
    let started = std::time::Instant::now();
    loop {
        app.update();
        let mut q = app.world_mut().query::<&GcadModel>();
        let now = q.iter(app.world()).map(|m| m.bounds_max.y).fold(0.0, f32::max);
        if now > before + 2.0 {
            break;
        }
        assert!(started.elapsed().as_secs() < 30, "no reload: {before} -> {now}");
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}
