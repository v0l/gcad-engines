use crate::cache::{self, BAKED_DIR, EXTENSION};
use crate::settings::GcadSettings;
use crate::source::{self, AssetCache, Index, Sources};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Default)]
pub struct Baked {
    pub index: Index,
    pub built: Vec<String>,
    pub reused: Vec<String>,
    pub removed: usize,
    pub bytes: u64,
}

pub fn baked_dir(assets: &Path) -> PathBuf {
    assets.join(BAKED_DIR)
}

pub fn key_of(assets: &Path, asset: &str, settings: &GcadSettings) -> Result<u64, String> {
    let text = std::fs::read_to_string(assets.join(asset)).map_err(|e| format!("{asset}: {e}"))?;
    let sources = Sources::read(asset, &text, |wanted| std::fs::read(assets.join(wanted)).ok())
        .map_err(|e| format!("{asset}: {e}"))?;
    source::key(&sources, settings).map_err(|e| format!("{asset}: {e}"))
}

pub fn models(assets: &Path) -> Vec<String> {
    source::sources_in(assets, &baked_dir(assets))
}

fn empty_index() -> Index {
    Index {
        gcad: source::GCAD_VERSION.to_string(),
        format: cache::format_tag(),
        ..Default::default()
    }
}

pub fn bake(assets: &Path, settings_for: impl Fn(&str) -> Option<GcadSettings>) -> Result<Baked, String> {
    let out = baked_dir(assets);
    let store = AssetCache::with_extension(&out, EXTENSION);
    let mut baked = Baked {
        index: empty_index(),
        ..Default::default()
    };
    for asset in models(assets) {
        let Some(settings) = settings_for(&asset) else {
            continue;
        };
        let text = std::fs::read_to_string(assets.join(&asset)).map_err(|e| format!("{asset}: {e}"))?;
        let sources = Sources::read(&asset, &text, |wanted| std::fs::read(assets.join(wanted)).ok())
            .map_err(|e| format!("{asset}: {e}"))?;
        let key = source::key(&sources, &settings).map_err(|e| format!("{asset}: {e}"))?;
        let have = std::fs::read(store.path(key)).ok().and_then(|b| cache::decode(&b));
        match have {
            Some(_) => baked.reused.push(asset.clone()),
            None => {
                let built = source::build_sources(&sources, Some(&assets.join(&asset)), &settings)
                    .map_err(|e| format!("{asset}: {e}"))?;
                source::write_atomic(&store.path(key), &cache::encode(&built))
                    .map_err(|e| format!("{}: {e}", store.path(key).display()))?;
                baked.built.push(asset.clone());
            }
        }
        baked.index.entries.insert(
            asset,
            gcad2glb::cache::Entry {
                key,
                options: gcad2glb::cache::options_key(&source::options_of(&settings)),
            },
        );
    }
    let keep = baked.index.keys();
    for (key, _, _) in store.entries() {
        if !keep.contains(&key) {
            store.remove(key);
            baked.removed += 1;
        }
    }
    let index = out.join(source::INDEX);
    source::write_atomic(&index, baked.index.to_json().as_bytes()).map_err(|e| format!("{}: {e}", index.display()))?;
    baked.bytes = store.bytes();
    Ok(baked)
}

pub fn check(assets: &Path, settings_for: impl Fn(&str) -> Option<GcadSettings>) -> Vec<String> {
    let out = baked_dir(assets);
    let index = match std::fs::read_to_string(out.join(source::INDEX)) {
        Ok(text) => match Index::from_json(&text) {
            Ok(index) => index,
            Err(e) => return vec![e.to_string()],
        },
        Err(e) => return vec![format!("{}: {e}", out.join(source::INDEX).display())],
    };
    let mut problems = Vec::new();
    if index.gcad != source::GCAD_VERSION {
        problems.push(format!("baked with gcad {}, this is {}", index.gcad, source::GCAD_VERSION));
    }
    if index.format != cache::format_tag() {
        problems.push(format!("baked as {}, this reads {}", index.format, cache::format_tag()));
    }
    let store = AssetCache::with_extension(&out, EXTENSION);
    for asset in models(assets) {
        let Some(settings) = settings_for(&asset) else {
            continue;
        };
        let key = match key_of(assets, &asset, &settings) {
            Ok(key) => key,
            Err(e) => {
                problems.push(e);
                continue;
            }
        };
        match index.entries.get(&asset) {
            None => problems.push(format!("{asset}: not baked")),
            Some(e) if e.key != key => {
                problems.push(format!("{asset}: stale, baked {:016x}, source is {key:016x}", e.key))
            }
            Some(_) if std::fs::read(store.path(key)).ok().and_then(|b| cache::decode(&b)).is_none() => {
                problems.push(format!("{asset}: {} missing or unreadable", cache::file_name(key)))
            }
            Some(_) => {}
        }
    }
    problems
}
