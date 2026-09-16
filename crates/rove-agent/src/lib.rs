mod ai;
#[cfg(unix)]
mod embedded;
#[cfg(unix)]
pub use embedded::EmbeddedAgent;
#[cfg(target_os = "linux")]
pub mod http;
#[cfg(target_os = "linux")]
mod http_listener;
#[cfg(all(feature = "easytier", target_os = "linux"))]
mod network_runtime;
mod networks;
#[cfg(feature = "easytier")]
pub mod overlay;
mod services;
mod sessions;
pub mod sharing;
mod socket;
pub mod store;
pub mod tools;
#[cfg(windows)]
mod windows_pipe;
use rove_protocol::{ApiError, Request, Response, contract::AGENT};
use serde_json::{Value, json};
use std::{path::Path, sync::Arc};
use store::Store;
use tokio_util::sync::CancellationToken;

pub struct Agent {
    pub store: Store,
    started_at: String,
    runtime: sessions::Runtime,
    services: services::Runtime,
    network_lifecycle: tokio::sync::Mutex<()>,
    #[cfg(all(feature = "easytier", target_os = "linux"))]
    network_driver: std::sync::OnceLock<network_runtime::Driver>,
}
fn capabilities() -> Vec<&'static str> {
    let mut values = vec![
        "device_config",
        "network_config",
        "network_sharing",
        "ai_sessions",
        "service_directory",
    ];
    if tools::supports_system_exec(std::env::consts::OS) {
        values.push("system_exec");
    }
    values
}
impl Agent {
    pub fn open(path: &Path) -> anyhow::Result<Arc<Self>> {
        tokio::runtime::Handle::try_current()
            .map_err(|_| anyhow::anyhow!("Agent::open requires an active Tokio runtime"))?;
        let agent = Arc::new(Self {
            store: Store::open(path)?,
            started_at: rove_core::now(),
            runtime: sessions::Runtime::default(),
            services: services::Runtime::default(),
            network_lifecycle: tokio::sync::Mutex::new(()),
            #[cfg(all(feature = "easytier", target_os = "linux"))]
            network_driver: std::sync::OnceLock::new(),
        });
        agent.interrupt_old_runs()?;
        Self::start_scheduler(&agent);
        Ok(agent)
    }
    pub async fn shutdown(&self) {
        self.runtime.stopping.cancel();
        #[cfg(all(feature = "easytier", target_os = "linux"))]
        if let Some(driver) = self.network_driver.get() {
            let _guard = self.network_lifecycle.lock().await;
            driver.close(self).await;
        }
        for cancel in self.runtime.active.lock().unwrap().values() {
            cancel.cancel();
        }
        while !self.runtime.active.lock().unwrap().is_empty() {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        let _ = self.stop_services(None).await;
    }
    pub async fn handle(self: &Arc<Self>, request: &Request) -> Response {
        let result = self.dispatch(request).await;
        match result {
            Ok((status, body)) => {
                let response = Response::new(request, status, body);
                if AGENT.response(&request.operation_id, &response).is_err() {
                    return Response::error(
                        request,
                        ApiError::new(500, "internal_error", "Response contract violation"),
                    );
                }
                response
            }
            Err(error) => Response::error(request, error),
        }
    }
    async fn dispatch(
        self: &Arc<Self>,
        request: &Request,
    ) -> Result<(u16, Option<Value>), ApiError> {
        AGENT.request(request)?;
        #[cfg(all(feature = "easytier", target_os = "linux"))]
        let joined_self = request.target.as_ref().is_some_and(|target| {
            target.device_id == self.store.device_id.0
                && self.network_driver.get().is_some()
                && self
                    .network(&target.network_id.to_string())
                    .is_ok_and(|(record, _)| {
                        record["state"] == "running" && record["enabled"] == true
                    })
        });
        #[cfg(not(all(feature = "easytier", target_os = "linux")))]
        let joined_self = false;
        if let Some(target) = request.target.as_ref().filter(|_| !joined_self) {
            #[cfg(all(feature = "easytier", target_os = "linux"))]
            if target.device_id != self.store.device_id.0 && self.network_driver.get().is_some() {
                let response = self.target_client(target).await?.call(request.clone()).await
                    .map_err(|_| ApiError::new(503, "target_unreachable", "Remote outcome is unknown; no local execution or automatic resubmission occurred"))?;
                return Ok((response.status_code, response.body));
            }
            if target.device_id != self.store.device_id.0 {
                return Err(ApiError::new(
                    503,
                    "target_unreachable",
                    "No discovered overlay route to the target; request was not executed locally",
                ));
            }
            return Err(ApiError::new(
                409,
                "network_unavailable",
                "Target network is not running",
            ));
        }
        let body = request.body.as_ref().unwrap_or(&Value::Null);
        let value = match request.operation_id.as_str() {
            "get_hello" => {
                json!({"device_id":self.store.device_id,"display_name":"Rove device","agent_version":env!("CARGO_PKG_VERSION"),"protocol":{"min":1,"max":1},"capabilities":capabilities(),"network_id":null})
            }
            "get_device" => {
                json!({"device_id":self.store.device_id,"display_name":"Rove device","agent_version":env!("CARGO_PKG_VERSION"),"os":std::env::consts::OS,"arch":std::env::consts::ARCH,"capabilities":capabilities(),"started_at":self.started_at})
            }
            "get_settings" => self.store.settings().map_err(storage_error)?,
            "update_settings" => {
                let mut settings = self.store.settings().map_err(storage_error)?;
                for (key, value) in body.as_object().unwrap() {
                    if key == "max_active_runs"
                        && value
                            .as_u64()
                            .and_then(|v| usize::try_from(v).ok())
                            .is_none()
                    {
                        return Err(ApiError::invalid(
                            "max_active_runs exceeds this platform's integer range",
                        ));
                    }
                    if key == "config_server_url" && !value.is_null() {
                        validate_server_url(value.as_str().unwrap())?;
                    }
                    settings[key] = value.clone();
                }
                self.store
                    .set("settings", &settings)
                    .map_err(storage_error)?;
                self.runtime.wake.notify_one();
                settings
            }
            "get_model_config" => self.store.model_state().map_err(storage_error)?,
            "set_model_config" => {
                let url = url::Url::parse(body["base_url"].as_str().unwrap())
                    .map_err(|_| ApiError::invalid("Invalid model base URL"))?;
                if !matches!(url.scheme(), "http" | "https")
                    || !url.username().is_empty()
                    || url.password().is_some()
                    || url.fragment().is_some()
                    || url.query().is_some()
                {
                    return Err(ApiError::invalid(
                        "Model URL must be HTTP(S), without embedded credentials, query or fragment",
                    ));
                }
                self.store
                    .set("model_config", body)
                    .map_err(storage_error)?;
                self.store.model_state().map_err(storage_error)?
            }
            "clear_model_config" => {
                self.store
                    .set("model_config", &Value::Null)
                    .map_err(storage_error)?;
                return Ok((204, None));
            }
            op if op.contains("network") => return self.network_operation(request).await,
            op if op.contains("service") => return self.service_operation(request).await,
            op if op.contains("session") || op.contains("run") || op == "list_messages" => {
                return self.session_operation(request);
            }
            _ => {
                return Err(ApiError::new(
                    501,
                    "unsupported",
                    "This operation has not been implemented in this development build",
                ));
            }
        };
        Ok((200, Some(value)))
    }
}
impl Drop for Agent {
    fn drop(&mut self) {
        self.runtime.stopping.cancel();
    }
}
pub(crate) fn storage_error(_: anyhow::Error) -> ApiError {
    ApiError::new(
        500,
        "storage_error",
        "Could not read or persist device state",
    )
}
pub(crate) fn page(request: &Request, items: Vec<Value>, id_key: &str) -> Result<Value, ApiError> {
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD as B64};
    let mut filter = request.query_parameters.clone();
    filter.remove("cursor");
    filter.remove("limit");
    let scope = json!({"op":request.operation_id,"path":request.path_parameters,"filter":filter});
    let after = if let Some(cursor) = request.query_parameters.get("cursor") {
        let parsed: Value = B64
            .decode(cursor.as_str().unwrap())
            .ok()
            .and_then(|v| serde_json::from_slice(&v).ok())
            .ok_or_else(|| ApiError::new(400, "invalid_cursor", "Invalid list cursor"))?;
        if parsed["scope"] != scope || !parsed["after"].is_string() {
            return Err(ApiError::new(
                400,
                "invalid_cursor",
                "Cursor does not match this query",
            ));
        }
        parsed["after"].as_str().unwrap().to_string()
    } else {
        String::new()
    };
    let limit = request
        .query_parameters
        .get("limit")
        .and_then(Value::as_u64)
        .unwrap_or(50) as usize;
    let key = |v: &Value| {
        if matches!(id_key, "session_id" | "run_id") {
            format!(
                "{}|{}",
                v["created_at"].as_str().unwrap_or(""),
                v[id_key].as_str().unwrap_or("")
            )
        } else {
            v[id_key].as_str().unwrap_or("").to_string()
        }
    };
    let mut matching = items.into_iter().filter(|v| key(v) > after).peekable();
    let mut selected = Vec::new();
    let mut size = 0;
    while let Some(item) = matching.peek() {
        let bytes = serde_json::to_vec(item).unwrap().len();
        if bytes > 6 * 1024 * 1024 {
            return Err(ApiError::new(
                413,
                "payload_too_large",
                "An item exceeds the page size limit",
            ));
        }
        if selected.len() >= limit || size + bytes > 6 * 1024 * 1024 {
            break;
        }
        size += bytes;
        selected.push(matching.next().unwrap());
    }
    let next = if matching.peek().is_some() {
        Some(B64.encode(json!({"scope":scope,"after":key(selected.last().unwrap())}).to_string()))
    } else {
        None
    };
    Ok(json!({"items":selected,"next_cursor":next}))
}
pub fn validate_server_url(value: &str) -> Result<url::Url, ApiError> {
    let url = url::Url::parse(value)
        .map_err(|_| ApiError::invalid("Invalid configuration server URL"))?;
    let loopback = url.host_str().is_some_and(|h| {
        h == "localhost" || h.parse::<std::net::IpAddr>().is_ok_and(|i| i.is_loopback())
    });
    if !(url.scheme() == "https" || (url.scheme() == "http" && loopback))
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(ApiError::invalid(
            "Configuration URL requires HTTPS (loopback HTTP is allowed for development) and no credentials, query or fragment",
        ));
    }
    Ok(url)
}

