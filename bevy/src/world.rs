use crate::built::{Built, BuiltBody, BuiltClip, BuiltNormalMap, BuiltPart};
use crate::settings::GcadSettings;
use bevy::animation::animation_curves::WeightsCurve;
use bevy::animation::gltf_curves::WideLinearKeyframeCurve;
use bevy::animation::{AnimatedBy, AnimationTargetId, animated_field};
use bevy::asset::{LoadContext, RenderAssetUsages};
use bevy::camera::visibility::{NoFrustumCulling, VisibilityRange};
use bevy::curve::{ConstantCurve, Interval, UnevenSampleAutoCurve};
use bevy::mesh::morph::{MeshMorphWeights, MorphAttributes, MorphWeights};
use bevy::mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes};
use bevy::image::{CompressedImageFormats, ImageSampler, ImageType};
use bevy::mesh::{Indices, PrimitiveTopology, VertexAttributeValues};
use bevy::prelude::*;
use bevy::world_serialization::WorldAsset;

#[derive(Component, Reflect, Clone, Debug, Default, PartialEq)]
#[reflect(Component, Default, Clone)]
pub struct GcadModel {
    pub clips: Vec<String>,
    pub bounds_min: Vec3,
    pub bounds_max: Vec3,
    pub triangles: usize,
}

#[derive(Component, Reflect, Clone, Debug, Default, PartialEq)]
#[reflect(Component, Default, Clone)]
pub struct GcadPart {
    pub body: String,
    pub part: String,
    pub lod: usize,
}

pub fn mesh_of(body: &BuiltBody, part: &BuiltPart) -> Mesh {
    let first = part.first_index as usize;
    let range = first..first + part.index_count as usize;
    let mut used: Vec<u32> = body.indices[range.clone()].to_vec();
    used.sort_unstable();
    used.dedup();
    let mut remap = vec![u32::MAX; body.positions.len()];
    for (new, old) in used.iter().enumerate() {
        remap[*old as usize] = new as u32;
    }
    let pick = |v: &[[f32; 3]]| -> Vec<[f32; 3]> { used.iter().map(|i| v[*i as usize]).collect() };
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, pick(&body.positions));
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, pick(&body.normals));
    if let Some(map) = &body.normal_map {
        let uvs: Vec<[f32; 2]> = used.iter().map(|i| map.uvs[*i as usize]).collect();
        let tangents: Vec<[f32; 4]> = used.iter().map(|i| map.tangents[*i as usize]).collect();
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        mesh.insert_attribute(Mesh::ATTRIBUTE_TANGENT, tangents);
    }
    if !body.colours.is_empty() {
        let colours: Vec<[f32; 4]> = used
            .iter()
            .map(|i| {
                let c = Color::srgb_from_array(body.colours[*i as usize]).to_linear();
                [c.red, c.green, c.blue, 1.0]
            })
            .collect();
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colours);
    }
    if !body.joints.is_empty() {
        let joints: Vec<[u16; 4]> = used.iter().map(|i| body.joint_indices[*i as usize]).collect();
        let weights: Vec<[f32; 4]> = used.iter().map(|i| body.joint_weights[*i as usize]).collect();
        mesh.insert_attribute(Mesh::ATTRIBUTE_JOINT_INDEX, VertexAttributeValues::Uint16x4(joints));
        mesh.insert_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT, weights);
    }
    if !body.targets.is_empty() {
        let attributes = body
            .targets
            .iter()
            .flat_map(|t| {
                used.iter()
                    .map(|i| MorphAttributes::from([Vec3::from(t.deltas[*i as usize]), Vec3::ZERO, Vec3::ZERO]))
            })
            .collect();
        mesh.set_morph_targets(attributes);
        mesh.set_morph_target_names(body.targets.iter().map(|t| t.name.clone()).collect());
    }
    mesh.insert_indices(Indices::U32(
        body.indices[range].iter().map(|i| remap[*i as usize]).collect(),
    ));
    mesh
}

pub fn normal_image(map: &BuiltNormalMap) -> Option<Image> {
    Image::from_buffer(
        &map.png,
        ImageType::Extension("png"),
        CompressedImageFormats::NONE,
        false,
        ImageSampler::Default,
        RenderAssetUsages::default(),
    )
    .ok()
}

pub fn material_of(body: &BuiltBody, part: &BuiltPart) -> StandardMaterial {
    let tint = if body.colours.is_empty() {
        Color::srgb_from_array(part.colour)
    } else {
        Color::WHITE
    };
    StandardMaterial {
        base_color: tint,
        perceptual_roughness: 1.0 - part.gloss * 0.8,
        metallic: 0.0,
        ..default()
    }
}

