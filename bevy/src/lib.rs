#![recursion_limit = "512"]

mod built;
mod cache;
mod loader;
mod settings;
mod source;
mod world;

pub use built::{Built, BuiltBody, BuiltClip, BuiltJoint, BuiltLod, BuiltPart};
pub use loader::{EXTENSIONS, GcadError, GcadLoader, expand_includes};
pub use settings::{Centre, GcadSettings, Lod};
pub use source::GCAD_VERSION;
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
        self.load_builder()
            .with_settings(move |s: &mut GcadSettings| *s = settings.clone())
            .load(format!("{path}#Animation/{clip}"))
    }
}

pub fn build(source: &str, settings: &GcadSettings) -> Result<Built, GcadError> {
    source::build(source, None, settings).map_err(GcadError)
}

pub fn build_file(path: impl AsRef<std::path::Path>, settings: &GcadSettings) -> Result<Built, GcadError> {
    let path = path.as_ref();
    let text = std::fs::read_to_string(path).map_err(|e| GcadError(format!("{}: {e}", path.display())))?;
    let dir = path.parent().map(std::path::Path::to_path_buf).unwrap_or_default();
    let source = expand_includes(&text, &mut |name| {
        std::fs::read_to_string(dir.join(name)).map_err(|e| GcadError(format!("{name}: {e}")))
    })?;
    let assembly = path.extension().is_some_and(|e| e.eq_ignore_ascii_case("gasm"));
    let by_path = cfg!(feature = "brep") && source == text;
    let file = (by_path && (assembly || has_references(&source))).then_some(path);
    source::build(&source, file, settings).map_err(GcadError)
}

fn has_references(source: &str) -> bool {
    source.lines().any(|l| matches!(l.split_whitespace().next(), Some("part" | "stamp" | "include")))
}

#[derive(Clone, Debug)]
pub struct GcadPlugin {
    pub cache_dir: Option<PathBuf>,
    pub asset_root: Option<PathBuf>,
}

impl Default for GcadPlugin {
    fn default() -> Self {
        GcadPlugin {
            cache_dir: cache::default_dir(),
            asset_root: None,
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
        let loader = GcadLoader::new(self.cache_dir.clone(), root);
        app.insert_resource(GcadStats(loader.cache.clone()));
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
        self.0.builds()
    }

    pub fn cache_dir(&self) -> Option<&std::path::Path> {
        self.0.dir()
    }
}
