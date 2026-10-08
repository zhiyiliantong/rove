//! Bounded public-directory access and EasyTier-compatible endpoint discovery.
use crate::network_probe::normalize_connector;
use futures_util::{StreamExt, stream};
use rove_protocol::ApiError;
use serde::Deserialize;
use serde_json::{Value, json};
use std::{collections::BTreeSet, net::IpAddr, time::Duration};

const MAX_BODY: usize = 512 * 1024;
const MAX_NODES: usize = 200;
const MAX_CANDIDATES: usize = 64;
const MAX_DNS_RECORDS: usize = 16;
const MAX_DNS_BYTES: usize = 8 * 1024;
const MAX_HOPS: usize = 16;
const FINAL_PROTOCOLS: [&str; 7] = ["tcp", "udp", "ws", "wss", "wg", "quic", "faketcp"];

#[derive(Deserialize)]
struct DirectoryEnvelope {
    #[serde(default)]
    source: String,
    #[serde(default)]
    stale: bool,
    items: Vec<DirectoryNode>,
}

#[derive(Deserialize)]
struct DirectoryNode {
    id: String,
    name: String,
    uri: String,
    protocol: String,
    uri_kind: String,
    health_status: String,
    #[serde(default)]
    latency_ms: Option<f64>,
    #[serde(default)]
    monitored_at: Option<String>,
    #[serde(default)]
    uptime_24h: Option<f64>,
    relay_claim: String,
    #[serde(default)]
    metadata_verified: bool,
}

fn error(status: u16, code: &'static str, message: &'static str) -> ApiError {
    ApiError::new(status, code, message)
}

fn unavailable() -> ApiError {
    error(
        503,
        "public_nodes_unavailable",
        "The configured public-node discovery service is unavailable; manual peers remain available",
    )
}

async fn bounded_bytes(mut response: reqwest::Response) -> Result<Vec<u8>, ApiError> {
    if response
        .content_length()
        .is_some_and(|length| length > MAX_BODY as u64)
    {
        return Err(error(
            413,
            "discovery_response_too_large",
            "Discovery response exceeds the safe limit",
        ));
    }
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(discovery_http_error)? {
        if body.len() + chunk.len() > MAX_BODY {
            return Err(error(
                413,
                "discovery_response_too_large",
                "Discovery response exceeds the safe limit",
            ));
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

fn validate_directory_root_with_private_http(
    value: &str,
    allow_private_http: bool,
) -> Result<url::Url, ApiError> {
    let mut url = url::Url::parse(value).map_err(|_| {
        error(
            422,
            "invalid_discovery_service_url",
            "Invalid discovery service URL",
        )
    })?;
    let loopback_http = url.scheme() == "http"
        && match url.host() {
            Some(url::Host::Domain(host)) => host.eq_ignore_ascii_case("localhost"),
            Some(url::Host::Ipv4(ip)) => ip.is_loopback(),
            Some(url::Host::Ipv6(ip)) => ip.is_loopback(),
            None => false,
        };
    let private_http = allow_private_http
        && url.scheme() == "http"
        && matches!(url.host(), Some(url::Host::Ipv4(ip)) if ip.is_private());
    if url.host().is_none()
        || (url.scheme() != "https" && !loopback_http && !private_http)
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(error(
            422,
            "invalid_discovery_service_url",
            "Discovery service must use HTTPS (or an explicitly packaged development HTTP address) without credentials, query, or fragment",
        ));
    }
    if !url.path().ends_with('/') {
        url.set_path(&format!("{}/", url.path()));
    }
    Ok(url)
}

pub(crate) fn validate_directory_root(value: &str) -> Result<url::Url, ApiError> {
    validate_directory_root_with_private_http(value, false)
}

pub(crate) fn validate_packaged_directory_root(value: &str) -> Result<url::Url, ApiError> {
    validate_directory_root_with_private_http(value, true)
}

fn parse_directory(body: &[u8], configured_source: &str) -> Result<Value, ApiError> {
    let envelope: DirectoryEnvelope = serde_json::from_slice(body).map_err(|_| unavailable())?;
    if envelope.items.len() > MAX_NODES {
        return Err(error(
            413,
            "discovery_response_too_large",
            "Discovery service returned too many nodes",
        ));
    }
    let mut seen = BTreeSet::new();
    let items = envelope
        .items
        .into_iter()
        .filter(|node| node.health_status == "up" && node.relay_claim != "denied")
        .filter_map(|node| {
            let uri = normalize_bootstrap(&node.uri).ok()?.to_string();
            if !seen.insert(uri.clone()) {
                return None;
            }
            Some(json!({
                "id": node.id, "name": node.name, "address": uri,
                "protocol": node.protocol, "uri_kind": node.uri_kind,
                "health_status": node.health_status, "latency_ms": node.latency_ms,
                "monitored_at": node.monitored_at, "uptime_24h": node.uptime_24h,
                "relay_claim": node.relay_claim, "metadata_verified": node.metadata_verified,
            }))
        })
        .collect::<Vec<_>>();
    Ok(
        json!({"items":items,"source":configured_source,"upstream_source":envelope.source,"stale":envelope.stale}),
    )
}

pub(crate) async fn list_public_nodes(
    directory_root: Option<String>,
    allow_packaged_http: bool,
) -> Result<Value, ApiError> {
    let root = directory_root
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            error(
                409,
                "discovery_service_not_configured",
                "Configure a public-node discovery service in device settings first",
            )
        })?;
    let url = if allow_packaged_http {
        validate_packaged_directory_root(&root)
    } else {
        validate_directory_root(&root)
    }?
    .join("v1/nodes")
    .map_err(|_| unavailable())?;
    let response = directory_client(&url)?
        .get(url.clone())
        .send()
        .await
        .map_err(|_| unavailable())?;
    if !response.status().is_success() {
        return Err(unavailable());
    }
    parse_directory(&bounded_bytes(response).await?, url.as_str())
}

