use bevy::asset::LoadState;
use bevy::prelude::*;
use bevy::world_serialization::{WorldAsset, WorldSerializationPlugin};
use gcad_bevy::prelude::*;
use gcad_bevy::{GcadStats, bake};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

const BODY: &str = "blob 0,0,0 r=2,2,2\nremesh 0.6\n";

fn tempdir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "gcad-bevy-{name}-{}-{}",
        std::process::id(),
        SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn app(assets: &Path, plugin: GcadPlugin) -> App {
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
    .init_asset::<Image>()
    .add_plugins(plugin);
    app
}

fn load(app: &mut App, path: &str, settings: GcadSettings) -> Handle<WorldAsset> {
    let handle = app.world().resource::<AssetServer>().load_gcad(path.to_string(), settings);
    let started = std::time::Instant::now();
    loop {
        app.update();
        let server = app.world().resource::<AssetServer>();
        match server.load_state(&handle) {
            LoadState::Loaded if server.is_loaded_with_dependencies(&handle) => return handle,
            LoadState::Failed(e) => panic!("{path}: {e}"),
            _ => {}
        }
        assert!(started.elapsed().as_secs() < 120, "{path} did not load");
        std::thread::sleep(Duration::from_millis(2));
    }
}

fn model(app: &mut App, handle: &Handle<WorldAsset>) -> GcadModel {
    let root = app.world_mut().spawn(bevy::world_serialization::WorldAssetRoot(handle.clone())).id();
    for _ in 0..4 {
        app.update();
    }
    let mut q = app.world_mut().query::<(Entity, &GcadModel)>();
    let found = q.iter(app.world()).find(|(e, _)| app.world().entity(*e).get::<ChildOf>().is_some_and(|c| c.parent() == root));
    found.map(|(_, m)| m.clone()).unwrap_or_default()
}

struct Run {
    builds: usize,
    baked: usize,
    disk: usize,
    memory: usize,
    model: GcadModel,
}

fn run(assets: &Path, plugin: GcadPlugin, path: &str, settings: GcadSettings) -> Run {
    let mut app = app(assets, plugin);
    let handle = load(&mut app, path, settings);
    let model = model(&mut app, &handle);
    let stats = app.world().resource::<GcadStats>();
    Run {
        builds: stats.builds(),
        baked: stats.hits_baked(),
        disk: stats.hits_disk(),
        memory: stats.hits_memory(),
        model,
    }
}

fn game(dir: &Path) {
    write(&dir.join("models/hero.gcad"), "include parts/body.gcad\n");
    write(&dir.join("models/parts/body.gcad"), "include cap.gcad\n");
    write(&dir.join("models/parts/cap.gcad"), BODY);
    write(&dir.join("models/rock.gcad"), "blob 0,0,0 r=3,1,2\nremesh 0.6\n");
}

fn settings_for(asset: &str) -> Option<GcadSettings> {
    Some(match asset {
        "models/hero.gcad" => GcadSettings::fit_height(1.0),
        _ => GcadSettings::fit_height(0.5),
    })
}

fn settings(asset: &str) -> GcadSettings {
    settings_for(asset).unwrap()
}

#[test]
fn a_baked_model_loads_with_no_build() {
    let assets = tempdir("baked");
    game(&assets);
    let baked = bake::bake(&assets, settings_for).unwrap();
    assert_eq!(baked.built.len(), 2);
    assert_eq!(baked.index.entries.keys().collect::<Vec<_>>(), ["models/hero.gcad", "models/rock.gcad"]);
    for asset in baked.index.entries.keys() {
        assert_eq!(
            baked.index.entries[asset].key,
            bake::key_of(&assets, asset, &settings(asset)).unwrap(),
            "the baked key is the key the loader computes"
        );
    }
    let fresh = run(&assets, GcadPlugin::without_disk_cache(), "models/hero.gcad", settings("models/hero.gcad"));
    assert_eq!((fresh.builds, fresh.baked, fresh.disk, fresh.memory), (0, 1, 0, 0));
    let mut unbaked = GcadPlugin::without_disk_cache();
    unbaked.baked_dir = None;
    let built = run(&assets, unbaked, "models/hero.gcad", settings("models/hero.gcad"));
    assert_eq!(built.builds, 1);
    assert_eq!(fresh.model, built.model, "the baked model is the built one");
}

