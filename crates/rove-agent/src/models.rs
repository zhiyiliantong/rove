//! Model entries share a private connection; public catalog never returns secrets.
#[cfg(all(test, target_os = "linux"))]
#[path = "model_sync_tests.rs"]
mod sync_tests;
use crate::{Agent, storage_error};
use rove_protocol::{ApiError, Request};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

fn fingerprint(config: &Value) -> String {
    format!("{:x}", Sha256::digest(config.to_string().as_bytes()))
}

impl Agent {
    pub(crate) async fn test_model(&self, body: &Value) -> Result<Value, ApiError> {
        // Serialize only explicit probes, never normal AI jobs. No DB lock across HTTP.
        static PROBES: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(1);
        let _permit = PROBES.try_acquire().map_err(|_| {
            ApiError::new(409, "model_test_busy", "Another model test is in progress")
        })?;
        let config = if let Some(id) = body["model_id"].as_str() {
            let db = self.store.connection.lock().unwrap();
            selected(&db, Some(id))?
        } else {
            json!({"provider":body["provider"],"base_url":body["base_url"],"api_key":body["api_key"],"model":body["model"]})
        };
        if config.is_null() {
            return Err(ApiError::new(404, "model_not_found", "Model not found"));
        }
        validate_connection(&config)?;
        let key = fingerprint(&config);
        let result = crate::ai::probe(config).await;
        let tested_at = rove_core::now();
        {
            let db = self.store.connection.lock().unwrap();
            let mut tests = read(&db, "model_tests").map_err(storage_error)?;
            if !tests.is_array() {
                tests = json!([]);
            }
            let entries = tests.as_array_mut().unwrap();
            entries.retain(|entry| entry["key"] != key);
            if entries.len() >= 1000 {
                entries.remove(0);
            }
            entries.push(json!({"key":key,"tested_at":tested_at,"available":result.is_ok()}));
            write(&db, "model_tests", &tests).map_err(storage_error)?;
        }
        result?;
        Ok(json!({"available":true,"tested_at":tested_at}))
    }
}

