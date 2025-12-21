use crate::LspRequestId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LspResponse {
    pub id: LspRequestId,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub error: Option<LspResponseError>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LspResponseError {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub data: Option<serde_json::Value>,
}