#[test]
fn an_edit_misses_the_bake_builds_once_then_hits_the_disk_cache() {
    for edit in ["models/hero.gcad", "models/parts/cap.gcad"] {
        let assets = tempdir("edit");
        let cache = tempdir("edit-cache");
        game(&assets);
        bake::bake(&assets, settings_for).unwrap();
        let text = std::fs::read_to_string(assets.join(edit)).unwrap();
        write(&assets.join(edit), &format!("{text}let unused=1\n"));
        let hero = settings("models/hero.gcad");
        let plugin = || GcadPlugin::with_cache_dir(cache.clone());
        let first = run(&assets, plugin(), "models/hero.gcad", hero.clone());
        assert_eq!((first.builds, first.baked, first.disk), (1, 0, 0), "{edit}");
        let second = run(&assets, plugin(), "models/hero.gcad", hero.clone());
        assert_eq!((second.builds, second.baked, second.disk), (0, 0, 1), "{edit}");
        let rock = run(&assets, plugin(), "models/rock.gcad", settings("models/rock.gcad"));
        assert_eq!((rock.builds, rock.baked), (0, 1), "the untouched model still comes from the bake");
        let problems = bake::check(&assets, settings_for);
        assert_eq!(problems.len(), 1, "{edit}: {problems:?}");
        assert!(problems[0].starts_with("models/hero.gcad: stale"), "{problems:?}");
    }
}

#[test]
fn a_part_or_stamp_file_named_by_the_source_is_in_the_key() {
    let assets = tempdir("part");
    write(
        &assets.join("m.gcad"),
        "blob 0,0,0 r=2\nremesh 0.6\nbrush grit type=stamp size=1 strength=0.1 alpha=parts/grit.png\n",
    );
    write(&assets.join("parts/grit.png"), "one");
    let s = GcadSettings::default();
    let before = bake::key_of(&assets, "m.gcad", &s).unwrap();
    write(&assets.join("parts/grit.png"), "two");
    assert_ne!(before, bake::key_of(&assets, "m.gcad", &s).unwrap());
}

#[test]
fn settings_and_the_gcad_version_are_in_the_key() {
    let assets = tempdir("key");
    game(&assets);
    let sources = gcad_bevy::Sources::from_path(assets.join("models/rock.gcad")).unwrap();
    let one = gcad_bevy::cache_key(&sources, &GcadSettings::fit_height(1.0)).unwrap();
    assert_ne!(one, gcad_bevy::cache_key(&sources, &GcadSettings::fit_height(2.0)).unwrap(), "fit height");
    let centred = GcadSettings {
        centre: Centre::Yes,
        ..GcadSettings::fit_height(1.0)
    };
    assert_ne!(one, gcad_bevy::cache_key(&sources, &centred).unwrap(), "centre");
    assert_ne!(
        one,
        gcad_bevy::cache_key_with("0.0.0-other", &sources, &GcadSettings::fit_height(1.0)).unwrap(),
        "gcad version"
    );
    bake::bake(&assets, settings_for).unwrap();
    let other = |_: &str| Some(GcadSettings::fit_height(3.0));
    assert_eq!(bake::check(&assets, other).len(), 2, "baked with other settings");
    let tall = run(&assets, GcadPlugin::without_disk_cache(), "models/rock.gcad", GcadSettings::fit_height(3.0));
    assert_eq!((tall.builds, tall.baked), (1, 0));
}