fn client() -> Result<reqwest::Client, ApiError> {
    client_builder().build().map_err(|_| unavailable())
}

fn client_builder() -> reqwest::ClientBuilder {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(5))
        .user_agent(concat!("rove-agent/", env!("CARGO_PKG_VERSION")))
}

fn direct_local_directory(url: &url::Url) -> bool {
    url.scheme() == "http"
        && matches!(url.host(), Some(url::Host::Ipv4(ip)) if ip.is_private() || ip.is_loopback())
}

fn directory_client(url: &url::Url) -> Result<reqwest::Client, ApiError> {
    if !direct_local_directory(url) {
        return client();
    }
    // This HTTP exception has already passed packaged/loopback validation.
    // It names a local development service, not an Internet proxy destination.
    // Windows system proxy bypass wildcards are not portable to reqwest's
    // proxy matching. Keep this decision shared rather than branching by OS.
    client_builder()
        .no_proxy()
        .build()
        .map_err(|_| unavailable())
}

fn clean_discovery_url(value: &str) -> Result<url::Url, ApiError> {
    let url = url::Url::parse(value).map_err(|_| {
        error(
            422,
            "invalid_discovery_url",
            "Invalid EasyTier discovery URL",
        )
    })?;
    let basic_invalid = url.host().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some();
    let dns_invalid = matches!(url.scheme(), "txt" | "srv")
        && (!matches!(url.host(), Some(url::Host::Domain(host)) if host.parse::<IpAddr>().is_err())
            || url.port().is_some()
            || !matches!(url.path(), "" | "/")
            || url.query().is_some());
    if basic_invalid || dns_invalid || !matches!(url.scheme(), "http" | "https" | "txt" | "srv") {
        return Err(error(
            422,
            "invalid_discovery_url",
            "Expected http(s):// URL or txt:// / srv:// DNS name without credentials or fragment",
        ));
    }
    Ok(url)
}

pub(crate) fn normalize_bootstrap(value: &str) -> Result<url::Url, ApiError> {
    normalize_connector(value).or_else(|_| clean_discovery_url(value))
}

fn normalized_candidates<'a>(values: impl IntoIterator<Item = &'a str>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    values
        .into_iter()
        .filter_map(|value| normalize_bootstrap(value.trim()).ok())
        .map(|value| value.to_string())
        .filter(|value| seen.insert(value.clone()))
        .take(MAX_CANDIDATES)
        .collect()
}

