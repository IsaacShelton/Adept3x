use crate::LspRequestId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LspRequest {
    pub id: LspRequestId,
    pub method: String,
    #[serde(default = "serde_json::Value::default")]
    #[serde(skip_serializing_if = "serde_json::Value::is_null")]
    pub params: serde_json::Value,
}