#[test]
fn an_index_from_another_gcad_version_is_ignored() {
    let assets = tempdir("version");
    game(&assets);
    bake::bake(&assets, settings_for).unwrap();
    let index = bake::baked_dir(&assets).join("index.json");
    let text = std::fs::read_to_string(&index).unwrap();
    write(&index, &text.replace(gcad_bevy::GCAD_VERSION, "0.0.0-old"));
    let r = run(&assets, GcadPlugin::without_disk_cache(), "models/rock.gcad", settings("models/rock.gcad"));
    assert_eq!((r.builds, r.baked), (1, 0));
    assert!(bake::check(&assets, settings_for)[0].contains("0.0.0-old"));
}

#[test]
fn a_bad_baked_or_cached_file_is_ignored_and_rebuilt() {
    let assets = tempdir("bad");
    let cache = tempdir("bad-cache");
    game(&assets);
    let baked = bake::bake(&assets, settings_for).unwrap();
    let key = baked.index.entries["models/rock.gcad"].key;
    let file = bake::baked_dir(&assets).join(format!("{key:016x}.bin"));
    let good = std::fs::read(&file).unwrap();
    let mut wrong_tag = good.clone();
    wrong_tag[0] ^= 0xff;
    let rock = settings("models/rock.gcad");
    for bad in [wrong_tag, good[..good.len() / 2].to_vec(), Vec::new()] {
        std::fs::write(&file, &bad).unwrap();
        let r = run(&assets, GcadPlugin::without_disk_cache(), "models/rock.gcad", rock.clone());
        assert_eq!((r.builds, r.baked), (1, 0));
        assert_eq!(bake::check(&assets, settings_for).len(), 1);

        let cached = cache.join(format!("{key:016x}.bin"));
        let plugin = || GcadPlugin::with_cache_dir(cache.clone());
        run(&assets, plugin(), "models/rock.gcad", rock.clone());
        std::fs::write(&cached, &bad).unwrap();
        let r = run(&assets, plugin(), "models/rock.gcad", rock.clone());
        assert_eq!((r.builds, r.disk), (1, 0), "a bad cache file is rebuilt");
        assert!(gcad_bevy::decode(&std::fs::read(&cached).unwrap()).is_some(), "and replaced");
    }
}

fn age(file: &Path, days: u64) {
    std::fs::File::options()
        .append(true)
        .open(file)
        .unwrap()
        .set_modified(SystemTime::now() - Duration::from_secs(days * 86400))
        .unwrap();
}

#[test]
fn cleanup_removes_old_orphans_and_keeps_shipped_and_used_entries() {
    let assets = tempdir("prune");
    let cache = tempdir("prune-cache");
    game(&assets);
    let baked = bake::bake(&assets, settings_for).unwrap();
    let shipped = baked.index.entries["models/rock.gcad"].key;
    std::fs::copy(
        bake::baked_dir(&assets).join(format!("{shipped:016x}.bin")),
        cache.join(format!("{shipped:016x}.bin")),
    )
    .unwrap();
    let orphan = cache.join("00000000000000aa.bin");
    std::fs::write(&orphan, b"old").unwrap();
    std::fs::write(cache.join("00000000000000bb.bin"), vec![0u8; 4000]).unwrap();
    age(&orphan, 31);
    age(&cache.join(format!("{shipped:016x}.bin")), 31);

    let mut plugin = GcadPlugin::with_cache_dir(cache.clone());
    plugin.baked_dir = None;
    plugin.max_cache_age = Duration::MAX;
    let mut hero_app = app(&assets, plugin);
    let edited = {
        write(&assets.join("models/hero.gcad"), "include parts/body.gcad\nlet x=2\n");
        load(&mut hero_app, "models/hero.gcad", settings("models/hero.gcad"));
        bake::key_of(&assets, "models/hero.gcad", &settings("models/hero.gcad")).unwrap()
    };
    let used = cache.join(format!("{edited:016x}.bin"));
    age(&used, 31);
    let stats = hero_app.world().resource::<GcadStats>().clone();
    let before = stats.bytes_on_disk();
    let pruned = stats.prune(gcad_bevy::DEFAULT_MAX_BYTES, gcad_bevy::DEFAULT_MAX_AGE);
    assert!(!orphan.exists(), "an orphan untouched for 31 days goes");
    assert!(used.exists(), "an entry used this run stays");
    assert!(cache.join("00000000000000bb.bin").exists(), "a fresh entry stays");
    assert_eq!(pruned.removed, 2, "the old copy of the shipped entry was not used this run");
    assert!(stats.bytes_on_disk() < before);

    let mut shipped_plugin = GcadPlugin::with_cache_dir(cache.clone());
    shipped_plugin.max_cache_age = Duration::MAX;
    let mut shipped_app = app(&assets, shipped_plugin);
    load(&mut shipped_app, "models/rock.gcad", settings("models/rock.gcad"));
    std::fs::copy(
        bake::baked_dir(&assets).join(format!("{shipped:016x}.bin")),
        cache.join(format!("{shipped:016x}.bin")),
    )
    .unwrap();
    age(&cache.join(format!("{shipped:016x}.bin")), 31);
    let stats = shipped_app.world().resource::<GcadStats>().clone();
    stats.prune(0, gcad_bevy::DEFAULT_MAX_AGE);
    assert!(cache.join(format!("{shipped:016x}.bin")).exists(), "entries named in the shipped index stay");
    assert!(!cache.join("00000000000000bb.bin").exists(), "past the size cap");
}