fn choose(candidates: &[String]) -> Result<String, ApiError> {
    if candidates.is_empty() {
        return Err(error(
            422,
            "discovery_no_candidates",
            "Discovery response contains no supported EasyTier endpoint",
        ));
    }
    Ok(candidates[usize::from(uuid::Uuid::new_v4().as_bytes()[0]) % candidates.len()].clone())
}

async fn resolve_http(url: url::Url, network_name: String) -> Result<Vec<String>, ApiError> {
    let network_name = reqwest::header::HeaderValue::from_str(&network_name).map_err(|_| {
        error(
            422,
            "invalid_discovery_url",
            "Network name cannot be used as an HTTP header",
        )
    })?;
    let response = client()?
        .get(url)
        .header("X-Network-Name", network_name)
        .send()
        .await
        .map_err(discovery_http_error)?;
    if response.status().is_redirection() {
        let location = response
            .headers()
            .get(reqwest::header::LOCATION)
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| {
                error(
                    422,
                    "discovery_no_candidates",
                    "HTTP redirect has no Location",
                )
            })?;
        let mut values = vec![location.to_owned()];
        if let Ok(location_url) = url::Url::parse(location) {
            values.extend(
                location_url
                    .query_pairs()
                    .map(|(_, value)| value.into_owned()),
            );
        }
        return Ok(normalized_candidates(values.iter().map(String::as_str)));
    }
    if !response.status().is_success() {
        return Err(ApiError::new(
            502,
            "discovery_http_status",
            &format!(
                "Discovery endpoint returned HTTP {}",
                response.status().as_u16()
            ),
        ));
    }
    let body = bounded_bytes(response).await?;
    let text = std::str::from_utf8(&body).map_err(|_| {
        error(
            422,
            "discovery_no_candidates",
            "HTTP discovery body is not UTF-8",
        )
    })?;
    Ok(normalized_candidates(text.lines()))
}

fn discovery_http_error(cause: reqwest::Error) -> ApiError {
    // Do not expose request URLs, query strings, headers or response bodies.
    if cause.is_timeout() {
        error(
            504,
            "discovery_http_timeout",
            "HTTP discovery request timed out",
        )
    } else if cause.is_connect() {
        error(
            503,
            "discovery_http_connect_failed",
            "HTTP discovery connection failed (DNS, TCP or TLS); certificate verification remains enabled",
        )
    } else {
        error(
            503,
            "discovery_http_failed",
            "HTTP discovery request failed",
        )
    }
}

fn resolver() -> Result<hickory_resolver::TokioResolver, ApiError> {
    use hickory_resolver::name_server::TokioConnectionProvider;
    let builder = hickory_resolver::Resolver::builder_tokio().unwrap_or_else(|_| {
        // Android has no desktop resolv.conf. Match the pinned EasyTier fallback
        // resolvers only when system configuration cannot be read. Queries carry
        // discovery names, never network secrets, and retain the outer deadline.
        hickory_resolver::Resolver::builder_with_config(
            fallback_dns_config(),
            TokioConnectionProvider::default(),
        )
    });
    Ok(build_discovery_resolver(builder))
}

fn build_discovery_resolver(
    mut builder: hickory_resolver::ResolverBuilder<
        hickory_resolver::name_server::TokioConnectionProvider,
    >,
) -> hickory_resolver::TokioResolver {
    // A 5-second per-attempt default never retries within our 3-second outer
    // deadline. Leave room for a second attempt without extending that bound.
    let options = builder.options_mut();
    options.timeout = Duration::from_millis(900);
    options.attempts = 2;
    builder.build()
}

fn fallback_dns_config() -> hickory_resolver::config::ResolverConfig {
    use hickory_resolver::{
        config::{NameServerConfig, ResolverConfig},
        proto::xfer::Protocol,
    };
    let mut config = ResolverConfig::new();
    for address in ["223.5.5.5:53", "180.184.1.1:53"] {
        config.add_name_server(NameServerConfig::new(
            address.parse().unwrap(),
            Protocol::Udp,
        ));
    }
    config
}

