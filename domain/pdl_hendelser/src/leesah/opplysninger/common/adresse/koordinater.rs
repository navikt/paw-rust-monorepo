use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Koordinater {
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub z: Option<f32>,
}
