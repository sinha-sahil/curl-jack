use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ApiResponse {
    pub status: u16,
    #[serde(default)]
    pub status_text: String,
    #[serde(default)]
    pub ok: bool,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    #[serde(default)]
    pub content_type: Option<String>,
    #[serde(default)]
    pub content_length: Option<u64>,
    #[serde(default)]
    pub cookies: BTreeMap<String, String>,
    #[serde(default)]
    pub body_text: Option<String>,
    #[serde(default)]
    pub body_json: Option<serde_json::Value>,
    #[serde(default)]
    pub elapsed_ms: u64,
}