fn read(db: &Connection, key: &str) -> anyhow::Result<Value> {
    let raw: Option<String> = db
        .query_row("SELECT value FROM metadata WHERE key=?1", [key], |r| {
            r.get(0)
        })
        .optional()?;
    Ok(raw
        .map(|s| serde_json::from_str(&s))
        .transpose()?
        .unwrap_or(Value::Null))
}
fn write(db: &Connection, key: &str, value: &Value) -> anyhow::Result<()> {
    db.execute("INSERT INTO metadata(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value", params![key,value.to_string()])?;
    Ok(())
}
fn empty() -> Value {
    json!({"connections":[],"models":[],"default_model_id":null})
}
fn new_id() -> Value {
    json!(uuid::Uuid::new_v4())
}
fn add_legacy(catalog: &mut Value, config: &Value) {
    let connection_id = new_id();
    let model_id = new_id();
    let name = model_name(
        catalog,
        config["provider"].as_str().unwrap_or("legacy"),
        config["model"].as_str().unwrap_or("model"),
    );
    catalog["connections"].as_array_mut().unwrap().push(json!({"connection_id":connection_id,"provider":config["provider"],"base_url":config["base_url"],"api_key":config["api_key"],"legacy":true}));
    catalog["models"].as_array_mut().unwrap().push(json!({"model_id":model_id,"connection_id":connection_id,"name":name,"model":config["model"]}));
    catalog["default_model_id"] = model_id;
}
pub(crate) fn migrate(db: &Connection) -> anyhow::Result<()> {
    if read(db, "model_catalog")?.is_null() {
        let mut catalog = empty();
        let config = read(db, "model_config")?;
        if !config.is_null() {
            add_legacy(&mut catalog, &config);
            write(db, "network_onboarding_seen", &json!(true))?;
        }
        write(db, "model_catalog", &catalog)?;
    }
    Ok(())
}
fn resolve(catalog: &Value, id: &Value) -> Value {
    let Some(model) = catalog["models"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| &m["model_id"] == id)
    else {
        return Value::Null;
    };
    let Some(connection) = catalog["connections"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["connection_id"] == model["connection_id"])
    else {
        return Value::Null;
    };
    json!({"provider":connection["provider"],"base_url":connection["base_url"],"api_key":connection["api_key"],"model":model["model"]})
}
pub(crate) fn selected(db: &Connection, id: Option<&str>) -> Result<Value, ApiError> {
    if let Some(id) = id {
        Ok(resolve(
            &read(db, "model_catalog").map_err(storage_error)?,
            &json!(id),
        ))
    } else {
        read(db, "model_config").map_err(storage_error)
    }
}
fn public(catalog: &Value, tests: &Value) -> Value {
    let mut result = catalog.clone();
    for model in result["models"].as_array_mut().unwrap() {
        let key = fingerprint(&resolve(catalog, &model["model_id"]));
        let passed = tests.as_array().and_then(|entries| {
            entries
                .iter()
                .find(|test| test["key"] == key && test["available"] == true)
        });
        model["tested_at"] = passed
            .map(|test| test["tested_at"].clone())
            .unwrap_or(Value::Null);
    }
    for c in result["connections"].as_array_mut().unwrap() {
        c["api_key_configured"] = json!(c["api_key"].as_str().is_some_and(|s| !s.is_empty()));
        c.as_object_mut().unwrap().remove("api_key");
        c.as_object_mut().unwrap().remove("legacy");
        c.as_object_mut().unwrap().remove("sync_source");
    }
    result
}
pub(crate) fn validate_connection(body: &Value) -> Result<(), ApiError> {
    let url = url::Url::parse(body["base_url"].as_str().unwrap())
        .map_err(|_| ApiError::invalid("Invalid model base URL"))?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(ApiError::invalid(
            "Model URL must be HTTP(S), without embedded credentials, query or fragment",
        ));
    }
    Ok(())
}
fn remove_connection(catalog: &mut Value, id: &Value) {
    catalog["connections"]
        .as_array_mut()
        .unwrap()
        .retain(|c| &c["connection_id"] != id);
    catalog["models"]
        .as_array_mut()
        .unwrap()
        .retain(|m| &m["connection_id"] != id);
    if resolve(catalog, &catalog["default_model_id"]).is_null() {
        catalog["default_model_id"] = Value::Null;
    }
}
fn slug(s: &str) -> String {
    s.to_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '.' && c != '-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}
fn model_name(catalog: &Value, provider: &str, model: &str) -> String {
    let p = slug(provider);
    let m = slug(model);
    let stem = if m.starts_with(&format!("{p}-")) {
        m
    } else {
        format!("{p}-{m}")
    };
    // Bound the generated name independently of provider/model input lengths.
    let stem = &stem[..stem.len().min(230)];
    let prefix = format!("{stem}-");
    let next = catalog["models"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|m| {
            m["name"]
                .as_str()?
                .strip_prefix(&prefix)?
                .parse::<u64>()
                .ok()
        })
        .max()
        .unwrap_or(0)
        .saturating_add(1);
    format!("{prefix}{next:02}")
}

// Sync is an explicit one-way snapshot, never a background account/session sync.
fn connection_snapshot(catalog: &Value, id: &Value) -> Result<Value, ApiError> {
    let connection = catalog["connections"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["connection_id"] == *id)
        .ok_or_else(|| {
            ApiError::new(
                404,
                "model_connection_not_found",
                "Model connection not found",
            )
        })?;
    if connection.get("auth_type").is_some_and(|v| v != "api_key")
        || connection["api_key"]
            .as_str()
            .is_none_or(|s| s.trim().is_empty())
    {
        return Err(ApiError::new(
            409,
            "model_sync_api_only",
            "Only API key connections can be synchronized; account login is per device",
        ));
    }
    let models: Vec<_> = catalog["models"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|m| m["connection_id"] == *id)
        .map(|m| json!({"model":m["model"],"name":m["name"]}))
        .collect();
    Ok(
        json!({"provider":connection["provider"],"base_url":connection["base_url"],"api_key":connection["api_key"],"models":models,"set_default":false}),
    )
}

fn apply_sync(catalog: &mut Value, body: &Value) -> Result<(), ApiError> {
    let config = &body["config"];
    validate_connection(config)?;
    if config["set_default"] != false
        || config["api_key"]
            .as_str()
            .is_none_or(|s| s.trim().is_empty())
    {
        return Err(ApiError::new(
            409,
            "model_sync_api_only",
            "Sync requires an API key and cannot change the default model",
        ));
    }
    let source =
        json!({"device_id":body["source_device_id"],"connection_id":body["source_connection_id"]});
    let known = catalog["connections"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["sync_source"] == source)
        .map(|c| c["connection_id"].clone());
    let replacement = body.get("replace_connection_id").cloned();
    if replacement.is_some() && body["overwrite"] != true {
        return Err(ApiError::new(
            409,
            "model_sync_conflict",
            "Confirm overwrite before replacing a target connection",
        ));
    }
    if known.is_some() && replacement.is_some() && known != replacement {
        return Err(ApiError::new(
            409,
            "model_sync_conflict",
            "This source is already synchronized to another connection",
        ));
    }
    let existing = known.or(replacement);
    if let Some(id) = &existing {
        // Check existence even when the old connection lacks an API key.
        if !catalog["connections"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["connection_id"] == *id)
        {
            return Err(ApiError::new(
                404,
                "model_connection_not_found",
                "Target model connection not found",
            ));
        }
        if connection_snapshot(catalog, id).ok().as_ref() == Some(config) {
            let connection = catalog["connections"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|c| c["connection_id"] == *id)
                .unwrap();
            connection["sync_source"] = source;
            return Ok(());
        }
        if catalog["connections"].as_array().unwrap().iter().any(|c| {
            c["connection_id"] == *id && c.get("auth_type").is_some_and(|v| v != "api_key")
        }) {
            return Err(ApiError::new(
                409,
                "model_sync_api_only",
                "Account login connections cannot be replaced by API synchronization",
            ));
        }
        if body["overwrite"] != true {
            return Err(ApiError::new(
                409,
                "model_sync_conflict",
                "Target configuration differs; explicit overwrite confirmation is required",
            ));
        }
    }
    let id = existing.clone().unwrap_or_else(new_id);
    let old_models: Vec<_> = catalog["models"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|m| m["connection_id"] == id)
        .cloned()
        .collect();
    let mut next = catalog.clone();
    remove_connection(&mut next, &id);
    if next["models"].as_array().unwrap().len() + config["models"].as_array().unwrap().len() > 1000
    {
        return Err(ApiError::new(
            409,
            "model_catalog_full",
            "At most 1000 model entries per device",
        ));
    }
    let mut seen = std::collections::HashSet::new();
    for model in config["models"].as_array().unwrap() {
        let identifier = model["model"].as_str().unwrap().trim();
        let name = model["name"].as_str().unwrap_or("").trim();
        if identifier.is_empty() || name.is_empty() || !seen.insert(identifier) {
            return Err(ApiError::invalid(
                "Sync requires unique model identifiers and explicit nonempty names",
            ));
        }
        if next["models"]
            .as_array()
            .unwrap()
            .iter()
            .any(|m| m["name"] == name)
        {
            return Err(ApiError::new(
                409,
                "model_name_conflict",
                "A model name exists outside the selected replacement connection",
            ));
        }
        let model_id = old_models
            .iter()
            .find(|m| m["model"] == identifier)
            .map(|m| m["model_id"].clone())
            .unwrap_or_else(new_id);
        next["models"]
            .as_array_mut()
            .unwrap()
            .push(json!({"model_id":model_id,"connection_id":id,"model":identifier,"name":name}));
    }
    next["connections"].as_array_mut().unwrap().push(json!({"connection_id":id,"provider":config["provider"],"base_url":config["base_url"],"api_key":config["api_key"],"sync_source":source}));
    // Existing session/default references survive when the model identifier survives.
    if !resolve(&next, &catalog["default_model_id"]).is_null() {
        next["default_model_id"] = catalog["default_model_id"].clone();
    }
    *catalog = next;
    Ok(())
}

impl Agent {
    pub(crate) async fn sync_model_connection(
        &self,
        request: &Request,
    ) -> Result<(u16, Option<Value>), ApiError> {
        let body = request.body.as_ref().unwrap();
        let target: rove_protocol::SocketTarget = serde_json::from_value(body["target"].clone())
            .map_err(|_| ApiError::invalid("Invalid synchronization target"))?;
        if target.device_id == self.store.device_id.0 {
            return Err(ApiError::invalid(
                "Choose another device for synchronization",
            ));
        }
        let config = {
            let db = self.store.connection.lock().unwrap();
            let catalog = read(&db, "model_catalog").map_err(storage_error)?;
            connection_snapshot(&catalog, &json!(request.path_parameters["connection_id"]))?
        };
        let mut payload = json!({"source_device_id":self.store.device_id,"source_connection_id":request.path_parameters["connection_id"],"config":config,"overwrite":body["overwrite"] == true});
        if let Some(id) = body.get("replace_connection_id") {
            payload["replace_connection_id"] = id.clone();
        }
        let forwarded = Request::new("receive_model_sync").with_body(payload);
        rove_protocol::contract::AGENT.request(&forwarded)?;
        let response = self.linked_client(&target).await?.call(forwarded).await
            .map_err(|_| ApiError::new(503, "model_sync_unknown", "Sync outcome is unknown; inspect the target or retry unchanged input; no local fallback occurred"))?;
        if response.status_code >= 400 {
            // Never relay peer-supplied error text, which may contain the submitted key.
            let code = response
                .body
                .as_ref()
                .and_then(|v| v["error"]["code"].as_str())
                .unwrap_or("");
            return Err(match code {
                "model_sync_conflict" | "model_name_conflict" => ApiError::new(
                    409,
                    "model_sync_conflict",
                    "Target configuration conflicts; select a replacement connection and confirm overwrite",
                ),
                "model_catalog_full" => {
                    ApiError::new(409, "model_catalog_full", "Target model catalog is full")
                }
                "model_connection_not_found" => ApiError::new(
                    404,
                    "model_connection_not_found",
                    "Target connection no longer exists",
                ),
                _ => ApiError::new(
                    response.status_code,
                    "model_sync_failed",
                    "Target rejected synchronization; credentials were not returned",
                ),
            });
        }
        Ok((200, response.body))
    }

    pub(crate) fn model_operation(
        &self,
        request: &Request,
    ) -> Result<(u16, Option<Value>), ApiError> {
        let body = request.body.as_ref().unwrap_or(&Value::Null);
        let mut db = self.store.connection.lock().unwrap();
        let tx = db.transaction().map_err(|e| storage_error(e.into()))?;
        let mut catalog = read(&tx, "model_catalog").map_err(storage_error)?;
        let tests = read(&tx, "model_tests").map_err(storage_error)?;
        let first_model = catalog["models"].as_array().unwrap().is_empty()
            && read(&tx, "network_onboarding_seen")
                .map_err(storage_error)?
                .is_null();
        let op = request.operation_id.as_str();
        match op {
            "get_model_catalog" => return Ok((200, Some(public(&catalog, &tests)))),
            "receive_model_sync" => apply_sync(&mut catalog, body)?,
            "import_models" => {
                validate_connection(body)?;
                if catalog["models"].as_array().unwrap().len()
                    + body["models"].as_array().unwrap().len()
                    > 1000
                {
                    return Err(ApiError::new(
                        409,
                        "model_catalog_full",
                        "At most 1000 model entries per device",
                    ));
                }
                let connection_id = new_id();
                catalog["connections"].as_array_mut().unwrap().push(json!({"connection_id":connection_id,"provider":body["provider"],"base_url":body["base_url"],"api_key":body["api_key"]}));
                let mut seen = std::collections::HashSet::new();
                let mut first = Value::Null;
                for entry in body["models"].as_array().unwrap() {
                    let model = entry["model"].as_str().unwrap().trim();
                    if model.is_empty() || !seen.insert(model) {
                        return Err(ApiError::invalid(
                            "Model identifiers must be nonempty and unique within a connection",
                        ));
                    }
                    let name = entry["name"]
                        .as_str()
                        .map(|s| s.trim().to_owned())
                        .unwrap_or_else(|| {
                            model_name(&catalog, body["provider"].as_str().unwrap(), model)
                        });
                    if name.is_empty()
                        || catalog["models"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .any(|m| m["name"] == name)
                    {
                        return Err(ApiError::new(
                            409,
                            "model_name_conflict",
                            "Model configuration names must be nonempty and unique",
                        ));
                    }
                    let model_id = new_id();
                    if first.is_null() {
                        first = model_id.clone();
                    }
                    catalog["models"].as_array_mut().unwrap().push(json!({"model_id":model_id,"connection_id":connection_id,"model":model,"name":name}));
                }
                if body["set_default"] == true {
                    catalog["default_model_id"] = first;
                }
            }
            "rename_model" => {
                let id = &request.path_parameters["model_id"];
                let name = body["name"].as_str().unwrap().trim();
                if name.is_empty()
                    || catalog["models"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|m| m["model_id"] != *id && m["name"] == name)
                {
                    return Err(ApiError::new(
                        409,
                        "model_name_conflict",
                        "Model configuration name already exists or is empty",
                    ));
                }
                let model = catalog["models"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|m| m["model_id"] == *id)
                    .ok_or_else(|| {
                        ApiError::new(404, "model_not_found", "Model configuration not found")
                    })?;
                model["name"] = json!(name);
            }
            "set_default_model" => {
                if !body["model_id"].is_null() && resolve(&catalog, &body["model_id"]).is_null() {
                    return Err(ApiError::new(
                        404,
                        "model_not_found",
                        "Model configuration not found",
                    ));
                }
                catalog["default_model_id"] = body["model_id"].clone();
            }
            "update_model_connection" | "delete_model_connection" => {
                let id = json!(request.path_parameters["connection_id"]);
                let c = catalog["connections"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|c| c["connection_id"] == id)
                    .ok_or_else(|| {
                        ApiError::new(
                            404,
                            "model_connection_not_found",
                            "Model connection not found",
                        )
                    })?;
                if op == "update_model_connection" {
                    validate_connection(body)?;
                    // A provider change can invalidate all shared model identifiers; require re-import.
                    if c["provider"] != body["provider"] {
                        return Err(ApiError::invalid("Re-import models to change provider"));
                    }
                    c["base_url"] = body["base_url"].clone();
                    c["api_key"] = body["api_key"].clone();
                } else {
                    remove_connection(&mut catalog, &id);
                }
            }
            "set_model_config" | "clear_model_config" => {
                if op == "set_model_config" {
                    validate_connection(body)?;
                }
                let ids: Vec<_> = catalog["connections"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|c| c["legacy"] == true)
                    .map(|c| c["connection_id"].clone())
                    .collect();
                for id in ids {
                    remove_connection(&mut catalog, &id);
                }
                catalog["default_model_id"] = Value::Null;
                if op == "set_model_config" {
                    add_legacy(&mut catalog, body);
                }
            }
            _ => {
                return Err(ApiError::new(
                    404,
                    "operation_not_found",
                    "Unknown model operation",
                ));
            }
        }
        if matches!(op, "import_models" | "receive_model_sync") && first_model {
            // Created atomically with the first model, independent of any GUI lifetime.
            let session_id = new_id();
            let now = rove_core::now();
            let session = json!({"session_id":session_id,"device_id":self.store.device_id,"title":"初始网络的会话","kind":"network_onboarding","created_at":now,"updated_at":now,"archived_at":null});
            tx.execute(
                "INSERT INTO sessions(id,record) VALUES(?1,?2)",
                params![session_id.as_str().unwrap(), session.to_string()],
            )
            .map_err(|e| storage_error(e.into()))?;
            catalog["onboarding_session_id"] = session_id;
        }
        if matches!(
            op,
            "import_models" | "set_model_config" | "receive_model_sync"
        ) {
            write(&tx, "network_onboarding_seen", &json!(true)).map_err(storage_error)?;
        }
        write(
            &tx,
            "model_config",
            &resolve(&catalog, &catalog["default_model_id"]),
        )
        .map_err(storage_error)?;
        write(&tx, "model_catalog", &catalog).map_err(storage_error)?;
        tx.commit().map_err(|e| storage_error(e.into()))?;
        drop(db);
        if op == "clear_model_config" {
            return Ok((204, None));
        }
        if op == "set_model_config" {
            return Ok((200, Some(self.store.model_state().map_err(storage_error)?)));
        }
        Ok((200, Some(public(&catalog, &tests))))
    }
}