#[test]
fn the_startup_cleanup_runs_off_the_main_thread() {
    let assets = tempdir("startup");
    let cache = tempdir("startup-cache");
    game(&assets);
    let orphan = cache.join("00000000000000aa.bin");
    std::fs::write(&orphan, b"old").unwrap();
    age(&orphan, 31);
    let mut app = app(&assets, GcadPlugin::with_cache_dir(cache.clone()));
    let started = std::time::Instant::now();
    while orphan.exists() {
        app.update();
        assert!(started.elapsed().as_secs() < 10, "the orphan was not removed");
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn two_apps_writing_one_key_leave_one_valid_file() {
    let assets = tempdir("race");
    let cache = tempdir("race-cache");
    game(&assets);
    let workers: Vec<_> = (0..4)
        .map(|_| {
            let (assets, cache) = (assets.clone(), cache.clone());
            std::thread::spawn(move || {
                run(&assets, GcadPlugin::with_cache_dir(cache), "models/rock.gcad", settings("models/rock.gcad")).model
            })
        })
        .collect();
    let models: Vec<GcadModel> = workers.into_iter().map(|w| w.join().unwrap()).collect();
    assert!(models.windows(2).all(|w| w[0] == w[1]));
    let bins: Vec<PathBuf> = std::fs::read_dir(&cache)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "bin"))
        .collect();
    assert_eq!(bins.len(), 1, "{bins:?}");
    assert!(gcad_bevy::decode(&std::fs::read(&bins[0]).unwrap()).is_some());
    assert!(std::fs::read_dir(&cache).unwrap().all(|e| !e.unwrap().path().to_string_lossy().ends_with(".part")));
}

#[test]
fn a_second_model_with_the_same_source_is_a_memory_hit() {
    let assets = tempdir("memory");
    game(&assets);
    std::fs::copy(assets.join("models/rock.gcad"), assets.join("models/rock2.gcad")).unwrap();
    let mut app = app(&assets, GcadPlugin::without_disk_cache());
    let rock = settings("models/rock.gcad");
    load(&mut app, "models/rock.gcad", rock.clone());
    load(&mut app, "models/rock2.gcad", rock);
    let stats = app.world().resource::<GcadStats>();
    assert_eq!((stats.builds(), stats.hits_memory()), (1, 1));
}