pub fn clip_of(clip: &BuiltClip, paths: &[Vec<Name>], morph: Option<&Name>) -> AnimationClip {
    let mut out = AnimationClip::default();
    let target = |joint: u32| AnimationTargetId::from_names(paths[joint as usize].iter());
    for track in &clip.rotations {
        let values: Vec<Quat> = track.values.iter().map(|q| Quat::from_array(*q)).collect();
        if values.len() == 1 {
            out.add_curve_to_target(
                target(track.joint),
                AnimatableCurve::new(
                    animated_field!(Transform::rotation),
                    ConstantCurve::new(Interval::EVERYWHERE, values[0]),
                ),
            );
        } else if let Ok(curve) = UnevenSampleAutoCurve::new(track.times.iter().copied().zip(values)) {
            out.add_curve_to_target(
                target(track.joint),
                AnimatableCurve::new(animated_field!(Transform::rotation), curve),
            );
        }
    }
    for track in &clip.translations {
        let values: Vec<Vec3> = track.values.iter().map(|v| Vec3::from(*v)).collect();
        if values.len() == 1 {
            out.add_curve_to_target(
                target(track.joint),
                AnimatableCurve::new(
                    animated_field!(Transform::translation),
                    ConstantCurve::new(Interval::EVERYWHERE, values[0]),
                ),
            );
        } else if let Ok(curve) = UnevenSampleAutoCurve::new(track.times.iter().copied().zip(values)) {
            out.add_curve_to_target(
                target(track.joint),
                AnimatableCurve::new(animated_field!(Transform::translation), curve),
            );
        }
    }
    if let (Some(weights), Some(name)) = (&clip.weights, morph) {
        let flat: Vec<f32> = weights.weights.iter().flatten().copied().collect();
        if let Ok(curve) = WideLinearKeyframeCurve::new(weights.times.iter().copied(), flat) {
            out.add_curve_to_target(AnimationTargetId::from_name(name), WeightsCurve(curve));
        }
    }
    out.set_duration(clip.duration.max(1.0 / clip.fps.max(1.0)));
    out
}

fn local_offset(body: &BuiltBody, joint: usize) -> Vec3 {
    let j = &body.joints[joint];
    match j.parent {
        Some(p) => Vec3::from(j.head) - Vec3::from(body.joints[p as usize].head),
        None => Vec3::from(j.head),
    }
}

fn lod_range(distances: &[f32], level: usize, levels: usize) -> Option<VisibilityRange> {
    if levels < 2 || distances.len() + 1 < levels {
        return None;
    }
    let near = if level == 0 { 0.0 } else { distances[level - 1] };
    let far = if level + 1 == levels { f32::MAX } else { distances[level] };
    Some(VisibilityRange {
        start_margin: near..near,
        end_margin: far..far,
        use_aabb: false,
    })
}

