use crate::cache::{self, Cache};
use crate::settings::GcadSettings;
use crate::{source, world};
use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, AssetPath, LoadContext};
use bevy::prelude::*;
use bevy::world_serialization::WorldAsset;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub const EXTENSIONS: &[&str] = &["gcad", "gasm"];

#[derive(Debug)]
pub struct GcadError(pub String);

impl std::fmt::Display for GcadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for GcadError {}

#[derive(TypePath, Clone)]
pub struct GcadLoader {
    pub(crate) cache: Arc<Cache>,
    pub(crate) root: Option<PathBuf>,
    pub(crate) baked: Option<String>,
    baked_keys: Arc<Mutex<Option<Arc<HashSet<u64>>>>>,
}

impl GcadLoader {
    pub fn new(cache_dir: Option<PathBuf>, root: Option<PathBuf>) -> GcadLoader {
        GcadLoader::with_cache(Arc::new(Cache::new(cache_dir)), root, Some(cache::BAKED_DIR.to_string()))
    }

    pub(crate) fn with_cache(cache: Arc<Cache>, root: Option<PathBuf>, baked: Option<String>) -> GcadLoader {
        GcadLoader {
            cache,
            root,
            baked,
            baked_keys: Arc::default(),
        }
    }

    async fn baked_keys(&self, dir: &str, context: &mut LoadContext<'_>) -> Arc<HashSet<u64>> {
        if let Some(keys) = self.baked_keys.lock().unwrap_or_else(|e| e.into_inner()).clone() {
            return keys;
        }
        let index = read_bytes(context, &format!("{dir}/{}", source::INDEX)).await;
        let keys = index
            .and_then(|b| String::from_utf8(b).ok())
            .and_then(|t| source::Index::from_json(&t).ok())
            .filter(|i| i.gcad == source::GCAD_VERSION && i.format == cache::format_tag())
            .map(|i| i.keys())
            .unwrap_or_default();
        let keys = Arc::new(keys);
        *self.baked_keys.lock().unwrap_or_else(|e| e.into_inner()) = Some(keys.clone());
        self.cache.keep(&keys);
        keys
    }
}

pub fn expand_includes(
    source: &str,
    read: &mut dyn FnMut(&str) -> Result<String, GcadError>,
) -> Result<String, GcadError> {
    expand(source, Path::new(""), &mut |dir, name| read(&dir.join(name).to_string_lossy()), 0)
}

fn expand(
    source: &str,
    dir: &Path,
    read: &mut dyn FnMut(&Path, &str) -> Result<String, GcadError>,
    depth: usize,
) -> Result<String, GcadError> {
    if depth > 16 {
        return Err(GcadError("includes are nested more than 16 deep".into()));
    }
    let mut out = String::with_capacity(source.len());
    for line in source.lines() {
        match include_of(line) {
            Some(name) => {
                let inner = read(dir, name)?;
                let inner_dir = dir.join(name).parent().map(Path::to_path_buf).unwrap_or_default();
                out.push_str(&expand(&inner, &inner_dir, read, depth + 1)?);
            }
            None => {
                out.push_str(line);
                out.push('\n');
            }
        }
    }
    Ok(out)
}

fn include_of(line: &str) -> Option<&str> {
    source::include_of(line)
}

async fn read_bytes(context: &mut LoadContext<'_>, path: &str) -> Option<Vec<u8>> {
    let source = context.path().source().clone_owned();
    let path = AssetPath::from(path.to_string()).with_source(source).into_owned();
    context.read_asset_bytes(path).await.ok()
}

impl AssetLoader for GcadLoader {
    type Asset = WorldAsset;
    type Settings = GcadSettings;
    type Error = GcadError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        settings: &GcadSettings,
        context: &mut LoadContext<'_>,
    ) -> Result<WorldAsset, GcadError> {
        let mut bytes = Vec::new();
        reader
            .read_to_end(&mut bytes)
            .await
            .map_err(|e| GcadError(e.to_string()))?;
        let text = String::from_utf8(bytes).map_err(|e| GcadError(e.to_string()))?;
        let asset = context.path().path().to_path_buf();
        let asset_name = asset.to_string_lossy().replace('\\', "/");
        let mut sources = source::Sources::new(&asset_name, &text);
        loop {
            let wanted = sources.wanted();
            if wanted.is_empty() {
                break;
            }
            for path in wanted {
                let bytes = read_bytes(context, &path).await;
                sources.supply(&path, bytes).map_err(|e| GcadError(e.to_string()))?;
            }
        }
        let is_assembly = asset
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("gasm"));
        let file = self
            .root
            .as_ref()
            .filter(|_| cfg!(feature = "brep") && (is_assembly || sources.has_references()))
            .map(|root| root.join(&asset));
        if is_assembly && file.is_none() {
            return Err(GcadError(format!(
                "{}: .gasm assemblies need the brep feature and assets on disk",
                asset.display()
            )));
        }
        let key = source::key(&sources, settings).map_err(GcadError)?;
        let meta = source::meta(&sources, &asset_name, settings);
        let baked = match (&self.baked, settings.cache && !self.cache.in_memory(key)) {
            (Some(dir), true) if self.baked_keys(dir, context).await.contains(&key) => {
                read_bytes(context, &format!("{dir}/{}", cache::file_name(key))).await
            }
            _ => None,
        };
        let build = {
            let settings = settings.clone();
            let file = file.clone();
            move || {
                let started = bevy::platform::time::Instant::now();
                let built = source::build_sources(&sources, file.as_deref(), &settings);
                if let Ok(b) = &built {
                    debug!(
                        "built gcad model ({} triangles) in {:.0} ms",
                        b.triangles(),
                        started.elapsed().as_secs_f64() * 1000.0
                    );
                }
                built
            }
        };
        let cache = self.cache.clone();
        let use_cache = settings.cache;
        let task = move || match use_cache {
            true => cache.get_or_build(key, baked, meta, build),
            false => cache.build_uncached(build),
        };
        let built = run_off_thread(task).await.map_err(|e| GcadError(format!("{}: {e}", asset.display())))?;
        Ok(world::into_world(&built, settings, context))
    }

    fn extensions(&self) -> &[&str] {
        EXTENSIONS
    }
}

async fn run_off_thread<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> T {
    bevy::tasks::AsyncComputeTaskPool::get()
        .spawn(async move { work() })
        .await
}
