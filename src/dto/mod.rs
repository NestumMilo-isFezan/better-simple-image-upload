use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct PresignResponse {
    pub token: Uuid,
    pub signed_url: String,
}

#[derive(Deserialize)]
pub struct OptParams {
    #[serde(rename = "type")]
    pub img_type: Option<String>,
    pub scale: Option<f32>,
}
