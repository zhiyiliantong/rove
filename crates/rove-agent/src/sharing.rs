//! Join secrets stay on clients. Nothing sent to the blob host includes the key.
use aes_gcm::{
    Aes256Gcm, KeyInit, Nonce,
    aead::{Aead, AeadCore, OsRng, Payload},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD as B64};
use rove_protocol::{
    ApiError,
    contract::{AGENT, BLOBS},
};
use serde_json::{Value, json};
const AAD: &[u8] = b"rove.network_join_config.v1";

pub fn validate_join(value: &Value) -> Result<(), ApiError> {
    if value["schema_version"] != 1 {
        return Err(ApiError::new(
            422,
            "unsupported_config_version",
            "Unsupported join configuration version",
        ));
    }
    AGENT.validate("JoinConfig", value)?;
    let cfg = &value["easytier"];
    // Preserve old development share payloads without claiming that their CIDR
    // controls EasyTier DHCP. New configurations do not include this metadata.
    if let Some(cidr) = cfg.get("ipv4_cidr") {
        let cidr: ipnet::Ipv4Net = cidr.as_str().unwrap().parse().map_err(|_| {
            ApiError::new(422, "invalid_network_config", "Invalid legacy IPv4 CIDR")
        })?;
        if cidr.prefix_len() == 0 || cidr.prefix_len() > 30 || cidr.addr() != cidr.network() {
            return Err(ApiError::new(
                422,
                "invalid_network_config",
                "Invalid legacy IPv4 CIDR",
            ));
        }
    }
    for peer in cfg["bootstrap_peers"].as_array().unwrap() {
        let peer = url::Url::parse(peer.as_str().unwrap())
            .map_err(|_| ApiError::invalid("Invalid bootstrap peer"))?;
        if !matches!(peer.scheme(), "tcp" | "udp" | "ws" | "wss" | "quic" | "wg")
            || peer.host().is_none()
            || !peer.username().is_empty()
            || peer.password().is_some()
            || peer.fragment().is_some()
        {
            return Err(ApiError::new(
                422,
                "invalid_network_config",
                "Unsupported bootstrap peer endpoint",
            ));
        }
    }
    Ok(())
}
pub fn encrypt(config: &Value) -> Result<(Value, String), ApiError> {
    validate_join(config)?;
    let plaintext = serde_json::to_vec(config).unwrap();
    if plaintext.len() + 16 > 128 * 1024 {
        return Err(ApiError::new(
            413,
            "payload_too_large",
            "Join configuration is too large",
        ));
    }
    let key = Aes256Gcm::generate_key(&mut OsRng);
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ciphertext = Aes256Gcm::new(&key)
        .encrypt(
            &nonce,
            Payload {
                msg: &plaintext,
                aad: AAD,
            },
        )
        .map_err(|_| {
            ApiError::new(
                500,
                "encryption_failed",
                "Cannot encrypt join configuration",
            )
        })?;
    Ok((
        json!({"schema_version":1,"algorithm":"aes-256-gcm","nonce":B64.encode(nonce),"ciphertext":B64.encode(ciphertext)}),
        B64.encode(key),
    ))
}
pub fn decrypt(envelope: &Value, key: &str) -> Result<Value, ApiError> {
    if envelope["schema_version"] != 1 || envelope["algorithm"] != "aes-256-gcm" {
        return Err(ApiError::new(
            422,
            "unsupported_config_version",
            "Unsupported encrypted configuration format",
        ));
    }
    BLOBS.validate("CipherEnvelope", envelope)?;
    let failure = || {
        ApiError::new(
            422,
            "decryption_failed",
            "Invalid key or damaged encrypted configuration",
        )
    };
    let key = B64.decode(key).map_err(|_| failure())?;
    let nonce = B64
        .decode(envelope["nonce"].as_str().unwrap())
        .map_err(|_| failure())?;
    let ciphertext = B64
        .decode(envelope["ciphertext"].as_str().unwrap())
        .map_err(|_| failure())?;
    if key.len() != 32
        || nonce.len() != 12
        || ciphertext.len() < 16
        || ciphertext.len() > 128 * 1024
    {
        return Err(failure());
    }
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|_| failure())?;
    let bytes = cipher
        .decrypt(
            Nonce::from_slice(&nonce),
            Payload {
                msg: &ciphertext,
                aad: AAD,
            },
        )
        .map_err(|_| failure())?;
    let config: Value = serde_json::from_slice(&bytes).map_err(|_| failure())?;
    validate_join(&config)?;
    Ok(config)
}
pub fn parse_share_url(value: &str) -> Result<(url::Url, String), ApiError> {
    let mut url = url::Url::parse(value).map_err(|_| ApiError::invalid("Invalid share URL"))?;
    let fragment = url
        .fragment()
        .ok_or_else(|| ApiError::invalid("Share URL has no decryption key"))?;
    let key = fragment
        .strip_prefix("key=")
        .filter(|s| s.len() == 43)
        .ok_or_else(|| ApiError::invalid("Invalid share key"))?
        .to_string();
    if B64.decode(&key).map_or(true, |v| v.len() != 32) {
        return Err(ApiError::invalid("Invalid share key encoding"));
    }
    url.set_fragment(None);
    crate::validate_server_url(url.as_str())?;
    let id = url
        .path()
        .strip_prefix("/c/")
        .ok_or_else(|| ApiError::invalid("Expected /c/<blob_id> share URL"))?;
    if id.len() != 32
        || !id
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
    {
        return Err(ApiError::invalid("Invalid share identifier"));
    }
    Ok((url, key))
}
fn client() -> Result<reqwest::Client, ApiError> {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|_| unavailable())
}
fn unavailable() -> ApiError {
    ApiError::new(
        503,
        "config_server_unavailable",
        "Configuration server is unavailable",
    )
}
async fn bounded_json(mut response: reqwest::Response) -> Result<Value, ApiError> {
    if response.status() == 410 {
        return Err(ApiError::new(
            410,
            "share_expired",
            "This share has expired; existing members are unaffected",
        ));
    }
    if response.status() == 404 {
        return Err(ApiError::new(
            404,
            "blob_not_found",
            "Share not found or already cleaned up",
        ));
    }
    if !response.status().is_success() {
        return Err(unavailable());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| unavailable())? {
        if bytes.len() + chunk.len() > 262144 {
            return Err(ApiError::new(
                413,
                "payload_too_large",
                "Configuration response too large",
            ));
        }
        bytes.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&bytes).map_err(|_| unavailable())
}
pub async fn download(value: &str) -> Result<Value, ApiError> {
    let (url, key) = parse_share_url(value)?;
    let expected_id = url.path().strip_prefix("/c/").unwrap().to_owned();
    let blob = bounded_json(client()?.get(url).send().await.map_err(|_| unavailable())?).await?;
    BLOBS.validate("Blob", &blob)?;
    if blob["blob_id"] != expected_id {
        return Err(ApiError::invalid("Downloaded share identifier mismatch"));
    }
    decrypt(&blob["envelope"], &key)
}
pub async fn upload(config: &Value, base: &str) -> Result<Value, ApiError> {
    let base = crate::validate_server_url(base)?;
    let (envelope, key) = encrypt(config)?;
    let blob = bounded_json(
        client()?
            .post(base.join("/v1/blobs").map_err(|_| unavailable())?)
            .json(&json!({"envelope":envelope}))
            .send()
            .await
            .map_err(|_| unavailable())?,
    )
    .await?;
    BLOBS.validate("BlobCreated", &blob)?;
    // Construct from the configured origin. Never attach secrets to a server-selected origin.
    let mut share = base
        .join(&format!("/c/{}", blob["blob_id"].as_str().unwrap()))
        .map_err(|_| unavailable())?;
    share.set_fragment(Some(&format!("key={key}")));
    Ok(
        json!({"network_id":config["network_id"],"url":share.as_str(),"expires_at":blob["expires_at"]}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn interoperable_and_tamper_safe() {
        let vector: Value = serde_json::from_str(include_str!(
            "../../../openspec/changes/bootstrap-rove/api/sharing-test-vector.json"
        ))
        .unwrap();
        assert_eq!(
            decrypt(&vector["envelope"], vector["key"].as_str().unwrap()).unwrap(),
            vector["plaintext"]
        );
        let (envelope, key) = encrypt(&vector["plaintext"]).unwrap();
        assert_eq!(decrypt(&envelope, &key).unwrap(), vector["plaintext"]);
        assert_ne!(encrypt(&vector["plaintext"]).unwrap().0, envelope);
        assert!(decrypt(&envelope, vector["key"].as_str().unwrap()).is_err());
        let mut broken = envelope;
        let mut bytes = B64.decode(broken["ciphertext"].as_str().unwrap()).unwrap();
        bytes[0] ^= 1;
        broken["ciphertext"] = json!(B64.encode(bytes));
        assert!(decrypt(&broken, &key).is_err());
        broken["schema_version"] = json!(2);
        assert_eq!(
            decrypt(&broken, &key).unwrap_err().code,
            "unsupported_config_version"
        );
        let share = format!("https://example.com/c/0123456789abcdef0123456789abcdef#key={key}");
        assert!(parse_share_url(&share).unwrap().0.fragment().is_none());
        let mut extra = vector["plaintext"].clone();
        extra["device_id"] = json!("private-device");
        assert!(validate_join(&extra).is_err());
    }
}
