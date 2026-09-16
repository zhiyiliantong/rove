//! Linux single-network lifecycle. Requests persist intent; this observer owns
//! RPC waits and listeners, so client disconnection cannot cancel accepted work.
use crate::{Agent, http, overlay::EasyTier, storage_error};
use rove_protocol::{ApiError, contract::AGENT};
use serde_json::{Value, json};
use std::{
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    path::Path,
    sync::Arc,
    time::Duration,
};
use tokio::{sync::Mutex, task::JoinHandle};
use tokio_util::sync::CancellationToken;

pub(crate) struct Driver {
    client: EasyTier,
    listener: String,
    port: u16,
    endpoint: Mutex<Option<Endpoint>>,
    peers: std::sync::RwLock<std::collections::BTreeMap<String, Value>>,
}
struct Endpoint {
    network: String,
    interface: String,
    interface_index: u32,
    address: Ipv4Addr,
    stopping: CancellationToken,
    task: JoinHandle<anyhow::Result<()>>,
}
impl Drop for Endpoint {
    fn drop(&mut self) {
        self.stopping.cancel();
        self.task.abort();
    }
}
impl Driver {
    pub(crate) async fn close(&self, agent: &Agent) {
        for peer in self.peers.write().unwrap().values_mut() {
            peer["state"] = json!("offline");
            peer["overlay_addresses"] = json!([]);
        }
        if let Some(mut endpoint) = self.endpoint.lock().await.take() {
            endpoint.stopping.cancel();
            if tokio::time::timeout(Duration::from_secs(2), &mut endpoint.task)
                .await
                .is_err()
            {
                endpoint.task.abort();
                let _ = (&mut endpoint.task).await;
            }
        }
        let _ = agent.stop_services(None).await;
    }
}
impl Agent {
    pub(crate) fn network_peers(&self, network: &str) -> Vec<Value> {
        self.network_driver
            .get()
            .map(|driver| {
                driver
                    .peers
                    .read()
                    .unwrap()
                    .values()
                    .filter(|peer| peer["network_id"] == network)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    pub(crate) async fn target_client(
        &self,
        target: &rove_protocol::SocketTarget,
    ) -> Result<rove_sdk::RemoteClient, ApiError> {
        let driver = self.network_driver.get().ok_or_else(|| {
            ApiError::new(503, "target_unreachable", "No overlay driver is available")
        })?;
        let endpoint = driver.endpoint.lock().await;
        let endpoint = endpoint
            .as_ref()
            .filter(|e| e.network == target.network_id.to_string() && !e.task.is_finished())
            .ok_or_else(|| {
                ApiError::new(409, "network_unavailable", "Target network is not joined")
            })?;
        let peer = driver
            .peers
            .read()
            .unwrap()
            .get(&format!("{}/{}", target.network_id, target.device_id))
            .cloned()
            .ok_or_else(|| {
                ApiError::new(
                    503,
                    "target_unreachable",
                    "Target has no discovered overlay route",
                )
            })?;
        if peer["state"] == "incompatible" {
            return Err(ApiError::new(
                409,
                "protocol_incompatible",
                "Target protocol is incompatible",
            ));
        }
        if peer["state"] != "online" {
            return Err(ApiError::new(
                503,
                "target_unreachable",
                "Target is offline",
            ));
        }
        let address = peer["overlay_addresses"][0]
            .as_str()
            .ok_or_else(|| ApiError::new(503, "target_unreachable", "No target address"))?;
        rove_sdk::RemoteClient::on_interface(
            format!("http://{address}:{}", driver.port)
                .parse()
                .map_err(|_| ApiError::invalid("Invalid discovered address"))?,
            target.network_id,
            target.device_id,
            &endpoint.interface,
        )
        .map_err(|_| {
            ApiError::new(
                503,
                "target_unreachable",
                "Could not create isolated transport",
            )
        })
    }

    async fn refresh_peers(
        &self,
        driver: &Driver,
        network: &str,
        interface: &str,
        info: &Value,
        address: Ipv4Addr,
    ) {
        use futures_util::{StreamExt, stream};
        let network_id: uuid::Uuid = network.parse().unwrap();
        let local = json!({"device_id":self.store.device_id,"network_id":network_id,"display_name":"Rove device",
            "os":std::env::consts::OS,"arch":std::env::consts::ARCH,"state":"online","peer_id":info["my_node_info"]["peer_id"].as_u64().map(|p|p.to_string()),
            "overlay_addresses":[address.to_string()],"capabilities":crate::capabilities(),"last_seen_at":rove_core::now(),"last_error":null});
        let mut refreshed: std::collections::BTreeMap<_, _> = driver
            .peers
            .read()
            .unwrap()
            .iter()
            .filter(|(_, peer)| peer["network_id"] == network)
            .map(|(key, peer)| (key.clone(), peer.clone()))
            .collect();
        {
            for peer in refreshed.values_mut() {
                peer["state"] = json!("offline");
                peer["overlay_addresses"] = json!([]);
            }
            refreshed.insert(format!("{network}/{}", self.store.device_id), local);
        }
        let candidates: Vec<_> = info["routes"]
            .as_array()
            .into_iter()
            .flatten()
            .take(128)
            .filter_map(|route| {
                let raw = route["ipv4_addr"]["address"]["addr"]
                    .as_u64()
                    .and_then(|v| u32::try_from(v).ok())?;
                let ip = Ipv4Addr::from(raw);
                if ip == address
                    || ip.is_loopback()
                    || ip.is_unspecified()
                    || ip.is_multicast()
                    || ip.is_broadcast()
                {
                    return None;
                }
                Some((ip, route["peer_id"].as_u64()?.to_string()))
            })
            .collect();
        let work = stream::iter(candidates)
            .map(|(ip, peer_id)| async move {
                let base: url::Url = format!("http://{ip}:{}", driver.port).parse().ok()?;
                let hello = rove_sdk::RemoteClient::discover(base.clone(), network_id, interface)
                    .await
                    .ok()?;
                let (key, mut peer) = peer_from_hello(network_id, ip, &peer_id, &hello)?;
                let device_id = peer["device_id"].as_str()?.parse().ok()?;
                if peer["state"] == "online" {
                    let client = rove_sdk::RemoteClient::on_interface(
                        base, network_id, device_id, interface,
                    )
                    .ok()?;
                    let response = client
                        .call(rove_protocol::Request::new("get_device"))
                        .await
                        .ok()?;
                    if response.status_code != 200 {
                        return None;
                    }
                    let body = response.body?;
                    peer["os"] = body["os"].clone();
                    peer["arch"] = body["arch"].clone();
                }
                AGENT.validate("Peer", &peer).ok()?;
                Some((key, peer))
            })
            .buffer_unordered(16);
        tokio::pin!(work);
        let _ = tokio::time::timeout(Duration::from_secs(2), async {
            while let Some(result) = work.next().await {
                if let Some((key, peer)) = result
                    && (refreshed.len() < 4096 || refreshed.contains_key(&key))
                {
                    refreshed.insert(key, peer);
                }
            }
        })
        .await;
        let mut peers = driver.peers.write().unwrap();
        for (key, peer) in refreshed {
            if peers.len() < 4096 || peers.contains_key(&key) {
                peers.insert(key, peer);
            }
        }
    }

    /// Configure the explicitly selected local service, never a remotely
    /// supplied endpoint. This does not start any network until enabled=true.
    pub async fn configure_easytier(
        self: &Arc<Self>,
        portal: SocketAddr,
        core: &Path,
        listener: String,
        api_port: u16,
    ) -> anyhow::Result<()> {
        let _guard = self.network_lifecycle.lock().await;
        anyhow::ensure!(
            self.network_driver.get().is_none(),
            "Network driver already configured"
        );
        anyhow::ensure!(api_port != 0, "Agent overlay port must be nonzero");
        let client = EasyTier::connect(portal)?;
        let endpoint: url::Url = listener.parse()?;
        anyhow::ensure!(
            endpoint.scheme() == "tcp"
                && endpoint.host().is_some()
                && endpoint.port().is_some()
                && endpoint.username().is_empty()
                && endpoint.password().is_none()
                && endpoint.query().is_none()
                && endpoint.fragment().is_none(),
            "Expected an explicit TCP overlay listener"
        );
        let mut command = tokio::process::Command::new(core);
        command
            .arg("--version")
            .kill_on_drop(true)
            .stdin(std::process::Stdio::null());
        let output = tokio::time::timeout(Duration::from_secs(5), command.output()).await??;
        anyhow::ensure!(
            output.status.success()
                && String::from_utf8_lossy(&output.stdout).trim()
                    == format!("easytier-core {}", crate::overlay::EASYTIER_VERSION),
            "EasyTier binary version does not match the pinned adapter"
        );
        self.store.connection.lock().unwrap().execute(
            "UPDATE networks SET record=json_set(record,'$.state',CASE WHEN json_extract(record,'$.enabled')=1 THEN 'starting' ELSE 'stopping' END,'$.overlay_addresses',json('[]'),'$.last_error',NULL,'$.updated_at',?1) WHERE json_extract(record,'$.enabled')=1 OR json_extract(record,'$.state')<>'stopped'", [rove_core::now()],
        )?;
        self.network_driver
            .set(Driver {
                client,
                listener,
                port: api_port,
                endpoint: Mutex::new(None),
                peers: std::sync::RwLock::new(std::collections::BTreeMap::new()),
            })
            .map_err(|_| anyhow::anyhow!("Network driver already configured"))?;
        let weak = Arc::downgrade(self);
        let stopping = self.runtime.stopping.clone();
        tokio::spawn(async move {
            loop {
                let Some(agent) = weak.upgrade() else {
                    break;
                };
                if stopping.is_cancelled() {
                    break;
                }
                agent.network_tick().await;
                drop(agent);
                tokio::select! { _ = stopping.cancelled() => break,
                _ = tokio::time::sleep(Duration::from_millis(500)) => {} }
            }
        });
        Ok(())
    }

    pub(crate) fn save_network_state(&self, record: &Value) -> Result<(), ApiError> {
        AGENT.validate("Network", record)?;
        self.store
            .connection
            .lock()
            .unwrap()
            .execute(
                "UPDATE networks SET record=?2 WHERE id=?1",
                rusqlite::params![record["network_id"].as_str().unwrap(), record.to_string()],
            )
            .map_err(|e| storage_error(e.into()))?;
        Ok(())
    }

    async fn network_tick(self: &Arc<Self>) {
        let _guard = self.network_lifecycle.lock().await;
        if self.runtime.stopping.is_cancelled() {
            return;
        }
        let driver = self.network_driver.get().unwrap();
        let records = (|| -> anyhow::Result<Vec<Value>> {
            let db = self.store.connection.lock().unwrap();
            let mut stmt = db.prepare("SELECT record FROM networks WHERE json_extract(record,'$.enabled')=1 OR json_extract(record,'$.state')<>'stopped' ORDER BY id")?;
            let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
            rows.map(|r| Ok(serde_json::from_str(&r?)?)).collect()
        })();
        let Ok(mut records) = records else {
            driver.close(self).await;
            return;
        };
        if records.len() != 1 {
            driver.close(self).await;
            // Never choose an arbitrary network if old data violates the rule.
            for record in &mut records {
                if record["enabled"] == false {
                    let result = tokio::time::timeout(
                        Duration::from_secs(8),
                        self.reconcile_network(driver, record),
                    )
                    .await;
                    if matches!(result, Ok(Ok(()))) {
                        record["updated_at"] = json!(rove_core::now());
                        let _ = self.save_network_state(record);
                        continue;
                    }
                }
                record["state"] = json!("failed");
                record["overlay_addresses"] = json!([]);
                record["last_error"] = json!(ApiError::new(
                    409,
                    "network_already_joined",
                    "Multiple network intents found; stop unwanted networks before continuing"
                ));
                record["updated_at"] = json!(rove_core::now());
                let _ = self.save_network_state(record);
            }
            return;
        }
        let mut record = records.pop().unwrap();
        let result = tokio::time::timeout(
            Duration::from_secs(8),
            self.reconcile_network(driver, &mut record),
        )
        .await;
        if !matches!(result, Ok(Ok(()))) {
            driver.close(self).await;
            record["state"] = json!("failed");
            record["overlay_addresses"] = json!([]);
            record["last_error"] = json!(ApiError::new(
                503,
                "overlay_unavailable",
                "EasyTier state or isolated listener could not be established; inspect the local network service"
            ));
        }
        record["updated_at"] = json!(rove_core::now());
        if self.save_network_state(&record).is_err() {
            driver.close(self).await;
        }
    }

    async fn reconcile_network(
        self: &Arc<Self>,
        driver: &Driver,
        record: &mut Value,
    ) -> anyhow::Result<()> {
        let id = record["network_id"].as_str().unwrap().to_owned();
        let instance = record["instance_id"].as_str().unwrap().parse()?;
        let instances = driver.client.list_instances().await?;
        if record["enabled"] == false {
            driver.close(self).await;
            if instances.contains(&instance) {
                driver.client.stop(instance).await?;
            }
            anyhow::ensure!(
                !driver.client.list_instances().await?.contains(&instance),
                "Network exit not confirmed"
            );
            record["state"] = json!("stopped");
            record["overlay_addresses"] = json!([]);
            record["last_error"] = Value::Null;
            return Ok(());
        }
        // Do not adopt, overwrite or remove instances belonging to another app.
        anyhow::ensure!(
            instances.iter().all(|value| *value == instance),
            "Dedicated EasyTier service contains another instance"
        );
        if !instances.contains(&instance) {
            driver.close(self).await;
            driver
                .client
                .start(instance, &self.network(&id)?.1, &driver.listener)
                .await?;
            record["state"] = json!("starting");
            record["overlay_addresses"] = json!([]);
            record["last_error"] = Value::Null;
            return Ok(());
        }
        let info = driver.client.info(instance).await?;
        let info = &info["map"][instance.to_string()];
        anyhow::ensure!(
            info.is_object() && info["error_msg"].as_str().is_none_or(str::is_empty),
            "Network instance failed"
        );
        let node = &info["my_node_info"];
        if info["running"] != true || node["virtual_ipv4"].is_null() {
            driver.close(self).await;
            record["state"] = json!("starting");
            record["overlay_addresses"] = json!([]);
            record["last_error"] = Value::Null;
            return Ok(());
        }
        anyhow::ensure!(
            node["version"].as_str() == Some(crate::overlay::EASYTIER_VERSION),
            "Running EasyTier version mismatch"
        );
        let raw = node["virtual_ipv4"]["address"]["addr"]
            .as_u64()
            .and_then(|v| u32::try_from(v).ok())
            .ok_or_else(|| anyhow::anyhow!("Invalid overlay address"))?;
        let address = Ipv4Addr::from(raw);
        let interface = info["dev_name"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing TUN interface"))?;
        let interface_index = http::validate_overlay(interface, address)?;
        let same = driver.endpoint.lock().await.as_ref().is_some_and(|e| {
            e.network == id
                && e.interface == interface
                && e.interface_index == interface_index
                && e.address == address
                && !e.task.is_finished()
        });
        if !same {
            driver.close(self).await;
            let listener = http::bind_overlay(interface, SocketAddrV4::new(address, driver.port))?;
            let stopping = self.runtime.stopping.child_token();
            let task = tokio::spawn(http::serve_bound(
                self.clone(),
                id.parse()?,
                listener,
                stopping.clone(),
            ));
            *driver.endpoint.lock().await = Some(Endpoint {
                network: id.clone(),
                interface: interface.into(),
                interface_index,
                address,
                stopping,
                task,
            });
        }
        self.reconcile_service_overlay(&id, interface, address)
            .await?;
        self.refresh_peers(driver, &id, interface, info, address)
            .await;
        record["state"] = json!("running");
        record["overlay_addresses"] = json!([address.to_string()]);
        record["last_error"] = Value::Null;
        Ok(())
    }
}

fn peer_from_hello(
    network: uuid::Uuid,
    ip: Ipv4Addr,
    peer_id: &str,
    hello: &Value,
) -> Option<(String, Value)> {
    AGENT.validate("Hello", hello).ok()?;
    if hello["network_id"].as_str()?.parse::<uuid::Uuid>().ok()? != network {
        return None;
    }
    let device: uuid::Uuid = hello["device_id"].as_str()?.parse().ok()?;
    let compatible = rove_protocol::check_protocol(
        hello["protocol"]["min"].as_u64()?,
        hello["protocol"]["max"].as_u64()?,
    )
    .is_ok();
    let peer = json!({"device_id":device,"network_id":network,"display_name":hello["display_name"],"os":"unknown","arch":"unknown",
        "state":if compatible {"online"} else {"incompatible"},"peer_id":peer_id,"overlay_addresses":[ip.to_string()],
        "capabilities":hello["capabilities"],"last_seen_at":rove_core::now(),
        "last_error":if compatible {Value::Null} else {json!(ApiError::new(409,"protocol_incompatible","No common protocol version"))}});
    AGENT.validate("Peer", &peer).ok()?;
    Some((format!("{network}/{device}"), peer))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn hello_classification_uses_stable_identity_not_current_peer_or_address() {
        let directory = tempfile::tempdir().unwrap();
        let agent = Agent::open(&directory.path().join("agent")).unwrap();
        let network = uuid::Uuid::new_v4();
        let mut hello = agent
            .dispatch(&rove_protocol::Request::new("get_hello"))
            .await
            .unwrap()
            .1
            .unwrap();
        hello["network_id"] = json!(network);
        let ip = Ipv4Addr::new(10, 126, 126, 1);
        let (key, first) = peer_from_hello(network, ip, "42", &hello).unwrap();
        assert_eq!(first["state"], "online");
        let (changed_key, changed) =
            peer_from_hello(network, Ipv4Addr::new(10, 126, 126, 2), "99", &hello).unwrap();
        assert_eq!(key, changed_key);
        assert_ne!(first["peer_id"], changed["peer_id"]);
        assert_ne!(first["overlay_addresses"], changed["overlay_addresses"]);
        assert!(peer_from_hello(network, ip, "42", &json!({"peer_id":42})).is_none());
        assert!(peer_from_hello(uuid::Uuid::new_v4(), ip, "42", &hello).is_none());
        hello["protocol"] = json!({"min":999,"max":999});
        let (_, incompatible) = peer_from_hello(network, ip, "42", &hello).unwrap();
        assert_eq!(incompatible["state"], "incompatible");
        assert_eq!(incompatible["last_error"]["code"], "protocol_incompatible");
        agent.shutdown().await;
    }
}