async fn resolve_txt(url: url::Url) -> Result<Vec<String>, ApiError> {
    let lookup = tokio::time::timeout(
        Duration::from_secs(3),
        resolver()?.txt_lookup(url.host_str().unwrap()),
    )
    .await
    .map_err(|_| error(504, "discovery_dns_timeout", "DNS TXT lookup timed out"))?
    .map_err(|_| error(503, "discovery_dns_failed", "DNS TXT lookup failed"))?;
    let records = lookup.iter().take(MAX_DNS_RECORDS + 1).collect::<Vec<_>>();
    if records.len() > MAX_DNS_RECORDS {
        return Err(error(
            413,
            "discovery_response_too_large",
            "DNS TXT lookup returned too many records",
        ));
    }
    let mut bytes = Vec::new();
    for record in records {
        for part in record.txt_data() {
            if bytes.len() + part.len() + 1 > MAX_DNS_BYTES {
                return Err(error(
                    413,
                    "discovery_response_too_large",
                    "DNS TXT response exceeds the safe limit",
                ));
            }
            bytes.extend_from_slice(part);
        }
        bytes.push(b' ');
    }
    let text = String::from_utf8(bytes).map_err(|_| {
        error(
            422,
            "discovery_no_candidates",
            "DNS TXT response is not UTF-8",
        )
    })?;
    Ok(normalized_candidates(text.split_ascii_whitespace()))
}

async fn resolve_srv(url: url::Url) -> Result<Vec<String>, ApiError> {
    let host = url.host_str().unwrap().to_owned();
    let dns = resolver()?;
    let mut pending = Vec::new();
    for protocol in FINAL_PROTOCOLS {
        let dns = dns.clone();
        let name = format!("_easytier._{protocol}.{host}");
        pending.push(async move { (protocol, dns.srv_lookup(name).await) });
    }
    let lookups = stream::iter(pending)
        .buffer_unordered(FINAL_PROTOCOLS.len())
        .collect::<Vec<_>>();
    let results = tokio::time::timeout(Duration::from_secs(3), lookups)
        .await
        .map_err(|_| error(504, "discovery_dns_timeout", "DNS SRV lookup timed out"))?;
    let mut values = Vec::new();
    for (protocol, result) in results {
        let Ok(records) = result else { continue };
        for record in records.iter() {
            if record.port() != 0 {
                values.push(format!(
                    "{protocol}://{}:{}",
                    record.target().to_utf8().trim_end_matches('.'),
                    record.port()
                ));
            }
            if values.len() >= MAX_CANDIDATES {
                break;
            }
        }
    }
    Ok(normalized_candidates(values.iter().map(String::as_str)))
}

