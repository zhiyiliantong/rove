//! Bounded provider listing. Native paths/headers follow the pinned Rig adapters.
//! Return identifiers only; never forward provider error bodies or credential fields.
use rove_protocol::ApiError;
use serde_json::{Value, json};

pub(crate) async fn discover(body: &Value) -> Result<Value, ApiError> {
    crate::models::validate_connection(body)?;
    tokio::time::timeout(std::time::Duration::from_secs(15), list(body))
        .await
        .map_err(|_| {
            ApiError::new(
                504,
                "model_discovery_timeout",
                "Model listing timed out; manual entry remains available",
            )
        })?
}
async fn list(body: &Value) -> Result<Value, ApiError> {
    let failure = || {
        ApiError::new(
            502,
            "model_discovery_failed",
            "Cannot list models; check address, API key and provider support, or enter model identifiers manually",
        )
    };
    let provider = crate::ai::adapter_provider(body);
    let base = body["base_url"].as_str().unwrap().trim_end_matches('/');
    let endpoint = match provider {
        "anthropic" => format!("{}/v1/models", base.trim_end_matches("/v1")),
        "gemini" => format!("{}/v1beta/models", base.trim_end_matches("/v1beta")),
        "ollama" => format!("{base}/api/tags"),
        _ => format!("{base}/models"),
    };
    let http = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|_| failure())?;
    let mut models = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut cursors = std::collections::HashSet::new();
    let mut cursor: Option<String> = None;
    for page in 0..10 {
        let mut request = http.get(&endpoint);
        if let Some(key) = body["api_key"].as_str().filter(|k| !k.is_empty()) {
            request = match provider {
                "anthropic" => request.header("x-api-key", key),
                "gemini" => request.header("x-goog-api-key", key),
                _ => request.bearer_auth(key),
            };
        }
        if provider == "anthropic" {
            request = request.header("anthropic-version", "2023-06-01");
        }
        if let Some(ref cursor) = cursor {
            request = request.query(&[(
                if provider == "anthropic" {
                    "after_id"
                } else {
                    "pageToken"
                },
                cursor,
            )]);
        }
        let mut response = request.send().await.map_err(|_| failure())?;
        if !response.status().is_success() {
            return Err(failure());
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| failure())? {
            if bytes.len() + chunk.len() > 1024 * 1024 {
                return Err(ApiError::new(
                    413,
                    "model_listing_too_large",
                    "Provider model list exceeds 1 MiB per page",
                ));
            }
            bytes.extend_from_slice(&chunk);
        }
        let value: Value = serde_json::from_slice(&bytes).map_err(|_| failure())?;
        let entries = value[if matches!(provider, "ollama" | "gemini") {
            "models"
        } else {
            "data"
        }]
        .as_array()
        .ok_or_else(failure)?;
        for entry in entries {
            let id = entry[if matches!(provider, "ollama" | "gemini") {
                "name"
            } else {
                "id"
            }]
            .as_str()
            .ok_or_else(failure)?;
            let id = if provider == "gemini" {
                id.strip_prefix("models/").unwrap_or(id)
            } else {
                id
            };
            if id.is_empty() || id.chars().count() > 256 {
                continue;
            }
            if seen.insert(id.to_owned()) {
                if models.len() == 2000 {
                    return Ok(json!({"models":models,"truncated":true}));
                }
                models.push(id.to_owned());
            }
        }
        cursor = match provider {
            "anthropic" if value["has_more"] == true => {
                Some(value["last_id"].as_str().ok_or_else(failure)?.to_owned())
            }
            "gemini" => value["nextPageToken"]
                .as_str()
                .filter(|s| !s.is_empty())
                .map(str::to_owned),
            _ => None,
        };
        if let Some(ref next) = cursor {
            if !cursors.insert(next.clone()) {
                return Err(failure());
            }
            if page == 9 {
                return Ok(json!({"models":models,"truncated":true}));
            }
        } else {
            return Ok(json!({"models":models,"truncated":false}));
        }
    }
    unreachable!()
}
