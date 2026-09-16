//! Transport clients only; never depends on the agent or its database.
mod qr;
mod remote_subscription;
mod subscription;
pub use qr::ShareQr;
pub use remote_subscription::RemoteSubscription;
use rove_protocol::{
    Request, Response, check_protocol,
    contract::AGENT,
    frame::{read_frame, write_frame},
};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::time::Duration;
pub use subscription::Subscription;

#[derive(Clone)]
pub struct LocalClient {
    endpoint: PathBuf,
}
impl LocalClient {
    pub fn new(endpoint: impl Into<PathBuf>) -> Self {
        Self {
            endpoint: endpoint.into(),
        }
    }
    pub fn endpoint(&self) -> &Path {
        &self.endpoint
    }
    pub async fn call(&self, request: Request) -> anyhow::Result<Response> {
        self.call_with_deadlines(request, Duration::from_secs(5), Duration::from_secs(30))
            .await
    }
    async fn call_with_deadlines(
        &self,
        request: Request,
        handshake_timeout: Duration,
        response_timeout: Duration,
    ) -> anyhow::Result<Response> {
        AGENT.request(&request)?;
        let mut stream = tokio::time::timeout(handshake_timeout, async {
            #[cfg(unix)]
            let mut stream = tokio::net::UnixStream::connect(&self.endpoint)
                .await
                .map_err(|_| {
                    anyhow::anyhow!(
                        "Cannot connect to rove-agent; start it with the same data directory"
                    )
                })?;
            #[cfg(windows)]
            let mut stream = open_pipe(&self.endpoint).await?;
            let hello_request = Request::new("get_hello");
            write_frame(&mut stream, &json!(hello_request)).await?;
            let hello: Response = serde_json::from_value(
                read_frame(&mut stream)
                    .await?
                    .ok_or_else(|| anyhow::anyhow!("Agent disconnected during handshake"))?,
            )?;
            AGENT.response("get_hello", &hello)?;
            anyhow::ensure!(
                hello.correlation_id == hello_request.correlation_id && hello.status_code == 200,
                "Invalid handshake response"
            );
            let body = hello.body.unwrap();
            check_protocol(
                body["protocol"]["min"].as_u64().unwrap(),
                body["protocol"]["max"].as_u64().unwrap(),
            )?;
            Ok::<_, anyhow::Error>(stream)
        })
        .await
        .map_err(|_| {
            anyhow::anyhow!("Local agent handshake timed out; business request was not sent")
        })??;
        let receive_response = async {
            write_frame(&mut stream, &json!(request)).await?;
            let response: Response =
                serde_json::from_value(read_frame(&mut stream).await?.ok_or_else(|| {
                    anyhow::anyhow!("Agent disconnected; accepted work is not cancelled")
                })?)?;
            anyhow::ensure!(
                response.correlation_id == request.correlation_id,
                "Unexpected response correlation_id"
            );
            AGENT.response(&request.operation_id, &response)?;
            Ok(response)
        };
        tokio::time::timeout(response_timeout, receive_response)
            .await
            .map_err(|_| {
                let retry = if request.operation_id == "submit_run" {
                    "; retry the original request_id"
                } else {
                    ""
                };
                anyhow::anyhow!("Local agent response timed out; outcome unknown. Accepted work was not cancelled{retry}")
            })?
    }
}

#[cfg(windows)]
async fn open_pipe(
    path: &Path,
) -> std::io::Result<tokio::net::windows::named_pipe::NamedPipeClient> {
    loop {
        match tokio::net::windows::named_pipe::ClientOptions::new().open(path) {
            Ok(stream) => return Ok(stream),
            // ERROR_PIPE_BUSY: another client won the available instance.
            // Both callers wrap this loop in their existing handshake deadline.
            Err(error) if error.raw_os_error() == Some(231) => {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            Err(error) => return Err(error),
        }
    }
}

#[cfg(all(test, unix))]
mod local_deadline_tests;

pub fn default_data_dir() -> PathBuf {
    if let Some(path) = std::env::var_os("ROVE_DATA_DIR") {
        return path.into();
    }
    #[cfg(windows)]
    let base = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
    #[cfg(not(windows))]
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".local/share")));
    base.unwrap_or_else(|| PathBuf::from(".")).join("rove")
}
pub fn socket_path(data_dir: &Path) -> PathBuf {
    #[cfg(unix)]
    {
        data_dir.join("agent.sock")
    }
    #[cfg(windows)]
    {
        use std::hash::{Hash, Hasher};
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        data_dir.hash(&mut hash);
        PathBuf::from(format!(r"\\.\pipe\rove-{:016x}", hash.finish()))
    }
}

// The remote transport takes an explicit, previously discovered overlay endpoint.
// It never falls back to local execution or follows redirects.
pub struct RemoteClient {
    client: reqwest::Client,
    base_url: reqwest::Url,
    network_id: uuid::Uuid,
    device_id: uuid::Uuid,
}
impl RemoteClient {
    /// Linux overlay transport pinned to the adapter-selected interface. Never
    /// use a caller's unvalidated network name as an interface selector.
    #[cfg(target_os = "linux")]
    pub fn on_interface(
        base_url: reqwest::Url,
        network_id: uuid::Uuid,
        device_id: uuid::Uuid,
        interface: &str,
    ) -> anyhow::Result<Self> {
        let mut remote = Self::new(base_url, network_id, device_id)?;
        anyhow::ensure!(!interface.is_empty(), "Missing overlay interface");
        remote.client = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .interface(interface)
            .connect_timeout(Duration::from_secs(5))
            .build()?;
        Ok(remote)
    }

