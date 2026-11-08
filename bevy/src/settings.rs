use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Centre {
    #[default]
    No,
    Yes,
    Unrigged,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub enum Lod {
    Triangles(usize),
    Share(f64),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct GcadSettings {
    pub detail: f64,
    pub vars: Vec<(String, f64)>,
    pub height: Option<f64>,
    pub scale: Option<f64>,
    pub centre: Centre,
    pub triangles: Option<usize>,
    pub max_error: Option<f64>,
    pub lods: Option<Vec<Lod>>,
    pub lod_distances: Vec<f32>,
    pub cache: bool,
}

impl Default for GcadSettings {
    fn default() -> Self {
        GcadSettings {
            detail: 1.0,
            vars: Vec::new(),
            height: None,
            scale: None,
            centre: Centre::No,
            triangles: None,
            max_error: None,
            lods: None,
            lod_distances: Vec::new(),
            cache: true,
        }
    }
}

impl GcadSettings {
    pub fn fit_height(height: f64) -> Self {
        GcadSettings {
            height: Some(height),
            centre: Centre::Unrigged,
            ..Default::default()
        }
    }

    pub fn set(mut self, name: &str, value: f64) -> Self {
        self.vars.retain(|(n, _)| n != name);
        self.vars.push((name.to_string(), value));
        self
    }
}
