//! Local address book. Never advertised in hello, peer discovery or network shares.
use crate::{Agent, storage_error};
use rove_protocol::{ApiError, Request};
use rusqlite::{OptionalExtension, params};
use serde_json::{Value, json};

const KEY: &str = "local_device_aliases";

pub(crate) fn is_local_operation(operation: &str) -> bool {
    matches!(
        operation,
        "list_device_aliases" | "set_device_alias" | "resolve_device_alias"
    )
}

fn name(value: &str) -> Result<String, ApiError> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > 64 || value.chars().any(char::is_control) {
        return Err(ApiError::invalid(
            "Device alias must be 1–64 characters without control characters",
        ));
    }
    Ok(value.to_owned())
}

impl Agent {
    pub(crate) fn device_alias_operation(&self, request: &Request) -> Result<Value, ApiError> {
        let body = request.body.as_ref().unwrap_or(&Value::Null);
        if request.operation_id == "set_device_alias" {
            // One lock covers read, uniqueness check and write: concurrent edits cannot lose updates.
            let network = body["network_id"]
                .as_str()
                .unwrap()
                .parse::<uuid::Uuid>()
                .unwrap()
                .to_string();
            let device = body["device_id"]
                .as_str()
                .unwrap()
                .parse::<uuid::Uuid>()
                .unwrap()
                .to_string();
            let alias = body["alias"].as_str().map(name).transpose()?;
            if alias.is_some() {
                self.network(&network)?;
            }
            let db = self.store.connection.lock().unwrap();
            let raw: Option<String> = db
                .query_row("SELECT value FROM metadata WHERE key=?1", [KEY], |r| {
                    r.get(0)
                })
                .optional()
                .map_err(|e| storage_error(e.into()))?;
            let mut entries: Vec<Value> = raw
                .map(|v| serde_json::from_str(&v))
                .transpose()
                .map_err(|e| storage_error(e.into()))?
                .unwrap_or_default();
            if let Some(alias) = &alias {
                if entries.iter().any(|entry| {
                    entry["alias"].as_str().unwrap_or_default().to_lowercase()
                        == alias.to_lowercase()
                        && !(entry["network_id"] == network && entry["device_id"] == device)
                }) {
                    return Err(ApiError::new(
                        409,
                        "device_alias_conflict",
                        "This name is already used on this device",
                    ));
                }
            }
            entries.retain(|e| !(e["network_id"] == network && e["device_id"] == device));
            if alias.is_some() && entries.len() >= 512 {
                return Err(ApiError::new(
                    409,
                    "device_alias_limit",
                    "At most 512 local device aliases are supported",
                ));
            }
            let record = json!({"network_id":network,"device_id":device,"alias":alias});
            if alias.is_some() {
                entries.push(record.clone());
            }
            db.execute("INSERT INTO metadata(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
                params![KEY, serde_json::to_string(&entries).map_err(|e| storage_error(e.into()))?])
                .map_err(|e| storage_error(e.into()))?;
            return Ok(record);
        }
        let entries = self
            .store
            .get(KEY)
            .map_err(storage_error)?
            .unwrap_or_else(|| json!([]));
        if request.operation_id == "list_device_aliases" {
            return Ok(json!({"items":entries}));
        }
        let alias = name(body["alias"].as_str().unwrap())?;
        let entry = entries
            .as_array()
            .unwrap()
            .iter()
            .find(|e| {
                e["alias"].as_str().unwrap_or_default().to_lowercase() == alias.to_lowercase()
            })
            .ok_or_else(|| {
                ApiError::new(
                    404,
                    "device_alias_not_found",
                    "No local device alias matches; do not execute locally",
                )
            })?;
        let network = entry["network_id"].as_str().unwrap();
        let (record, _) = self.network(network)?;
        if record["state"] != "running" {
            return Err(ApiError::new(
                409,
                "network_unavailable",
                "Alias network is not running; no local fallback",
            ));
        }
        #[cfg(all(
            feature = "easytier",
            any(target_os = "linux", target_os = "android", windows)
        ))]
        let peers = self.network_peers(network);
        #[cfg(not(all(
            feature = "easytier",
            any(target_os = "linux", target_os = "android", windows)
        )))]
        let peers: Vec<Value> = vec![];
        Self::resolve_alias_peer(entry, &peers, &self.store.device_id.to_string())
    }

    fn resolve_alias_peer(entry: &Value, peers: &[Value], local: &str) -> Result<Value, ApiError> {
        let peer = peers
            .iter()
            .find(|p| {
                p["device_id"] == entry["device_id"]
                    && p["network_id"] == entry["network_id"]
                    && p["state"] == "online"
            })
            .ok_or_else(|| {
                ApiError::new(
                    503,
                    "target_unreachable",
                    "Named device is not online; no local fallback",
                )
            })?;
        Ok(
            json!({"alias":entry["alias"],"target":{"network_id":entry["network_id"],"device_id":entry["device_id"]},
            "overlay_addresses":peer["overlay_addresses"],"is_local":entry["device_id"] == local}),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    async fn set(
        agent: &Arc<Agent>,
        network: &str,
        device: &str,
        alias: Value,
    ) -> rove_protocol::Response {
        agent
            .handle(
                &Request::new("set_device_alias")
                    .with_body(json!({"network_id":network,"device_id":device,"alias":alias})),
            )
            .await
    }

    #[tokio::test]
    async fn aliases_are_private_unique_persistent_and_fail_closed() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("origin");
        let agent = Agent::open(&path).unwrap();
        let other = Agent::open(&tmp.path().join("other")).unwrap();
        let network = agent
            .handle(&Request::new("create_network").with_body(json!({"display_name":"aliases"})))
            .await
            .body
            .unwrap()["network_id"]
            .as_str()
            .unwrap()
            .to_owned();
        let device = uuid::Uuid::new_v4().to_string();
        assert_eq!(
            set(&agent, &network, &device, json!(" 客厅小盒子 "))
                .await
                .status_code,
            200
        );
        let second = uuid::Uuid::new_v4().to_string();
        assert_eq!(
            set(&agent, &network, &second, json!("客厅小盒子"))
                .await
                .status_code,
            409
        );
        assert_eq!(
            set(&agent, &network, &device, json!("Box"))
                .await
                .status_code,
            200
        );
        assert_eq!(
            set(&agent, &network, &second, json!(" box "))
                .await
                .status_code,
            409
        );
        for bad in [json!("   "), json!("bad\nname"), json!("a".repeat(65))] {
            assert!(set(&agent, &network, &device, bad).await.status_code >= 400);
        }
        let list = Request::new("list_device_aliases");
        assert_eq!(
            agent.handle(&list).await.body.unwrap()["items"][0]["alias"],
            "Box"
        );
        assert_eq!(other.handle(&list).await.body.unwrap()["items"], json!([]));
        // Same identity, different agent-local name. No propagation or shared state.
        other
            .store
            .connection
            .lock()
            .unwrap()
            .execute(
                "INSERT INTO networks(id,record,join_config) VALUES(?1,?2,'{}')",
                params![network, json!({"state":"stopped"}).to_string()],
            )
            .unwrap();
        assert_eq!(
            set(&other, &network, &device, json!("音乐盒子"))
                .await
                .status_code,
            200
        );
        for op in ["get_hello", "get_device"] {
            assert_eq!(
                agent.handle(&Request::new(op)).await.body.unwrap()["display_name"],
                "Rove device"
            );
        }
        let resolve =
            |alias: &str| Request::new("resolve_device_alias").with_body(json!({"alias":alias}));
        assert_eq!(agent.handle(&resolve("unknown")).await.status_code, 404);
        assert_eq!(agent.handle(&resolve("Box")).await.status_code, 409);
        let mut remote = list.clone();
        remote.target = Some(rove_protocol::SocketTarget {
            network_id: network.parse().unwrap(),
            device_id: device.parse().unwrap(),
        });
        assert_eq!(agent.handle(&remote).await.status_code, 400);
        assert!(crate::management::call(&agent,&json!({"operation_id":"resolve_device_alias","target":remote.target,"body":{"alias":"Box"}})).await.is_err());
        assert_eq!(
            crate::management::call(&agent, &json!({"operation_id":"list_device_aliases"}))
                .await
                .unwrap()
                .unwrap()["items"][0]["alias"],
            "Box"
        );
        // The overlay HTTP boundary must never expose the local address book.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let stop = tokio_util::sync::CancellationToken::new();
        let server = tokio::spawn(crate::http::serve_bound(
            agent.clone(),
            network.parse().unwrap(),
            listener,
            stop.clone(),
        ));
        let response = reqwest::Client::new()
            .get(format!("http://{address}/v1/local/device-aliases"))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 403);
        stop.cancel();
        server.await.unwrap().unwrap();
        agent.shutdown().await;
        drop(agent);
        let reopened = Agent::open(&path).unwrap();
        assert_eq!(
            reopened.handle(&list).await.body.unwrap()["items"][0]["alias"],
            "Box"
        );
        assert_eq!(
            set(&reopened, &network, &device, Value::Null)
                .await
                .status_code,
            200
        );
        assert_eq!(
            reopened.handle(&list).await.body.unwrap()["items"],
            json!([])
        );
        assert_eq!(
            other.handle(&list).await.body.unwrap()["items"][0]["alias"],
            "音乐盒子"
        );
        reopened.shutdown().await;
        other.shutdown().await;
    }

    #[test]
    fn alias_resolution_uses_current_identity_and_address() {
        let entry = json!({"network_id":"network-a","device_id":"device-a","alias":"客厅小盒子"});
        let mut peer = json!({"network_id":"network-a","device_id":"device-a","state":"online","overlay_addresses":["10.0.0.2"]});
        assert_eq!(
            Agent::resolve_alias_peer(&entry, &[peer.clone()], "other").unwrap()["target"]["device_id"],
            "device-a"
        );
        peer["overlay_addresses"] = json!(["10.0.0.9"]);
        assert_eq!(
            Agent::resolve_alias_peer(&entry, &[peer.clone()], "device-a").unwrap()["overlay_addresses"],
            json!(["10.0.0.9"])
        );
        assert_eq!(
            Agent::resolve_alias_peer(&entry, &[peer.clone()], "device-a").unwrap()["is_local"],
            true
        );
        peer["state"] = json!("offline");
        assert_eq!(
            Agent::resolve_alias_peer(&entry, &[peer.clone()], "other")
                .unwrap_err()
                .status,
            503
        );
        peer["state"] = json!("online");
        peer["network_id"] = json!("network-b");
        assert!(Agent::resolve_alias_peer(&entry, &[peer], "other").is_err());
    }
}
