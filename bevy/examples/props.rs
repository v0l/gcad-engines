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
        .add_systems(Update, spin)
        .run();
}

fn assets_dir() -> String {
    std::env::var("GCAD_REPO").map_or_else(|_| "../../gcad/examples".into(), |repo| format!("{repo}/examples"))
}

#[derive(Component)]
struct Spin;

fn setup(mut commands: Commands, assets: Res<AssetServer>) {
    for (k, path) in PROPS.iter().enumerate() {
        let settings = GcadSettings {
            centre: Centre::Yes,
            ..GcadSettings::fit_height(0.8)
        };
        let prop = assets.load_gcad(*path, settings);
        let x = (k as f32 - (PROPS.len() as f32 - 1.0) / 2.0) * 1.1;
        commands.spawn((WorldAssetRoot(prop), Transform::from_xyz(x, 0.0, 0.0), Spin));
    }
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 1.6, 4.5).looking_at(Vec3::new(0.0, 0.4, 0.0), Vec3::Y),
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
