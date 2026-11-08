use bevy::animation::graph::AnimationGraphHandle;
use bevy::prelude::*;
use bevy::world_serialization::WorldAssetRoot;
use gcad_bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(AssetPlugin {
            file_path: assets_dir(),
            ..default()
        }))
        .add_plugins(GcadPlugin::default())
        .add_systems(Startup, setup)
        .add_systems(Update, play_walk)
        .run();
}

fn assets_dir() -> String {
    std::env::var("GCAD_REPO").map_or_else(|_| "../../gcad/examples".into(), |repo| format!("{repo}/examples"))
}

#[derive(Resource)]
struct Walk(Handle<AnimationGraph>, AnimationNodeIndex);

fn setup(
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    let creature = assets.load_gcad("mesh/rigged_creature.gcad", GcadSettings::fit_height(1.0));
    let (graph, nodes) = AnimationGraph::from_clip(assets.load("mesh/rigged_creature.gcad#Animation/walk"));
    commands.insert_resource(Walk(graphs.add(graph), nodes));
    commands.spawn(WorldAssetRoot(creature));
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(1.6, 1.1, 2.2).looking_at(Vec3::new(0.0, 0.5, 0.0), Vec3::Y),
    ));
    commands.spawn((
        DirectionalLight {
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(2.0, 4.0, 3.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn play_walk(
    mut commands: Commands,
    walk: Res<Walk>,
    mut players: Query<(Entity, &mut AnimationPlayer), Without<AnimationGraphHandle>>,
) {
    for (entity, mut player) in &mut players {
        player.play(walk.1).repeat();
        commands.entity(entity).insert(AnimationGraphHandle(walk.0.clone()));
    }
}
