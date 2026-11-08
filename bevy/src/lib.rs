#![recursion_limit = "512"]

mod built;
mod cache;
mod loader;
mod settings;
mod source;
mod world;
pub mod bake;

pub use built::{Built, BuiltBody, BuiltClip, BuiltJoint, BuiltLod, BuiltPart};
pub use cache::{BAKED_DIR, DEFAULT_MAX_AGE, DEFAULT_MAX_BYTES, FORMAT, decode, default_dir, encode, format_tag};
pub use loader::{EXTENSIONS, GcadError, GcadLoader, expand_includes};
pub use settings::{Centre, GcadSettings, Lod};
pub use source::{GCAD_VERSION, Sources, key as cache_key, key_with as cache_key_with};
pub use world::{GcadModel, GcadPart, material_of, mesh_of};

use bevy::prelude::*;
use std::path::PathBuf;

pub mod prelude {
    pub use crate::{Centre, GcadAssets, GcadModel, GcadPart, GcadPlugin, GcadSettings, Lod};
}

pub trait GcadAssets {
    fn load_gcad<'a>(
        &self,
        path: impl Into<bevy::asset::AssetPath<'a>>,
        settings: GcadSettings,
    ) -> Handle<bevy::world_serialization::WorldAsset>;

    fn load_gcad_clip(&self, path: &str, clip: &str, settings: GcadSettings) -> Handle<AnimationClip>;
}

impl GcadAssets for AssetServer {
    fn load_gcad<'a>(
        &self,
        path: impl Into<bevy::asset::AssetPath<'a>>,
        settings: GcadSettings,
    ) -> Handle<bevy::world_serialization::WorldAsset> {
        self.load_builder()
            .with_settings(move |s: &mut GcadSettings| *s = settings.clone())
            .load(path)
    }

    fn load_gcad_clip(&self, path: &str, clip: &str, settings: GcadSettings) -> Handle<AnimationClip> {
        let label = format!("{path}#Animation/{clip}");
        let _scene = self.load_gcad(path.to_string(), settings.clone());
        if let Some(loaded) = self.get_handle::<AnimationClip>(&label) {
            return loaded;
        }
        self.load_builder()
            .with_settings(move |s: &mut GcadSettings| *s = settings.clone())
            .load(label)
    }
}

pub fn build(source: &str, settings: &GcadSettings) -> Result<Built, GcadError> {
    source::build(source, None, settings).map_err(GcadError)
}

pub fn build_file(path: impl AsRef<std::path::Path>, settings: &GcadSettings) -> Result<Built, GcadError> {
    let path = path.as_ref();
    let sources = Sources::from_path(path).map_err(|e| GcadError(format!("{}: {e}", path.display())))?;
    source::build_sources(&sources, Some(path), settings).map_err(GcadError)
}

#[derive(Clone, Debug)]
pub struct GcadPlugin {
    pub cache_dir: Option<PathBuf>,
    pub asset_root: Option<PathBuf>,
    pub baked_dir: Option<String>,
    pub max_cache_bytes: u64,
    pub max_cache_age: std::time::Duration,
}

impl Default for GcadPlugin {
    fn default() -> Self {
        GcadPlugin {
            cache_dir: cache::default_dir(),
            asset_root: None,
            baked_dir: Some(BAKED_DIR.to_string()),
            max_cache_bytes: DEFAULT_MAX_BYTES,
            max_cache_age: DEFAULT_MAX_AGE,
        }
    }
}

impl GcadPlugin {
    pub fn without_disk_cache() -> Self {
        GcadPlugin {
            cache_dir: None,
            ..Default::default()
        }
    }

    pub fn with_cache_dir(dir: impl Into<PathBuf>) -> Self {
        GcadPlugin {
            cache_dir: Some(dir.into()),
            ..Default::default()
        }
    }
}

#[cfg(not(any(target_arch = "wasm32", target_os = "android", target_os = "ios")))]
fn disk_root(app: &App) -> Option<PathBuf> {
    let plugins = app.get_added_plugins::<AssetPlugin>();
    let file_path = plugins.first().map_or("assets", |p| p.file_path.as_str());
    Some(bevy::asset::io::file::FileAssetReader::get_base_path().join(file_path))
}

#[cfg(any(target_arch = "wasm32", target_os = "android", target_os = "ios"))]
fn disk_root(_app: &App) -> Option<PathBuf> {
    None
}

impl Plugin for GcadPlugin {
    fn build(&self, app: &mut App) {
        let root = self.asset_root.clone().or_else(|| disk_root(app));
        let cache = std::sync::Arc::new(cache::Cache::new(self.cache_dir.clone()));
        let loader = GcadLoader::with_cache(cache.clone(), root, self.baked_dir.clone());
        app.insert_resource(GcadStats(cache));
        if self.cache_dir.is_some() {
            let (max_bytes, max_age) = (self.max_cache_bytes, self.max_cache_age);
            app.add_systems(Startup, move |stats: Res<GcadStats>| stats.prune_in_background(max_bytes, max_age));
        }
        app.register_type::<GcadModel>()
            .register_type::<GcadPart>()
            .register_type::<Name>()
            .register_type::<Transform>()
            .register_type::<GlobalTransform>()
            .register_type::<bevy::transform::components::TransformTreeChanged>()
            .register_type::<Visibility>()
            .register_type::<InheritedVisibility>()
            .register_type::<ViewVisibility>()
            .register_type::<ChildOf>()
            .register_type::<Children>()
            .register_type::<Mesh3d>()
            .register_type::<MeshMaterial3d<StandardMaterial>>()
            .register_type::<bevy::mesh::skinning::SkinnedMesh>()
            .register_type::<bevy::mesh::morph::MorphWeights>()
            .register_type::<bevy::mesh::morph::MeshMorphWeights>()
            .register_type::<bevy::camera::visibility::NoFrustumCulling>()
            .register_type::<bevy::camera::visibility::VisibilityRange>()
            .register_type::<AnimationPlayer>()
            .register_type::<bevy::animation::AnimationTargetId>()
            .register_type::<bevy::animation::AnimatedBy>()
            .register_asset_loader(loader);
    }
}

#[derive(Resource, Clone)]
pub struct GcadStats(std::sync::Arc<cache::Cache>);

impl GcadStats {
    pub fn builds(&self) -> usize {
        cache::Cache::count(&self.0.builds)
    }

    pub fn hits_baked(&self) -> usize {
        cache::Cache::count(&self.0.hits_baked)
    }

    pub fn hits_disk(&self) -> usize {
        cache::Cache::count(&self.0.hits_disk)
    }

    pub fn hits_memory(&self) -> usize {
        cache::Cache::count(&self.0.hits_memory)
    }

    pub fn bytes_on_disk(&self) -> u64 {
        self.0.bytes_on_disk()
    }

    pub fn cache_dir(&self) -> Option<&std::path::Path> {
        self.0.dir()
    }

    pub fn prune(&self, max_bytes: u64, max_age: std::time::Duration) -> gcad2glb::cache::Pruned {
        self.0.prune(max_bytes, max_age)
    }

    pub fn prune_in_background(&self, max_bytes: u64, max_age: std::time::Duration) {
        let stats = self.clone();
        bevy::tasks::IoTaskPool::get()
            .spawn(async move {
                let pruned = stats.prune(max_bytes, max_age);
                if pruned.removed > 0 {
                    debug!("gcad cache: removed {} entries, {} bytes", pruned.removed, pruned.freed);
                }
            })
            .detach();
    }
}
