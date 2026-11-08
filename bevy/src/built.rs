use bitcode::{Decode, Encode};

#[derive(Clone, Debug, Default, Encode, Decode, PartialEq)]
pub struct Built {
    pub bodies: Vec<BuiltBody>,
    pub bounds: Option<([f32; 3], [f32; 3])>,
    pub fit: Option<(f32, [f32; 3])>,
}

#[derive(Clone, Debug, Default, Encode, Decode, PartialEq)]
pub struct BuiltBody {
    pub name: String,
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub colours: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
    pub parts: Vec<BuiltPart>,
    pub joints: Vec<BuiltJoint>,
    pub joint_indices: Vec<[u16; 4]>,
    pub joint_weights: Vec<[f32; 4]>,
    pub clips: Vec<BuiltClip>,
    pub targets: Vec<BuiltTarget>,
    pub target_weights: Vec<f32>,
    pub lods: Vec<BuiltLod>,
    pub normal_map: Option<BuiltNormalMap>,
}

#[derive(Clone, Debug, Default, Encode, Decode, PartialEq)]
pub struct BuiltNormalMap {
    pub png: Vec<u8>,
    pub uvs: Vec<[f32; 2]>,
    pub tangents: Vec<[f32; 4]>,
}

#[derive(Clone, Debug, Default, Encode, Decode, PartialEq)]
pub struct BuiltLod {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
    pub parts: Vec<BuiltPart>,
    pub joint_indices: Vec<[u16; 4]>,
    pub joint_weights: Vec<[f32; 4]>,
}

impl BuiltLod {
    pub fn body(&self, base: &BuiltBody) -> BuiltBody {
        BuiltBody {
            name: base.name.clone(),
            positions: self.positions.clone(),
            normals: self.normals.clone(),
            indices: self.indices.clone(),
            parts: self.parts.clone(),
            joints: base.joints.clone(),
            joint_indices: self.joint_indices.clone(),
            joint_weights: self.joint_weights.clone(),
            ..Default::default()
        }
    }
}

#[derive(Clone, Debug, Default, Encode, Decode, PartialEq)]
pub struct BuiltPart {
    pub name: String,
    pub colour: [f32; 3],
    pub gloss: f32,
    pub first_index: u32,
    pub index_count: u32,
}

#[derive(Clone, Debug, Default, Encode, Decode, PartialEq)]
pub struct BuiltJoint {
    pub name: String,
    pub parent: Option<u32>,
    pub head: [f32; 3],
}

#[derive(Clone, Debug, Default, Encode, Decode, PartialEq)]
pub struct BuiltTarget {
    pub name: String,
    pub deltas: Vec<[f32; 3]>,
}

#[derive(Clone, Debug, Default, Encode, Decode, PartialEq)]
pub struct BuiltClip {
    pub name: String,
    pub fps: f32,
    pub duration: f32,
    pub rotations: Vec<BuiltRotations>,
    pub translations: Vec<BuiltTranslations>,
    pub weights: Option<BuiltWeights>,
}

#[derive(Clone, Debug, Default, Encode, Decode, PartialEq)]
pub struct BuiltRotations {
    pub joint: u32,
    pub times: Vec<f32>,
    pub values: Vec<[f32; 4]>,
}

#[derive(Clone, Debug, Default, Encode, Decode, PartialEq)]
pub struct BuiltTranslations {
    pub joint: u32,
    pub times: Vec<f32>,
    pub values: Vec<[f32; 3]>,
}

#[derive(Clone, Debug, Default, Encode, Decode, PartialEq)]
pub struct BuiltWeights {
    pub times: Vec<f32>,
    pub weights: Vec<Vec<f32>>,
}

impl Built {
    pub fn triangles(&self) -> usize {
        self.bodies.iter().map(|b| b.indices.len() / 3).sum()
    }

    pub fn clip_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .bodies
            .iter()
            .flat_map(|b| b.clips.iter().map(|c| c.name.clone()))
            .collect();
        names.sort();
        names.dedup();
        names
    }
}
