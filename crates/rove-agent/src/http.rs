//! Remote transport. The router is deliberately private: production callers must
//! supply an adapter-owned TUN interface, never a wildcard/physical listener.
use crate::Agent;
use axum::{
    Router,
    body::{Body, to_bytes},
    extract::{MatchedPath, Path, State},
    http::{HeaderMap, Method, StatusCode, Uri},
    response::{IntoResponse, Response, Sse, sse::Event},
    routing::{MethodFilter, on},
};
use futures_util::stream;
use rove_protocol::{ApiError, Request, contract::AGENT};
use serde_json::Value;
use std::{
    collections::{BTreeMap, VecDeque},
    sync::Arc,
    time::Duration,
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Clone)]
struct Context {
    agent: Arc<Agent>,
    network_id: Uuid,
    stopping: CancellationToken,
}

/// Serve a network adapter's assigned IPv4 on its TUN interface. The caller must
/// cancel this instance BEFORE removing/reconfiguring that network interface.
/// No portable or unprivileged fallback is allowed when interface binding fails.
#[cfg(target_os = "linux")]
pub async fn serve_overlay(
    agent: Arc<Agent>,
    network_id: Uuid,
    interface: &str,
    address: std::net::SocketAddrV4,
    stopping: CancellationToken,
) -> anyhow::Result<()> {
    let listener = bind_overlay(interface, address)?;
    serve_bound(agent, network_id, listener, stopping).await
}

pub(crate) async fn serve_bound(
    agent: Arc<Agent>,
    network_id: Uuid,
    listener: tokio::net::TcpListener,
    stopping: CancellationToken,
) -> anyhow::Result<()> {
    let context = Context {
        agent,
        network_id,
        stopping: stopping.clone(),
    };
    axum::serve(
        crate::http_listener::BoundedListener::new(listener, 128),
        router(context),
    )
    .with_graceful_shutdown(stopping.cancelled_owned())
    .await?;
    Ok(())
}

#[cfg(target_os = "linux")]
pub(crate) fn bind_overlay(
    interface: &str,
    address: std::net::SocketAddrV4,
) -> anyhow::Result<tokio::net::TcpListener> {
    validate_overlay(interface, *address.ip())?;
    let socket = tokio::net::TcpSocket::new_v4()?;
    socket.bind_device(Some(interface.as_bytes()))?;
    socket.bind(address.into())?;
    Ok(socket.listen(128)?)
}

#[cfg(target_os = "linux")]
pub(crate) fn validate_overlay(
    interface: &str,
    address: std::net::Ipv4Addr,
) -> anyhow::Result<u32> {
    anyhow::ensure!(
        !interface.is_empty()
            && interface.len() < libc::IFNAMSIZ
            && interface
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_-.".contains(&b))
            && interface != "."
            && interface != "..",
        "Invalid overlay interface"
    );
    anyhow::ensure!(
        !address.is_unspecified()
            && !address.is_loopback()
            && !address.is_multicast()
            && !address.is_broadcast(),
        "Invalid overlay address"
    );
    let flags = std::fs::read_to_string(format!("/sys/class/net/{interface}/tun_flags"))?;
    let flags = u32::from_str_radix(flags.trim().trim_start_matches("0x"), 16)?;
    anyhow::ensure!(flags & 3 == 1, "Overlay listener requires a TUN interface");
    anyhow::ensure!(
        interface_has_address(interface, address)?,
        "Address is not assigned to the overlay interface"
    );
    check_main_routes(
        &std::fs::read_to_string("/proc/net/route")?,
        interface,
        address,
    )?;
    let name = std::ffi::CString::new(interface)?;
    // SAFETY: name is a live NUL-terminated interface name.
    let index = unsafe { libc::if_nametoindex(name.as_ptr()) };
    anyhow::ensure!(index != 0, "Overlay interface disappeared");
    Ok(index)
}

