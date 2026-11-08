use bevy::animation::graph::AnimationGraphHandle;
use bevy::prelude::*;
use bevy::world_serialization::WorldAssetRoot;
use gcad_bevy::prelude::*;

const HERO: &str = "mesh/rigged_creature.gcad";

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(AssetPlugin {
            file_path: assets_dir(),
            ..default()
        }))
        .add_plugins(GcadPlugin::default())
        .add_systems(Startup, setup)
        .add_systems(Update, (play_walk, report))
        .run();
}

fn assets_dir() -> String {
    std::env::var("GCAD_REPO").map_or_else(|_| "../../gcad/examples".into(), |repo| format!("{repo}/examples"))
}

#[derive(Resource)]
struct Walk(Handle<AnimationGraph>, AnimationNodeIndex);

fn setup(mut commands: Commands, assets: Res<AssetServer>, mut graphs: ResMut<Assets<AnimationGraph>>) {
    let settings = GcadSettings::fit_height(1.0);
    let creature = assets.load_gcad(HERO, settings.clone());
    let walk = assets.load_gcad_clip(HERO, "walk", settings);
    let (graph, node) = AnimationGraph::from_clip(walk);
    commands.insert_resource(Walk(graphs.add(graph), node));
    commands.spawn(WorldAssetRoot(creature));
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(1.6, 1.0, 2.0).looking_at(Vec3::new(0.0, 0.4, 0.0), Vec3::Y),
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

fn report(models: Query<(&GcadModel, &GlobalTransform), Added<GlobalTransform>>) {
    for (model, at) in &models {
        info!(
            "loaded {} triangles, clips {:?}, height {:.3}",
            model.triangles,
            model.clips,
            (model.bounds_max.y - model.bounds_min.y) * at.compute_transform().scale.y
        );
    }
}
