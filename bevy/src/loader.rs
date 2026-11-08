use crate::cache::{self, Cache};
use crate::settings::{Centre, GcadSettings};
use crate::{source, world};
use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, AssetPath, LoadContext};
use bevy::prelude::*;
use bevy::world_serialization::WorldAsset;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub const EXTENSIONS: &[&str] = &["gcad", "gasm"];
const REFERENCED: &[&str] = &[".gcad", ".gasm", ".png", ".svg", ".dxf", ".step", ".stp", ".stl", ".ttf"];

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
}

impl GcadLoader {
    pub fn new(cache_dir: Option<PathBuf>, root: Option<PathBuf>) -> GcadLoader {
        GcadLoader {
            cache: Arc::new(Cache::new(cache_dir)),
            root,
        }
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
    let name = line.trim().strip_prefix("include ")?.split_whitespace().next()?;
    (!name.contains('=')).then_some(name)
}

fn references(source: &str) -> Vec<String> {
    let mut out: Vec<String> = source
        .lines()
        .filter(|line| include_of(line).is_none() && !line.trim_start().starts_with('#'))
        .flat_map(|line| line.split_whitespace())
        .map(|token| token.rsplit('=').next().unwrap_or(token).trim_matches(|c| c == '"' || c == ','))
        .filter(|token| {
            let lower = token.to_ascii_lowercase();
            REFERENCED.iter().any(|ext| lower.ends_with(ext)) && !token.starts_with("std:")
        })
        .map(str::to_string)
        .collect();
    out.sort();
    out.dedup();
    out
}

async fn read_text(context: &mut LoadContext<'_>, path: AssetPath<'static>) -> Result<String, GcadError> {
    let bytes = context
        .read_asset_bytes(path.clone())
        .await
        .map_err(|e| GcadError(format!("{path}: {e}")))?;
    String::from_utf8(bytes).map_err(|e| GcadError(format!("{path}: {e}")))
}

fn sibling(context: &LoadContext<'_>, dir: &Path, name: &str) -> AssetPath<'static> {
    let source = context.path().source().clone_owned();
    AssetPath::from_path_buf(dir.join(name)).with_source(source).into_owned()
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
        let dir = asset.parent().map(Path::to_path_buf).unwrap_or_default();
        let mut included: Vec<(PathBuf, String)> = Vec::new();
        let mut pending: Vec<(PathBuf, String)> = vec![(dir.clone(), text.clone())];
        while let Some((at, body)) = pending.pop() {
            for line in body.lines() {
                if let Some(name) = include_of(line) {
                    let key = at.join(name);
                    if included.iter().any(|(k, _)| *k == key) {
                        continue;
                    }
                    if included.len() >= 256 {
                        return Err(GcadError("more than 256 included files".into()));
                    }
                    let inner = read_text(context, sibling(context, &at, name)).await?;
                    let inner_dir = key.parent().map(Path::to_path_buf).unwrap_or_default();
                    pending.push((inner_dir, inner.clone()));
                    included.push((key, inner));
                }
            }
        }
        let source = expand(&text, &dir, &mut |at, name| {
            let key = at.join(name);
            included
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, t)| t.clone())
                .ok_or_else(|| GcadError(format!("include {} not found", key.display())))
        }, 0)?;
        let mut extra = Vec::new();
        for name in references(&source) {
            if let Ok(bytes) = context.read_asset_bytes(sibling(context, &dir, &name)).await {
                extra.push(bytes);
            }
        }
        let is_assembly = asset
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("gasm"));
        let file = self
            .root
            .as_ref()
            .filter(|_| cfg!(feature = "brep") && (is_assembly || !extra.is_empty()))
            .map(|root| root.join(&asset));
        if is_assembly && file.is_none() {
            return Err(GcadError(format!(
                "{}: .gasm assemblies need the brep feature and assets on disk",
                asset.display()
            )));
        }
        let centre = [match settings.centre {
            Centre::No => 0u8,
            Centre::Yes => 1,
            Centre::Unrigged => 2,
        }];
        let asset_name = asset.to_string_lossy().into_owned();
        let mut parts: Vec<&[u8]> = vec![&centre, asset_name.as_bytes()];
        parts.extend(extra.iter().map(Vec::as_slice));
        let key = cache::fold(source::key(&source, settings), &parts);
        let build = {
            let source = source.clone();
            let settings = settings.clone();
            let file = file.clone();
            move || {
                let started = bevy::platform::time::Instant::now();
                let built = source::build(&source, file.as_deref(), &settings);
                if let Ok(b) = &built {
                    debug!(
                        "built gcad model ({} bytes, {} triangles) in {:.0} ms",
                        source.len(),
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
            true => cache.get_or_build(key, build),
            false => build().map(Arc::new),
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