pub fn into_world(built: &Built, settings: &GcadSettings, load_context: &mut LoadContext) -> WorldAsset {
    let mut world = World::default();
    let (scale, translation) = built.fit.unwrap_or((1.0, [0.0; 3]));
    let (lo, hi) = built.bounds.unwrap_or(([0.0; 3], [0.0; 3]));
    let names = built.clip_names();
    let root = world
        .spawn((
            Transform::from_scale(Vec3::splat(scale)).with_translation(Vec3::from(translation)),
            Visibility::default(),
            Name::new("gcad"),
            GcadModel {
                clips: names.clone(),
                bounds_min: Vec3::from(lo),
                bounds_max: Vec3::from(hi),
                triangles: built.triangles(),
            },
        ))
        .id();
    let rigged = built.bodies.iter().any(|b| !b.joints.is_empty());
    let player_root = rigged.then_some(root);
    if rigged {
        world.entity_mut(root).insert(AnimationPlayer::default());
    }
    let mut clips: Vec<(String, AnimationClip)> = names
        .iter()
        .map(|n| (n.clone(), AnimationClip::default()))
        .collect();
    for body in &built.bodies {
        let body_name = Name::new(body.name.clone());
        let body_root = world
            .spawn((Transform::default(), Visibility::default(), body_name.clone()))
            .id();
        world.entity_mut(root).add_child(body_root);
        let mut joints = Vec::new();
        let mut paths: Vec<Vec<Name>> = Vec::new();
        for (i, j) in body.joints.iter().enumerate() {
            let name = Name::new(j.name.clone());
            let path = match j.parent {
                Some(p) => {
                    let mut v = paths[p as usize].clone();
                    v.push(name.clone());
                    v
                }
                None => vec![body_name.clone(), name.clone()],
            };
            let mut e = world.spawn((
                Transform::from_translation(local_offset(body, i)),
                Visibility::default(),
                name,
                AnimationTargetId::from_names(path.iter()),
            ));
            if let Some(player) = player_root {
                e.insert(AnimatedBy(player));
            }
            let e = e.id();
            let parent = j.parent.map_or(body_root, |p| joints[p as usize]);
            world.entity_mut(parent).add_child(e);
            joints.push(e);
            paths.push(path);
        }
        let skin = (!body.joints.is_empty()).then(|| {
            let binds: Vec<Mat4> = body
                .joints
                .iter()
                .map(|j| Mat4::from_translation(-Vec3::from(j.head)))
                .collect();
            load_context.add_labeled_asset(format!("{}/Skin", body.name), SkinnedMeshInverseBindposes::from(binds))
        });
        let morph_name = (!body.targets.is_empty()).then(|| Name::new(format!("{}/Morphs", body.name)));
        for (name, clip) in &mut clips {
            if let Some(source) = body.clips.iter().find(|c| &c.name == name) {
                let part = clip_of(source, &paths, morph_name.as_ref());
                merge(clip, part);
            }
        }
        let levels: Vec<BuiltBody> = std::iter::once(body.clone())
            .chain(body.lods.iter().map(|l| l.body(body)))
            .collect();
        let normal_texture = body
            .normal_map
            .as_ref()
            .and_then(normal_image)
            .map(|image| load_context.add_labeled_asset(format!("{}/Normal", body.name), image));
        for (level, lod) in levels.iter().enumerate() {
            let holder = match (&morph_name, level) {
                (Some(name), 0) => {
                    let mut e = world.spawn((
                        Transform::default(),
                        Visibility::default(),
                        name.clone(),
                        AnimationTargetId::from_name(name),
                    ));
                    if let Some(player) = player_root {
                        e.insert(AnimatedBy(player));
                    }
                    let e = e.id();
                    world.entity_mut(body_root).add_child(e);
                    e
                }
                _ => body_root,
            };
            let mut first_mesh = None;
            for part in &lod.parts {
                let label = match level {
                    0 => format!("{}/{}", body.name, part.name),
                    k => format!("{}/LOD{k}/{}", body.name, part.name),
                };
                let mesh = load_context.add_labeled_asset(format!("{label}/Mesh"), mesh_of(lod, part));
                first_mesh.get_or_insert_with(|| mesh.clone());
                let mut material = material_of(lod, part);
                if level == 0 {
                    material.normal_map_texture = normal_texture.clone();
                }
                let material = load_context.add_labeled_asset(format!("{label}/Material"), material);
                let mut e = world.spawn((
                    Mesh3d(mesh),
                    MeshMaterial3d(material),
                    Transform::default(),
                    Visibility::default(),
                    Name::new(part.name.clone()),
                    GcadPart {
                        body: body.name.clone(),
                        part: part.name.clone(),
                        lod: level,
                    },
                ));
                if let Some(binds) = &skin {
                    e.insert((
                        SkinnedMesh {
                            inverse_bindposes: binds.clone(),
                            joints: joints.clone(),
                        },
                        NoFrustumCulling,
                    ));
                }
                if level == 0 && morph_name.is_some() {
                    e.insert(MeshMorphWeights::Reference(holder));
                }
                if let Some(range) = lod_range(&settings.lod_distances, level, levels.len()) {
                    e.insert(range);
                }
                let id = e.id();
                world.entity_mut(holder).add_child(id);
            }
            if level == 0 && morph_name.is_some() {
                if let Ok(weights) = MorphWeights::new(body.target_weights.clone(), first_mesh) {
                    world.entity_mut(holder).insert(weights);
                }
            }
        }
    }
    for (name, clip) in clips {
        load_context.add_labeled_asset(format!("Animation/{name}"), clip);
    }
    WorldAsset::new(world)
}

fn merge(into: &mut AnimationClip, from: AnimationClip) {
    let duration = into.duration().max(from.duration());
    for (target, curves) in from.curves() {
        for curve in curves {
            into.add_variable_curve_to_target(*target, curve.clone());
        }
    }
    into.set_duration(duration);
}
