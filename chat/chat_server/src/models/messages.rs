use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
pub struct CreateMessage {
    pub content: String,
    #[serde(default)]
    pub files: Vec<String>,
}

#[derive(Debug, Clone, IntoParams, ToSchema, Serialize, Deserialize)]
pub struct ListMessages {
    #[serde(default)]
    pub last_id: Option<u64>,
    #[serde(default)]
    pub limit: u64,
}