    /// Discovery knows a route, not a stable device ID yet. Only hello is sent;
    /// callers check protocol compatibility before any device/business request.
    #[cfg(target_os = "linux")]
    pub async fn discover(
        base_url: reqwest::Url,
        network_id: uuid::Uuid,
        interface: &str,
    ) -> anyhow::Result<Value> {
        let remote = Self::on_interface(base_url, network_id, uuid::Uuid::nil(), interface)?;
        let response = remote
            .client
            .get(remote.base_url.join("/v1/hello")?)
            .header("X-Rove-Network-Id", network_id.to_string())
            .timeout(Duration::from_secs(2))
            .send()
            .await?
            .error_for_status()?;
        let hello: Value = serde_json::from_slice(&limited_body(response, 64 * 1024).await?)?;
        AGENT.validate("Hello", &hello)?;
        anyhow::ensure!(
            hello["network_id"] == network_id.to_string(),
            "target_mismatch"
        );
        Ok(hello)
    }
    pub fn new(
        base_url: reqwest::Url,
        network_id: uuid::Uuid,
        device_id: uuid::Uuid,
    ) -> anyhow::Result<Self> {
        anyhow::ensure!(
            base_url.scheme() == "http"
                && base_url.username().is_empty()
                && base_url.password().is_none()
                && base_url.host().is_some()
                && base_url.query().is_none()
                && base_url.fragment().is_none(),
            "Expected an overlay HTTP endpoint"
        );
        Ok(Self {
            client: reqwest::Client::builder()
                .no_proxy()
                .redirect(reqwest::redirect::Policy::none())
                .connect_timeout(std::time::Duration::from_secs(5))
                .build()?,
            base_url,
            network_id,
            device_id,
        })
    }
    pub async fn call(&self, request: Request) -> anyhow::Result<Response> {
        let op = AGENT.request(&request)?;
        anyhow::ensure!(
            request
                .target
                .as_ref()
                .is_none_or(|target| target.network_id == self.network_id
                    && target.device_id == self.device_id),
            "target_mismatch"
        );
        anyhow::ensure!(
            request.operation_id != "subscribe_run_events",
            "Remote SSE requires the event transport; JSON call cannot subscribe"
        );
        self.handshake().await?;
        let mut path = op.path.clone();
        for (name, value) in &request.path_parameters {
            path = path.replace(&format!("{{{name}}}"), value);
        }
        let query: Vec<_> = request
            .query_parameters
            .iter()
            .map(|(k, v)| {
                (
                    k,
                    v.as_str()
                        .map(str::to_owned)
                        .unwrap_or_else(|| v.to_string()),
                )
            })
            .collect();
        let mut call = self
            .client
            .request(op.method.parse()?, self.base_url.join(&path)?)
            .header("X-Rove-Network-Id", self.network_id.to_string())
            .header("X-Rove-Target-Device-Id", self.device_id.to_string())
            .timeout(std::time::Duration::from_secs(30))
            .query(&query);
        if let Some(body) = &request.body {
            call = call.json(body);
        }
        let result = call.send().await?;
        let status = result.status().as_u16();
        let body = if status == 204 {
            None
        } else {
            let bytes = limited_body(result, rove_protocol::frame::MAX_FRAME_BYTES).await?;
            Some(
                serde_json::from_slice(&bytes)
                    .map_err(|_| anyhow::anyhow!("Invalid remote JSON response"))?,
            )
        };
        let response = Response::new(&request, status, body);
        AGENT.response(&request.operation_id, &response)?;
        Ok(response)
    }
    async fn handshake(&self) -> anyhow::Result<()> {
        let response = self
            .client
            .get(self.base_url.join("/v1/hello")?)
            .header("X-Rove-Network-Id", self.network_id.to_string())
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await?
            .error_for_status()?;
        let hello: Value = serde_json::from_slice(&limited_body(response, 64 * 1024).await?)
            .map_err(|_| anyhow::anyhow!("Invalid remote handshake JSON"))?;
        AGENT.validate("Hello", &hello)?;
        anyhow::ensure!(
            hello["device_id"] == self.device_id.to_string()
                && hello["network_id"] == self.network_id.to_string(),
            "target_mismatch"
        );
        check_protocol(
            hello["protocol"]["min"].as_u64().unwrap(),
            hello["protocol"]["max"].as_u64().unwrap(),
        )?;
        Ok(())
    }
}

async fn limited_body(mut response: reqwest::Response, limit: usize) -> anyhow::Result<Vec<u8>> {
    anyhow::ensure!(
        response.content_length().is_none_or(|n| n <= limit as u64),
        "Remote response exceeds size limit"
    );
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        anyhow::ensure!(
            bytes.len().saturating_add(chunk.len()) <= limit,
            "Remote response exceeds size limit"
        );
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}
