use crate::built::{
    Built, BuiltBody, BuiltClip, BuiltJoint, BuiltLod, BuiltPart, BuiltRotations, BuiltTarget,
    BuiltTranslations, BuiltWeights,
};
use crate::settings::{Centre, GcadSettings};
use std::path::Path;

pub const GCAD_VERSION: &str = gcad2glb::VERSION;

pub fn key(source: &str, settings: &GcadSettings) -> u64 {
    gcad2glb::cache_key(source, &options(settings, false))
}

pub fn build(source: &str, file: Option<&Path>, settings: &GcadSettings) -> Result<Built, String> {
    let options = options(settings, false);
    let assets = match file {
        Some(path) => gcad2glb::Assets::from_path(path, &options),
        None => gcad2glb::Assets::from_source(source, &options),
    }
    .map_err(|e| e.to_string())?;
    Ok(convert(assets, settings))
}

fn options(settings: &GcadSettings, centre: bool) -> gcad2glb::Options {
    let mut options = gcad2glb::Options {
        detail: settings.detail,
        triangles: settings.triangles,
        max_error: settings.max_error,
        fit: gcad2glb::Fit {
            height: settings.height,
            scale: settings.scale,
            centre: match settings.centre {
                Centre::Yes => true,
                Centre::No => false,
                Centre::Unrigged => centre,
            },
        },
        lods: settings.lods.as_ref().map(|lods| {
            lods.iter()
                .map(|l| match *l {
                    crate::settings::Lod::Triangles(n) => gcad2glb::LodSize::Triangles(n),
                    crate::settings::Lod::Share(s) => gcad2glb::LodSize::Share(s),
                })
                .collect()
        }),
        ..Default::default()
    };
    for (name, value) in &settings.vars {
        options = options.set(name, *value);
    }
    options
}

fn convert(mut assets: gcad2glb::Assets, settings: &GcadSettings) -> Built {
    let rigged = assets.bodies.iter().any(|b| !b.joints.is_empty());
    assets.options.fit.centre = options(settings, !rigged).fit.centre;
    let fit = assets.fit().map(|t| (t.scale, t.translation));
    Built {
        bounds: assets.bounds(),
        fit,
        bodies: assets
            .bodies
            .iter()
            .map(|b| {
                let mut body = body_of(b);
                body.lods = b
                    .lods()
                    .iter()
                    .map(|l| BuiltLod {
                        positions: l.positions.clone(),
                        normals: l.normals.clone(),
                        indices: l.indices.clone(),
                        parts: parts_of(l),
                        joint_indices: l.joint_indices.clone(),
                        joint_weights: l.joint_weights.clone(),
                    })
                    .collect();
                body
            })
            .collect(),
    }
}

fn parts_of(b: &gcad2glb::Body) -> Vec<BuiltPart> {
    b.parts
        .iter()
        .map(|p| BuiltPart {
            name: p.name.clone(),
            colour: p.colour,
            gloss: p.gloss,
            first_index: p.first_index as u32,
            index_count: p.index_count as u32,
        })
        .collect()
}

fn body_of(b: &gcad2glb::Body) -> BuiltBody {
    BuiltBody {
        name: b.name.clone(),
        positions: b.positions.clone(),
        normals: b.normals.clone(),
        colours: b.colours.clone(),
        indices: b.indices.clone(),
        parts: parts_of(b),
        joints: b
            .joints
            .iter()
            .map(|j| BuiltJoint {
                name: j.name.clone(),
                parent: j.parent.map(|p| p as u32),
                head: j.head,
            })
            .collect(),
        joint_indices: b.joint_indices.clone(),
        joint_weights: b.joint_weights.clone(),
        clips: b.clips.iter().map(clip_of).collect(),
        targets: b
            .targets
            .iter()
            .map(|t| BuiltTarget {
                name: t.name.clone(),
                deltas: t.deltas.clone(),
            })
            .collect(),
        target_weights: b.target_weights.clone(),
        lods: Vec::new(),
    }
}

fn clip_of(c: &gcad2glb::Clip) -> BuiltClip {
    BuiltClip {
        name: c.name.clone(),
        fps: c.fps,
        duration: c.duration,
        rotations: c
            .tracks
            .iter()
            .map(|t| BuiltRotations {
                joint: t.joint as u32,
                times: t.times.clone(),
                values: t.rotations.clone(),
            })
            .collect(),
        translations: c
            .moves
            .iter()
            .map(|t| BuiltTranslations {
                joint: t.joint as u32,
                times: t.times.clone(),
                values: t.offsets.clone(),
            })
            .collect(),
        weights: c.weights.as_ref().map(|w| BuiltWeights {
            times: w.times.clone(),
            weights: w.weights.clone(),
        }),
    }
}