#[cfg(unix)]
pub async fn serve(agent: Arc<Agent>, shutdown: CancellationToken) -> anyhow::Result<()> {
    use std::os::unix::fs::{FileTypeExt, PermissionsExt};
    let path = rove_sdk::socket_path(&agent.store.data_dir);
    if let Ok(metadata) = std::fs::symlink_metadata(&path) {
        anyhow::ensure!(
            metadata.file_type().is_socket(),
            "Socket path is occupied by a non-socket file"
        );
        // We hold the data lock, so this can only be a stale socket.
        std::fs::remove_file(&path)?;
    }
    let listener = tokio::net::UnixListener::bind(&path)?;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    let mut clients = tokio::task::JoinSet::new();
    loop {
        tokio::select! {
            _ = shutdown.cancelled() => break,
            result = listener.accept() => {
                let (stream, _) = result?;
                let agent = agent.clone();
                clients.spawn(async move { let _ = socket::serve_connection(agent, stream).await; });
            },
            _ = clients.join_next(), if !clients.is_empty() => {},
        }
    }
    clients.abort_all();
    while clients.join_next().await.is_some() {}
    agent.shutdown().await;
    std::fs::remove_file(path)?;
    Ok(())
}
#[cfg(windows)]
pub async fn serve(agent: Arc<Agent>, shutdown: CancellationToken) -> anyhow::Result<()> {
    windows_pipe::serve(agent, shutdown).await
}
#[cfg(not(any(unix, windows)))]
pub async fn serve(_agent: Arc<Agent>, _shutdown: CancellationToken) -> anyhow::Result<()> {
    anyhow::bail!("Local transport is unsupported on this platform; no TCP fallback is opened")
}
