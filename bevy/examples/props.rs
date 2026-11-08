use bevy::prelude::*;
use bevy::world_serialization::WorldAssetRoot;
use gcad_bevy::prelude::*;

const PROPS: &[&str] = &[
    "parts/bracket.gcad",
    "parts/plate.gcad",
    "parts/ring.gcad",
    "parts/link.gcad",
    "mesh/bracket_mascot.gcad",
];

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(AssetPlugin {
            file_path: assets_dir(),
            ..default()
        }))
        .add_plugins(GcadPlugin::default())
        .add_systems(Startup, setup)
        .add_systems(Update, (spin, report))
        .run();
}

fn report(models: Query<(&GcadModel, &GlobalTransform), Added<GlobalTransform>>) {
    for (model, at) in &models {
        info!("loaded {} triangles, bounds {:?}..{:?}, placed {:?}", model.triangles, model.bounds_min, model.bounds_max, at.compute_transform());
    }
}

fn assets_dir() -> String {
    std::env::var("GCAD_REPO").map_or_else(|_| "../../gcad/examples".into(), |repo| format!("{repo}/examples"))
}

#[derive(Component)]
struct Spin;

fn setup(mut commands: Commands, assets: Res<AssetServer>) {
    for (k, path) in PROPS.iter().enumerate() {
        let settings = GcadSettings {
            scale: Some(0.02),
            centre: Centre::Yes,
            ..default()
        };
        let prop = assets.load_gcad(*path, settings);
        let x = (k as f32 - (PROPS.len() as f32 - 1.0) / 2.0) * 1.5;
        commands.spawn((WorldAssetRoot(prop), Transform::from_xyz(x, 0.0, 0.0), Spin));
    }
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 2.6, 6.0).looking_at(Vec3::new(0.0, 0.2, 0.0), Vec3::Y),
    ));
    commands.spawn((
        DirectionalLight {
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(3.0, 5.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn spin(time: Res<Time>, mut props: Query<&mut Transform, With<Spin>>) {
    for mut t in &mut props {
        t.rotate_y(time.delta_secs() * 0.6);
    }
}