#[test]
fn the_bake_cli_checks_and_bakes() {
    let assets = tempdir("cli");
    game(&assets);
    let cli = |args: &[&str]| {
        std::process::Command::new(env!("CARGO_BIN_EXE_gcad-bevy-bake"))
            .args(args)
            .arg(&assets)
            .output()
            .unwrap()
    };
    assert!(!cli(&["--check", "--height", "1"]).status.success(), "nothing baked");
    let baked = cli(&["--height", "1", "--centre", "unrigged"]);
    assert!(baked.status.success(), "{baked:?}");
    assert!(cli(&["--check", "--height", "1", "--centre", "unrigged"]).status.success());
    assert!(!cli(&["--check", "--height", "2", "--centre", "unrigged"]).status.success());
    let r = run(&assets, GcadPlugin::without_disk_cache(), "models/rock.gcad", GcadSettings::fit_height(1.0));
    assert_eq!((r.builds, r.baked), (0, 1));
}

fn kids_settings(asset: &str) -> Option<GcadSettings> {
    let scaled = ["road", "building1", "building2", "building3", "building4", "building5"]
        .iter()
        .any(|n| asset == format!("models/prop_{n}.gcad"));
    let scale = scaled.then_some(0.01f32).map(f64::from);
    Some(GcadSettings {
        height: scale.is_none().then_some(1.0),
        scale,
        ..GcadSettings::fit_height(1.0)
    })
}

fn gzipped(bytes: &[u8]) -> usize {
    let mut child = std::process::Command::new("gzip")
        .arg("-9c")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let owned = bytes.to_vec();
    let writer = std::thread::spawn(move || std::io::Write::write_all(&mut stdin, &owned));
    let out = child.wait_with_output().unwrap();
    writer.join().unwrap().unwrap();
    out.stdout.len()
}

#[test]
fn every_model_kids_free_games_loads_is_baked_when_present() {
    let Some(assets) = std::env::var_os("KIDS_GAMES_ASSETS").map(PathBuf::from) else {
        return;
    };
    let out = tempdir("kfg");
    let mut text = 0;
    for entry in std::fs::read_dir(assets.join("models")).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "gcad" || e == "gasm") {
            std::fs::create_dir_all(out.join("models")).unwrap();
            text += std::fs::copy(&path, out.join("models").join(path.file_name().unwrap())).unwrap();
        }
    }
    let started = std::time::Instant::now();
    let baked = bake::bake(&out, kids_settings).unwrap();
    let bake_s = started.elapsed().as_secs_f64();
    assert!(bake::check(&out, kids_settings).is_empty());
    let started = std::time::Instant::now();
    let again = bake::bake(&out, kids_settings).unwrap();
    let rebake_s = started.elapsed().as_secs_f64();
    assert!(again.built.is_empty());
    eprintln!("| model | bitcode bytes | gzip -9 bytes | build ms | from bake ms |");
    let (mut raw, mut packed) = (0, 0);
    for (asset, entry) in &baked.index.entries {
        let bytes = std::fs::read(bake::baked_dir(&out).join(format!("{:016x}.bin", entry.key))).unwrap();
        let settings = kids_settings(asset).unwrap();
        let mut unbaked = GcadPlugin::without_disk_cache();
        unbaked.baked_dir = None;
        let t = std::time::Instant::now();
        let built = run(&out, unbaked, asset, settings.clone());
        let build_ms = t.elapsed().as_secs_f64() * 1000.0;
        let t = std::time::Instant::now();
        let r = run(&out, GcadPlugin::without_disk_cache(), asset, settings);
        let baked_ms = t.elapsed().as_secs_f64() * 1000.0;
        assert_eq!((r.builds, r.baked), (0, 1), "{asset}");
        assert_eq!(r.model, built.model, "{asset}");
        let gz = gzipped(&bytes);
        eprintln!("| {asset} | {} | {gz} | {build_ms:.0} | {baked_ms:.0} |", bytes.len());
        raw += bytes.len();
        packed += gz;
    }
    eprintln!(
        "{} models from {text} bytes of text: bitcode {raw} bytes, gzip -9 {packed} bytes; bake {bake_s:.1} s, re-bake with nothing changed {rebake_s:.2} s",
        baked.index.entries.len()
    );
}