#[cfg(target_os = "linux")]
fn check_main_routes(
    routes: &str,
    interface: &str,
    address: std::net::Ipv4Addr,
) -> std::io::Result<()> {
    for line in routes.lines().skip(1) {
        let fields: Vec<_> = line.split_whitespace().collect();
        if fields.len() < 8 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid kernel route table",
            ));
        }
        let parse = |value: &str| {
            u32::from_str_radix(value, 16).map_err(|_| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Invalid kernel route field",
                )
            })
        };
        let destination = u32::from(std::net::Ipv4Addr::from(parse(fields[1])?.to_ne_bytes()));
        let mask = u32::from(std::net::Ipv4Addr::from(parse(fields[7])?.to_ne_bytes()));
        let flags = parse(fields[3])?;
        // Default routes are expected for Internet access, not overlapping
        // specific prefixes. Every non-overlay, non-loopback specific route
        // covering our own address is a conflict, including gateway routes.
        if fields[0] != interface
            && fields[0] != "lo"
            && flags & 1 != 0
            && mask != 0
            && destination & mask == u32::from(address) & mask
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AddrInUse,
                "Overlay address overlaps another interface route",
            ));
        }
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn interface_has_address(interface: &str, address: std::net::Ipv4Addr) -> std::io::Result<bool> {
    use std::{ffi::CStr, ptr};
    let mut head = ptr::null_mut();
    // SAFETY: getifaddrs initializes the pointer; it is freed once after traversal.
    if unsafe { libc::getifaddrs(&mut head) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    struct Addresses(*mut libc::ifaddrs);
    impl Drop for Addresses {
        fn drop(&mut self) {
            // SAFETY: this is the unmodified list returned by getifaddrs.
            unsafe { libc::freeifaddrs(self.0) };
        }
    }
    let addresses = Addresses(head);
    let mut current = addresses.0;
    let mut found = false;
    while !current.is_null() {
        // SAFETY: the linked list and sockaddr pointers remain alive until drop.
        let item = unsafe { &*current };
        if !item.ifa_name.is_null()
            && !item.ifa_addr.is_null()
            && unsafe { (*item.ifa_addr).sa_family } as i32 == libc::AF_INET
        {
            let ip = unsafe { &*item.ifa_addr.cast::<libc::sockaddr_in>() };
            let assigned = std::net::Ipv4Addr::from(ip.sin_addr.s_addr.to_ne_bytes());
            if unsafe { CStr::from_ptr(item.ifa_name) }.to_bytes() == interface.as_bytes() {
                found |= assigned == address;
            } else if item.ifa_flags & libc::IFF_LOOPBACK as u32 == 0 && !item.ifa_netmask.is_null()
            {
                // SAFETY: IPv4 getifaddrs entries supply an IPv4 netmask.
                let mask = unsafe { &*item.ifa_netmask.cast::<libc::sockaddr_in>() };
                let mask = u32::from_be(mask.sin_addr.s_addr);
                if (u32::from(assigned) & mask) == (u32::from(address) & mask) {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::AddrInUse,
                        "Overlay address overlaps another interface subnet",
                    ));
                }
            }
        }
        current = item.ifa_next;
    }
    Ok(found)
}

fn router(context: Context) -> Router {
    let mut router = Router::new();
    for operation in AGENT.operations.values() {
        let method = Method::from_bytes(operation.method.as_bytes()).unwrap();
        router = router.route(
            &operation.path,
            on(MethodFilter::try_from(method).unwrap(), dispatch),
        );
    }
    router
        .fallback(|| async {
            error(ApiError::new(
                404,
                "operation_not_found",
                "Unknown endpoint",
            ))
        })
        .method_not_allowed_fallback(|| async {
            error(ApiError::new(
                405,
                "method_not_allowed",
                "Unsupported HTTP method",
            ))
        })
        .with_state(context)
}

fn error(value: ApiError) -> Response {
    json_response(value.status, Some(value.body()))
}

fn json_response(status: u16, body: Option<Value>) -> Response {
    let status = StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    let mut response = match body {
        Some(body) => (status, axum::Json(body)).into_response(),
        None => status.into_response(),
    };
    response
        .headers_mut()
        .insert("cache-control", "no-store".parse().unwrap());
    response
}

fn identity(
    headers: &HeaderMap,
    name: &str,
    expected: Uuid,
    required: bool,
) -> Result<(), ApiError> {
    let mut values = headers.get_all(name).iter();
    let Some(value) = values.next() else {
        return if required {
            Err(ApiError::invalid("Missing target context"))
        } else {
            Ok(())
        };
    };
    if values.next().is_some() {
        return Err(ApiError::invalid("Duplicate target context"));
    }
    let parsed = value
        .to_str()
        .ok()
        .and_then(|value| Uuid::parse_str(value).ok())
        .ok_or_else(|| ApiError::invalid("Invalid target context"))?;
    if parsed != expected {
        return Err(ApiError::new(
            409,
            "target_mismatch",
            "Request does not match this network/device",
        ));
    }
    Ok(())
}

