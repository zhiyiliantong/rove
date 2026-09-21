//! Wire validation is compiled directly from the versioned OpenAPI contracts.
pub mod contract;
pub mod dto;
pub mod frame;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use uuid::Uuid;

pub const PROTOCOL_VERSION: u32 = 1;
pub const AGENT_OPENAPI: &str = include_str!("../../../api/rove-agent.openapi.json");
pub const BLOB_OPENAPI: &str = include_str!("../../../api/rove-config-server.openapi.json");

#[derive(Clone, Debug, Serialize, Deserialize, thiserror::Error)]
#[error("{code}: {message}")]
#[serde(deny_unknown_fields)]
pub struct ApiError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
    #[serde(skip)]
    pub status: u16,
}
impl ApiError {
    pub fn new(status: u16, code: &str, message: &str) -> Self {
        Self {
            status,
            code: code.into(),
            message: message.into(),
            retryable: matches!(status, 429 | 502 | 503 | 504),
        }
    }
    pub fn invalid(message: &str) -> Self {
        Self::new(400, "invalid_request", message)
    }
    pub fn body(&self) -> Value {
        json!({"error": self})
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SocketTarget {
    pub network_id: Uuid,
    pub device_id: Uuid,
}

// Do not derive Debug: requests can contain model keys or join secrets.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub kind: String,
    pub correlation_id: Uuid,
    pub operation_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<SocketTarget>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub path_parameters: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub query_parameters: BTreeMap<String, Value>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_value"
    )]
    pub body: Option<Value>,
}
fn present_value<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<Value>, D::Error> {
    Value::deserialize(d).map(Some)
}
impl Request {
    pub fn new(operation_id: &str) -> Self {
        Self {
            kind: "request".into(),
            correlation_id: Uuid::new_v4(),
            operation_id: operation_id.into(),
            target: None,
            path_parameters: BTreeMap::new(),
            query_parameters: BTreeMap::new(),
            body: None,
        }
    }
    pub fn with_body(mut self, body: Value) -> Self {
        self.body = Some(body);
        self
    }
    pub fn with_path(mut self, name: &str, value: impl ToString) -> Self {
        self.path_parameters.insert(name.into(), value.to_string());
        self
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Response {
    pub kind: String,
    pub correlation_id: Uuid,
    pub status_code: u16,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_value"
    )]
    pub body: Option<Value>,
}
impl Response {
    pub fn new(request: &Request, status_code: u16, body: Option<Value>) -> Self {
        Self {
            kind: "response".into(),
            correlation_id: request.correlation_id,
            status_code,
            body,
        }
    }
    pub fn error(request: &Request, error: ApiError) -> Self {
        Self::new(request, error.status, Some(error.body()))
    }
}

pub fn check_protocol(min: u64, max: u64) -> Result<(), ApiError> {
    if min > max || !(min..=max).contains(&(PROTOCOL_VERSION as u64)) {
        return Err(ApiError::new(
            409,
            "protocol_incompatible",
            "No common protocol version; upgrade the client or agent",
        ));
    }
    Ok(())
}
