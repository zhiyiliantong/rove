//! Bounded transport reachability only: no credentials or protocol payloads.
use rove_protocol::ApiError;
use serde_json::{Value, json};
use std::time::{Duration, Instant};
static PROBES: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(4);

pub(crate) async fn probe(endpoint: &str) -> Result<Value, ApiError> {
    let url =
        url::Url::parse(endpoint).map_err(|_| ApiError::invalid("Invalid bootstrap endpoint"))?;
    if url.host().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
        || url.query().is_some()
        || !matches!(url.scheme(), "tcp" | "udp" | "ws" | "wss" | "quic" | "wg")
    {
        return Err(ApiError::invalid("Invalid bootstrap endpoint"));
    }
    let result = |status: &str, latency: Option<u64>, message: &str| {
        json!({
            "endpoint": endpoint, "status": status, "latency_ms": latency, "message": message
        })
    };
    if !matches!(url.scheme(), "tcp" | "ws" | "wss") {
        return Ok(result(
            "unsupported",
            None,
            "Datagram reachability requires an EasyTier handshake; no success inferred",
        ));
    }
    let port = url
        .port_or_known_default()
        .filter(|p| *p != 0)
        .ok_or_else(|| ApiError::invalid("An explicit nonzero TCP port is required"))?;
    let _permit = PROBES
        .try_acquire()
        .map_err(|_| ApiError::new(429, "probe_busy", "Too many concurrent node tests"))?;
    let host = match url.host().unwrap() {
        url::Host::Domain(host) => host.to_owned(),
        url::Host::Ipv4(ip) => ip.to_string(),
        url::Host::Ipv6(ip) => ip.to_string(),
    };
    let start = Instant::now();
    let connected = tokio::time::timeout(
        Duration::from_secs(3),
        tokio::net::TcpStream::connect((host.as_str(), port)),
    )
    .await;
    Ok(if matches!(connected, Ok(Ok(_))) {
        result(
            "reachable",
            Some(start.elapsed().as_millis() as u64),
            "TCP connection succeeded; EasyTier identity, credentials and TLS were not verified",
        )
    } else {
        result(
            "unreachable",
            None,
            "TCP connection failed or timed out; network configuration is unchanged",
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn tests_actual_tcp_without_sending_credentials_or_claiming_udp_success() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("tcp://{}", listener.local_addr().unwrap());
        assert_eq!(probe(&endpoint).await.unwrap()["status"], "reachable");
        let (mut stream, _) = listener.accept().await.unwrap();
        use tokio::io::AsyncReadExt;
        assert_eq!(stream.read(&mut [0; 1]).await.unwrap(), 0);
        drop(listener);
        assert_eq!(probe(&endpoint).await.unwrap()["status"], "unreachable");
        assert_eq!(
            probe("udp://127.0.0.1:11010").await.unwrap()["status"],
            "unsupported"
        );
        for invalid in [
            "tcp://user:secret@127.0.0.1:80",
            "file:///etc/passwd",
            "tcp://127.0.0.1:0",
            "tcp://127.0.0.1:80#secret",
        ] {
            assert!(probe(invalid).await.is_err());
        }
    }
}