async fn dispatch(
    State(context): State<Context>,
    path: MatchedPath,
    Path(parameters): Path<BTreeMap<String, String>>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Body,
) -> Response {
    match execute(
        context,
        path.as_str().to_owned(),
        parameters,
        method,
        uri,
        headers,
        body,
    )
    .await
    {
        Ok(response) => response,
        Err(value) => error(value),
    }
}

async fn execute(
    context: Context,
    path: String,
    parameters: BTreeMap<String, String>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Body,
) -> Result<Response, ApiError> {
    let (operation_id, _) = AGENT
        .operations
        .iter()
        .find(|(_, operation)| operation.path == path && operation.method == method.as_str())
        .ok_or_else(|| ApiError::new(404, "operation_not_found", "Unknown endpoint"))?;
    identity(&headers, "x-rove-network-id", context.network_id, true)?;
    identity(
        &headers,
        "x-rove-target-device-id",
        context.agent.store.device_id.0,
        operation_id != "get_hello",
    )?;
    if context.stopping.is_cancelled() {
        return Err(ApiError::new(
            503,
            "network_unavailable",
            "Network is stopping",
        ));
    }
    let mut request = Request::new(operation_id);
    request.path_parameters = parameters;
    for (key, value) in url::form_urlencoded::parse(uri.query().unwrap_or("").as_bytes()) {
        let parsed = if matches!(key.as_ref(), "limit" | "after_seq") {
            Value::from(
                value
                    .parse::<u64>()
                    .map_err(|_| ApiError::invalid("Invalid numeric query parameter"))?,
            )
        } else {
            Value::String(value.into_owned())
        };
        if request
            .query_parameters
            .insert(key.into_owned(), parsed)
            .is_some()
        {
            return Err(ApiError::invalid("Duplicate query parameter"));
        }
    }
    let bytes = tokio::time::timeout(Duration::from_secs(5), to_bytes(body, 8 * 1024 * 1024))
        .await
        .map_err(|_| ApiError::new(408, "request_timeout", "Request body deadline exceeded"))?
        .map_err(|_| ApiError::invalid("Request body exceeds limit or could not be read"))?;
    if !bytes.is_empty() {
        if !headers
            .get("content-type")
            .and_then(|h| h.to_str().ok())
            .is_some_and(|h| {
                h.split(';')
                    .next()
                    .unwrap_or("")
                    .trim()
                    .eq_ignore_ascii_case("application/json")
            })
        {
            return Err(ApiError::new(
                415,
                "unsupported_media_type",
                "Expected application/json",
            ));
        }
        request.body = Some(
            serde_json::from_slice(&bytes).map_err(|_| ApiError::invalid("Invalid JSON body"))?,
        );
    }
    if operation_id == "subscribe_run_events" {
        if let Some(value) = headers.get("last-event-id") {
            if headers.get_all("last-event-id").iter().count() != 1 {
                return Err(ApiError::invalid("Duplicate Last-Event-ID"));
            }
            let value = value
                .to_str()
                .map_err(|_| ApiError::invalid("Invalid Last-Event-ID"))?;
            if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
                return Err(ApiError::invalid("Invalid Last-Event-ID"));
            }
            let after = Value::from(
                value
                    .parse::<u64>()
                    .map_err(|_| ApiError::invalid("Invalid Last-Event-ID"))?,
            );
            if request
                .query_parameters
                .get("after_seq")
                .is_some_and(|v| *v != after)
            {
                return Err(ApiError::invalid("Conflicting event cursors"));
            }
            request.query_parameters.insert("after_seq".into(), after);
        }
        AGENT.request(&request)?;
        return events(context, &request);
    }
    let mut response = context.agent.handle(&request).await;
    if operation_id == "get_hello" && response.status_code == 200 {
        response.body.as_mut().unwrap()["network_id"] =
            Value::String(context.network_id.to_string());
    }
    Ok(json_response(response.status_code, response.body))
}