pub(crate) async fn resolve(value: String, network_name: String) -> Result<Value, ApiError> {
    let source = clean_discovery_url(&value)?.to_string();
    let mut current = source.clone();
    let mut visited = BTreeSet::new();
    let mut chain = Vec::new();
    let mut candidates = Vec::new();
    for hop in 0..=MAX_HOPS {
        let url = normalize_bootstrap(&current)?;
        if !visited.insert(url.to_string()) {
            return Err(error(
                422,
                "discovery_cycle",
                "EasyTier discovery cycle detected",
            ));
        }
        chain.push(url.to_string());
        if let Ok(final_url) = normalize_connector(url.as_str()) {
            let protocol = final_url.scheme().to_owned();
            return Ok(
                json!({"source_url":source,"resolved_url":final_url.to_string(),"protocol":protocol,"candidates":candidates,"chain":chain}),
            );
        }
        if hop == MAX_HOPS {
            return Err(error(
                422,
                "discovery_too_deep",
                "EasyTier discovery exceeded 16 hops",
            ));
        }
        candidates = match url.scheme() {
            "http" | "https" => resolve_http(url, network_name.clone()).await?,
            "txt" => resolve_txt(url).await?,
            "srv" => resolve_srv(url).await?,
            _ => unreachable!(),
        };
        current = choose(&candidates)?;
    }
    unreachable!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_explicit_local_http_directories_bypass_system_proxy() {
        for value in [
            "http://10.1.2.237:8090/v1/nodes",
            "http://127.0.0.1:8090/v1/nodes",
        ] {
            assert!(direct_local_directory(&value.parse().unwrap()));
        }
        for value in [
            "https://directory.example/v1/nodes",
            "http://directory.example/v1/nodes",
            "http://8.8.8.8/v1/nodes",
            "https://10.1.2.237/v1/nodes",
        ] {
            assert!(!direct_local_directory(&value.parse().unwrap()));
        }
        assert!(validate_directory_root("http://10.1.2.237:8090").is_err());
        assert!(validate_packaged_directory_root("http://10.1.2.237:8090").is_ok());
    }

    #[tokio::test]
    async fn dropped_dns_packet_retries_within_outer_deadline() {
        use hickory_resolver::{
            config::{NameServerConfig, ResolverConfig},
            name_server::TokioConnectionProvider,
            proto::{
                op::{Message, MessageType},
                rr::{RData, Record, rdata::TXT},
                xfer::Protocol,
            },
        };
        let socket = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let mut config = ResolverConfig::new();
        config.add_name_server(NameServerConfig::new(
            socket.local_addr().unwrap(),
            Protocol::Udp,
        ));
        let server = tokio::spawn(async move {
            let mut buffer = [0u8; 4096];
            socket.recv_from(&mut buffer).await.unwrap(); // Drop the first query.
            let (length, peer) = socket.recv_from(&mut buffer).await.unwrap();
            let request = Message::from_vec(&buffer[..length]).unwrap();
            let query = request.queries()[0].clone();
            let mut response = Message::new();
            response
                .set_id(request.id())
                .set_message_type(MessageType::Response)
                .set_recursion_desired(true)
                .set_recursion_available(true)
                .add_query(query.clone())
                .add_answer(Record::from_rdata(
                    query.name().clone(),
                    60,
                    RData::TXT(TXT::new(vec!["tcp://node.example:11010".to_owned()])),
                ));
            socket
                .send_to(&response.to_vec().unwrap(), peer)
                .await
                .unwrap();
        });
        let dns = build_discovery_resolver(hickory_resolver::Resolver::builder_with_config(
            config,
            TokioConnectionProvider::default(),
        ));
        let result =
            tokio::time::timeout(Duration::from_secs(3), dns.txt_lookup("seed.example.")).await;
        server.abort();
        assert!(result.unwrap().is_ok());
    }

    #[tokio::test]
    async fn discovery_http_errors_identify_phase_without_leaking_urls() {
        use axum::{Router, http::StatusCode, routing::get};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let app = Router::new().route(
            "/failure",
            get(|| async { StatusCode::SERVICE_UNAVAILABLE }),
        );
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let failure = resolve(
            format!("http://{address}/failure?token=must-not-leak"),
            String::new(),
        )
        .await
        .unwrap_err();
        assert_eq!(failure.code, "discovery_http_status");
        assert!(failure.message.contains("503"));
        assert!(!failure.to_string().contains("must-not-leak"));
        server.abort();
        let stalled = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let cause = client()
            .unwrap()
            .get(format!(
                "http://{}/?token=must-not-leak",
                stalled.local_addr().unwrap()
            ))
            .timeout(Duration::from_millis(100))
            .send()
            .await
            .unwrap_err();
        let failure = discovery_http_error(cause);
        assert_eq!(failure.code, "discovery_http_timeout");
        assert!(!failure.to_string().contains("must-not-leak"));
    }

    #[tokio::test]
    async fn fallback_dns_does_not_require_system_configuration() {
        let config = fallback_dns_config();
        assert_eq!(config.name_servers().len(), 2);
        assert_eq!(
            config.name_servers()[0].socket_addr.to_string(),
            "223.5.5.5:53"
        );
        let resolver = hickory_resolver::Resolver::builder_with_config(
            config,
            hickory_resolver::name_server::TokioConnectionProvider::default(),
        )
        .build();
        drop(resolver); // No production DNS traffic in this test.
    }

    #[test]
    fn directory_filters_and_keeps_metadata_unverified() {
        let body = br#"{"source":"ruixuan","stale":true,"network_secret":"must-not-leak","items":[
          {"id":"1","name":"good","uri":"faketcp://node.example:11010","protocol":"faketcp","uri_kind":"connector","health_status":"up","relay_claim":"allowed","metadata_verified":false,"description_html":"<script>bad()</script>"},
          {"id":"2","name":"duplicate","uri":"faketcp://node.example:11010","protocol":"faketcp","uri_kind":"connector","health_status":"up","relay_claim":"unknown","metadata_verified":false},
          {"id":"3","name":"down","uri":"tcp://down.example:1","protocol":"tcp","uri_kind":"connector","health_status":"down","relay_claim":"allowed","metadata_verified":false}
        ]}"#;
        let result = parse_directory(body, "https://directory.example/v1/nodes").unwrap();
        assert_eq!(result["items"].as_array().unwrap().len(), 1);
        assert_eq!(result["items"][0]["metadata_verified"], false);
        assert_eq!(result["stale"], true);
        assert!(!result.to_string().contains("must-not-leak"));
        assert!(!result.to_string().contains("script"));
    }

    #[test]
    fn accepts_all_discovery_and_connector_protocols() {
        for value in [
            "https://directory.example/node",
            "http://directory.example/node",
            "txt://bootstrap.example.com",
            "srv://bootstrap.example.com",
            "tcp://host.example:11010",
            "udp://host.example:11010",
            "ws://host.example:11011",
            "wss://host.example:11012",
            "wg://host.example:11013",
            "quic://host.example:11014",
            "faketcp://host.example:11015",
        ] {
            assert!(normalize_bootstrap(value).is_ok(), "{value}");
        }
    }

    #[test]
    fn directory_setting_requires_https_or_loopback_http() {
        assert!(validate_directory_root("https://directory.example").is_ok());
        assert!(validate_directory_root("http://127.0.0.1:8090").is_ok());
        assert!(validate_directory_root("http://directory.example").is_err());
        assert!(validate_directory_root("https://user@directory.example").is_err());
    }

    #[tokio::test]
    async fn configured_directory_and_http_discovery_use_bounded_agent_requests() {
        use axum::{Json, Router, http::HeaderMap, routing::get};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let app = Router::new()
            .route(
                "/v1/nodes",
                get(|| async {
                    Json(json!({"source":"fixture","stale":false,"items":[{
                        "id":"fixture:1","name":"fixture","uri":"srv://seed.example",
                        "protocol":"srv","uri_kind":"discovery","health_status":"up",
                        "relay_claim":"unknown","metadata_verified":false
                    }]}))
                }),
            )
            .route(
                "/discover",
                get(|headers: HeaderMap| async move {
                    assert_eq!(headers["x-network-name"], "test-network");
                    assert!(!headers.contains_key("authorization"));
                    assert!(!headers.contains_key("x-network-secret"));
                    "faketcp://node.example:11010\n"
                }),
            );
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

        let root = format!("http://{address}");
        let nodes = list_public_nodes(Some(root), false).await.unwrap();
        assert_eq!(nodes["items"][0]["address"], "srv://seed.example");
        let result = resolve(
            format!("http://{address}/discover"),
            "test-network".to_owned(),
        )
        .await
        .unwrap();
        assert_eq!(result["resolved_url"], "faketcp://node.example:11010");
        assert_eq!(result["chain"].as_array().unwrap().len(), 2);
        server.abort();
    }

    #[tokio::test]
    async fn http_redirect_cycles_and_hop_limit_follow_easytier_semantics() {
        use axum::{
            Router,
            extract::{Path, State},
            http::{StatusCode, header},
            routing::get,
        };
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let base = format!("http://{address}");
        let app = Router::new()
            .route(
                "/redirect",
                get(|| async {
                    (
                        StatusCode::FOUND,
                        [(header::LOCATION, "udp://node.example:11010")],
                    )
                }),
            )
            .route(
                "/cycle",
                get(|State(base): State<String>| async move { format!("{base}/cycle") }),
            )
            .route("/large", get(|| async { vec![b'x'; MAX_BODY + 1] }))
            .route(
                "/hop/{number}",
                get(
                    |State(base): State<String>, Path(number): Path<usize>| async move {
                        if number >= 17 {
                            "tcp://node.example:11010".to_owned()
                        } else {
                            format!("{base}/hop/{}", number + 1)
                        }
                    },
                ),
            )
            .with_state(base.clone());
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

        let redirected = resolve(format!("{base}/redirect"), String::new())
            .await
            .unwrap();
        assert_eq!(redirected["resolved_url"], "udp://node.example:11010");
        assert!(
            resolve(format!("{base}/cycle"), String::new())
                .await
                .is_err()
        );
        assert!(
            resolve(format!("{base}/hop/0"), String::new())
                .await
                .is_err()
        );
        assert!(
            resolve(format!("{base}/large"), String::new())
                .await
                .is_err()
        );
        server.abort();
    }
}