fn events(context: Context, request: &Request) -> Result<Response, ApiError> {
    let run_id = request.path_parameters["run_id"].clone();
    let after = request
        .query_parameters
        .get("after_seq")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let (initial, done) = context.agent.events_after(&run_id, after)?;
    if initial.is_empty() && done {
        return Ok(json_response(204, None));
    }
    let state = (context, run_id, after, VecDeque::from(initial), done);
    let stream = stream::unfold(
        state,
        |(context, run_id, mut after, mut pending, mut done)| async move {
            loop {
                if context.stopping.is_cancelled() {
                    return None;
                }
                if let Some(value) = pending.pop_front() {
                    after = value["seq"].as_i64().unwrap();
                    let event = Event::default()
                        .id(after.to_string())
                        .event("run_event")
                        .data(value.to_string());
                    return Some((
                        Ok::<_, std::io::Error>(event),
                        (context, run_id, after, pending, done),
                    ));
                }
                if done {
                    return None;
                }
                tokio::select! {
                    _ = context.stopping.cancelled() => return None,
                    _ = tokio::time::sleep(Duration::from_millis(100)) => {}
                }
                match context.agent.events_after(&run_id, after) {
                    Ok((values, terminal)) => {
                        pending = values.into();
                        done = terminal;
                    }
                    // Headers are already sent. Closing forces clients to resnapshot;
                    // never invent an out-of-contract run event or cancel the AI run.
                    Err(_) => return None,
                }
            }
        },
    );
    let mut response = Sse::new(stream).into_response();
    response
        .headers_mut()
        .insert("cache-control", "no-cache".parse().unwrap());
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn main_routes_reject_specific_physical_overlap_but_allow_default_and_own_tun() {
        let encode = |ip: std::net::Ipv4Addr| format!("{:08X}", u32::from_ne_bytes(ip.octets()));
        let ip = std::net::Ipv4Addr::new(10, 126, 126, 7);
        let header = "Iface Destination Gateway Flags RefCnt Use Metric Mask\n";
        let route = format!(
            "eth0 {} 00000000 0001 0 0 0 {}\n",
            encode(ip),
            encode(std::net::Ipv4Addr::BROADCAST)
        );
        assert!(check_main_routes(&(header.to_owned() + &route), "rove123", ip).is_err());
        assert!(
            check_main_routes(
                &(header.to_owned() + &route.replace("eth0", "rove123")),
                "rove123",
                ip
            )
            .is_ok()
        );
        assert!(
            check_main_routes(
                &(header.to_owned() + "eth0 00000000 00000000 0003 0 0 0 00000000\n"),
                "rove123",
                ip
            )
            .is_ok()
        );
        assert!(check_main_routes(&(header.to_owned() + "malformed\n"), "rove123", ip).is_err());
    }

    // This listener tests HTTP serialization only, not overlay isolation. It is
    // intentionally constructed inside the private test module, never exported.
    async fn fixture() -> (
        tempfile::TempDir,
        Context,
        String,
        tokio::task::JoinHandle<()>,
    ) {
        let directory = tempfile::tempdir().unwrap();
        let context = Context {
            agent: Agent::open(&directory.path().join("agent")).unwrap(),
            network_id: Uuid::new_v4(),
            stopping: CancellationToken::new(),
        };
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let app = router(context.clone());
        let stopping = context.stopping.clone();
        let task = tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(stopping.cancelled_owned())
                .await
                .unwrap();
        });
        (directory, context, base, task)
    }

    fn client(context: &Context) -> reqwest::Client {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-rove-network-id",
            context.network_id.to_string().parse().unwrap(),
        );
        headers.insert(
            "x-rove-target-device-id",
            context.agent.store.device_id.to_string().parse().unwrap(),
        );
        reqwest::Client::builder()
            .no_proxy()
            .default_headers(headers)
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap()
    }

    #[tokio::test]
    async fn http_sdk_round_trip_and_context_rejection() {
        let (_directory, context, base, task) = fixture().await;
        let sdk = rove_sdk::RemoteClient::new(
            base.parse().unwrap(),
            context.network_id,
            context.agent.store.device_id.0,
        )
        .unwrap();
        let hello = sdk.call(Request::new("get_hello")).await.unwrap();
        assert_eq!(
            hello.body.unwrap()["network_id"],
            context.network_id.to_string()
        );
        let created = sdk
            .call(Request::new("create_session").with_body(json!({"title":"remote session"})))
            .await
            .unwrap();
        assert_eq!(created.status_code, 201);
        let session_id = created.body.unwrap()["session_id"]
            .as_str()
            .unwrap()
            .to_owned();
        let read = sdk
            .call(Request::new("get_session").with_path("session_id", &session_id))
            .await
            .unwrap();
        assert_eq!(read.body.unwrap()["title"], "remote session");
        let mut list = Request::new("list_sessions");
        list.query_parameters.insert("limit".into(), json!(1));
        assert_eq!(sdk.call(list).await.unwrap().status_code, 200);

        let client = client(&context);
        for header in ["x-rove-network-id", "x-rove-target-device-id"] {
            let response = client
                .post(format!("{base}/v1/sessions"))
                .header(header, Uuid::new_v4().to_string())
                .json(&json!({"title":"must not exist"}))
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), 409);
            assert_eq!(
                response.json::<Value>().await.unwrap()["error"]["code"],
                "target_mismatch"
            );
        }
        let listed = context
            .agent
            .handle(&Request::new("list_sessions"))
            .await
            .body
            .unwrap();
        assert_eq!(listed["items"].as_array().unwrap().len(), 1);
        let response = reqwest::Client::new()
            .get(format!("{base}/v1/device"))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 400);
        for query in ["limit=1&limit=2", "limit=-1", "unknown=1"] {
            let response = client
                .get(format!("{base}/v1/sessions?{query}"))
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), 400);
        }
        let response = client
            .post(format!("{base}/v1/sessions"))
            .json(&Value::Null)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 400);
        let response = client
            .post(format!("{base}/v1/sessions"))
            .body("{}")
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 415);
        let response = client
            .get(format!("{base}/v1/no-such-operation"))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 404);
        assert!(response.json::<Value>().await.unwrap()["error"].is_object());
        context.stopping.cancel();
        task.await.unwrap();
        context.agent.shutdown().await;
    }

    #[tokio::test]
    async fn sse_replays_terminal_events_and_validates_resume_cursor() {
        let (_directory, context, base, task) = fixture().await;
        // No external provider: connect to an unserved, test-owned loopback port.
        let unavailable = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let model_url = format!("http://{}/v1", unavailable.local_addr().unwrap());
        drop(unavailable);
        assert_eq!(context.agent.handle(&Request::new("set_model_config").with_body(json!({
            "provider":"openai_compatible","base_url":model_url,"model":"test","api_key":null
        }))).await.status_code, 200);
        let session = context
            .agent
            .handle(&Request::new("create_session").with_body(json!({})))
            .await
            .body
            .unwrap();
        let run = context
            .agent
            .handle(
                &Request::new("submit_run")
                    .with_path("session_id", session["session_id"].as_str().unwrap())
                    .with_body(json!({"request_id":Uuid::new_v4(),"message":"test"})),
            )
            .await
            .body
            .unwrap();
        let run_id = run["run_id"].as_str().unwrap();
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if context.agent.events_after(run_id, 0).unwrap().1 {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        let (events, _) = context.agent.events_after(run_id, 0).unwrap();
        let last = events.last().unwrap()["seq"].as_i64().unwrap();
        let sdk = rove_sdk::RemoteClient::new(
            base.parse().unwrap(),
            context.network_id,
            context.agent.store.device_id.0,
        )
        .unwrap();
        let mut subscription = sdk.subscribe(run_id.parse().unwrap(), 0).await.unwrap();
        let mut received = Vec::new();
        while let Some(event) = subscription.next().await.unwrap() {
            received.push(event);
        }
        assert_eq!(received, events);
        assert!(subscription.terminal);
        let mut resumed = sdk.resume(&subscription).await.unwrap();
        assert!(resumed.next().await.unwrap().is_none());
        let client = client(&context);
        let endpoint = format!("{base}/v1/runs/{run_id}/events");
        let response = client.get(&endpoint).send().await.unwrap();
        assert_eq!(response.status(), 200);
        assert_eq!(response.headers()["cache-control"], "no-cache");
        assert_eq!(response.headers()["content-type"], "text/event-stream");
        let text = response.text().await.unwrap();
        for event in &events {
            assert!(text.contains(&format!("id: {}\n", event["seq"])));
            assert!(text.contains(&format!("data: {event}\n")));
        }
        assert_eq!(text.matches("event: run_event\n").count(), events.len());
        let response = client
            .get(&endpoint)
            .header("last-event-id", last.to_string())
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 204);
        assert!(response.bytes().await.unwrap().is_empty());
        let response = client
            .get(format!("{endpoint}?after_seq=0"))
            .header("last-event-id", last.to_string())
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 400);
        let response = client
            .get(format!("{endpoint}?after_seq={}", last + 1))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 409);
        context
            .agent
            .store
            .connection
            .lock()
            .unwrap()
            .execute("DELETE FROM run_events WHERE run_id=?1 AND seq=1", [run_id])
            .unwrap();
        let response = client.get(&endpoint).send().await.unwrap();
        assert_eq!(response.status(), 410);
        context.stopping.cancel();
        task.await.unwrap();
        context.agent.shutdown().await;
    }

    #[tokio::test]
    async fn all_agent_operations_have_sdk_socket_and_http_wire_coverage() {
        let (_directory, context, base, task) = fixture().await;
        let local_shutdown = CancellationToken::new();
        let socket = rove_sdk::socket_path(&context.agent.store.data_dir);
        let serving = tokio::spawn(crate::serve(context.agent.clone(), local_shutdown.clone()));
        tokio::time::timeout(Duration::from_secs(5), async {
            while !socket.exists() {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        let local = rove_sdk::LocalClient::new(&socket);
        let remote = rove_sdk::RemoteClient::new(
            base.parse().unwrap(),
            context.network_id,
            context.agent.store.device_id.0,
        )
        .unwrap();
        let missing = Uuid::new_v4().to_string();
        let join = json!({"schema_version":1,"network_id":Uuid::new_v4(),"display_name":"wire fixture","easytier":{
            "network_name":"wire-fixture","network_secret":"test-only","bootstrap_peers":[],"dhcp":true}});
        let service = json!({"network_id":missing,"name":"wire service","target":{"host":"127.0.0.1","port":1},"protocol":"http"});
        let mut covered = std::collections::BTreeSet::new();
        for (id, operation) in &AGENT.operations {
            let mut request = Request::new(id);
            for name in ["network_id", "session_id", "run_id", "service_id"] {
                if operation.path.contains(&format!("{{{name}}}")) {
                    request.path_parameters.insert(name.into(), missing.clone());
                }
            }
            request.body = match id.as_str() {
                "update_settings" => Some(json!({"max_active_runs":2})),
                "set_model_config" => Some(
                    json!({"provider":"openai_compatible","base_url":"http://127.0.0.1:1/v1","model":"test","api_key":null}),
                ),
                "create_network" => Some(json!({"display_name":"wire test"})),
                "import_network" => Some(json!({"source":"manual","config":join})),
                "update_network" => {
                    Some(json!({"display_name":"wire test","easytier":join["easytier"]}))
                }
                "create_network_share" | "create_session" => Some(json!({})),
                "update_session" => Some(json!({"title":"wire test"})),
                "submit_run" => Some(json!({"request_id":Uuid::new_v4(),"message":"wire test"})),
                "publish_service" | "update_service" => Some(service.clone()),
                _ => None,
            };
            if id == "list_services" {
                request
                    .query_parameters
                    .insert("network_id".into(), json!(missing));
            }
            AGENT
                .request(&request)
                .unwrap_or_else(|error| panic!("{id}: {error}"));
            if id == "subscribe_run_events" {
                assert!(local.subscribe(missing.parse().unwrap(), 0).await.is_err());
                let error = remote
                    .subscribe(missing.parse().unwrap(), 0)
                    .await
                    .err()
                    .unwrap();
                assert_eq!(error.downcast_ref::<ApiError>().unwrap().status, 404);
            } else {
                let first = local
                    .call(request.clone())
                    .await
                    .unwrap_or_else(|error| panic!("socket {id}: {error}"));
                let second = remote
                    .call(request)
                    .await
                    .unwrap_or_else(|error| panic!("HTTP {id}: {error}"));
                AGENT.response(id, &first).unwrap();
                AGENT.response(id, &second).unwrap();
                assert_eq!(first.status_code, second.status_code, "{id}");
                if first.status_code >= 400 {
                    assert_eq!(
                        first.body.unwrap()["error"]["code"],
                        second.body.unwrap()["error"]["code"],
                        "{id}"
                    );
                }
            }
            covered.insert(id.clone());
        }
        assert_eq!(covered.len(), 33);
        // Expected errors deliberately exercise nonexistent resources without
        // external effects; happy-path SSE/sharing/service tests are separate.
        context.stopping.cancel();
        task.await.unwrap();
        local_shutdown.cancel();
        serving.await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn production_binding_rejects_loopback_and_non_tun_interfaces() {
        assert!(bind_overlay("lo", "127.0.0.1:0".parse().unwrap()).is_err());
        assert!(bind_overlay("lo", "10.1.2.3:0".parse().unwrap()).is_err());
        assert!(bind_overlay("../../lo", "10.1.2.3:0".parse().unwrap()).is_err());
        assert!(bind_overlay("missing-rove", "0.0.0.0:0".parse().unwrap()).is_err());
        assert!(interface_has_address("lo", std::net::Ipv4Addr::LOCALHOST).unwrap());
        assert!(!interface_has_address("lo", "10.1.2.3".parse().unwrap()).unwrap_or(false));
    }
}
