//! Service definitions are durable; live listeners are owned by this agent.
//! Only the network adapter may supply a verified overlay address. Persisted
//! network JSON and caller-supplied service data are never listener authority.
use crate::{Agent, storage_error};
use rove_protocol::{ApiError, Request, contract::AGENT};
use rusqlite::{OptionalExtension, params};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    sync::{Arc, RwLock},
};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{Mutex, Semaphore},
    task::{JoinHandle, JoinSet},
};

#[derive(Default)]
pub(crate) struct Runtime(Mutex<State>);
#[derive(Default)]
struct State {
    // Empty until the EasyTier adapter establishes the actual interface. Tests
    // inject only private loopback listeners; there is no API to set this map.
    addresses: HashMap<String, IpAddr>,
    interfaces: HashMap<String, String>,
    proxies: HashMap<String, Proxy>,
}
struct Proxy {
    network_id: String,
    address: SocketAddr,
    target: Arc<RwLock<SocketAddr>>,
    task: JoinHandle<()>,
}
impl Drop for Proxy {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl Proxy {
    fn start(listener: TcpListener, target: SocketAddr, network_id: &str) -> Self {
        let address = listener.local_addr().expect("bound listener");
        let target = Arc::new(RwLock::new(target));
        let shared = target.clone();
        let task = tokio::spawn(async move {
            let slots = Arc::new(Semaphore::new(128));
            let mut connections = JoinSet::new();
            loop {
                tokio::select! {
                    accepted=listener.accept()=>{
                        let Ok((mut incoming,_))=accepted else {break};
                        let Ok(permit)=slots.clone().try_acquire_owned() else {continue};
                        let target=*shared.read().unwrap();
                        connections.spawn(async move {
                            let _permit=permit;
                            if let Ok(Ok(mut outgoing))=tokio::time::timeout(std::time::Duration::from_secs(5),TcpStream::connect(target)).await {
                                let _=tokio::io::copy_bidirectional(&mut incoming,&mut outgoing).await;
                            }
                        });
                    },
                    _=connections.join_next(),if !connections.is_empty()=>{},
                }
            }
            // Dropping the set closes only this proxy's connections.
        });
        Self {
            network_id: network_id.to_owned(),
            address,
            target,
            task,
        }
    }
    async fn stop(mut self) {
        self.task.abort();
        let _ = (&mut self.task).await;
    }
}
fn target(body: &Value) -> Result<SocketAddr, ApiError> {
    let ip: IpAddr = body["target"]["host"]
        .as_str()
        .unwrap()
        .parse()
        .map_err(|_| ApiError::invalid("Service target must be an explicit loopback IP"))?;
    if !ip.is_loopback() {
        return Err(ApiError::invalid("Service target must be loopback"));
    }
    Ok(SocketAddr::new(
        ip,
        body["target"]["port"].as_u64().unwrap() as u16,
    ))
}
fn read(agent: &Agent, id: &str) -> Result<Value, ApiError> {
    let record: Option<String> = agent
        .store
        .connection
        .lock()
        .unwrap()
        .query_row("SELECT record FROM services WHERE id=?1", [id], |r| {
            r.get(0)
        })
        .optional()
        .map_err(|e| storage_error(e.into()))?;
    serde_json::from_str(&record.ok_or_else(|| {
        ApiError::new(404, "service_not_found", "Service not found on this device")
    })?)
    .map_err(|e| storage_error(e.into()))
}
fn view(mut record: Value, state: &State) -> Value {
    let id = record["service_id"].as_str().unwrap();
    if let Some(proxy) = state.proxies.get(id).filter(|p| !p.task.is_finished()) {
        record["state"] = json!("published");
        record["endpoints"] = json!([format!(
            "{}://{}",
            record["protocol"].as_str().unwrap(),
            proxy.address
        )]);
        record["last_error"] = Value::Null;
    } else if state
        .addresses
        .contains_key(record["network_id"].as_str().unwrap())
    {
        record["endpoints"] = json!([]);
        if record["state"] != "failed" || record["last_error"].is_null() {
            record["last_error"] = json!(ApiError::new(
                503,
                "service_listener_stopped",
                "Service listener is not active; restore or update its publication"
            ));
        }
        record["state"] = json!("failed");
    } else {
        record["state"] = json!("unavailable");
        record["endpoints"] = json!([]);
        record["last_error"] = json!(ApiError::new(
            503,
            "overlay_unavailable",
            "Service listener is not active; the saved definition is retained"
        ));
    }
    record["target_status"] = json!("unknown");
    record
}
impl Agent {
    #[cfg(all(feature = "easytier", target_os = "linux"))]
    pub(crate) async fn reconcile_service_overlay(
        &self,
        network: &str,
        interface: &str,
        address: std::net::Ipv4Addr,
    ) -> Result<(), ApiError> {
        self.services
            .0
            .lock()
            .await
            .interfaces
            .insert(network.to_owned(), interface.to_owned());
        self.reconcile_service_network(network, Some(address.into()))
            .await
    }
    /// Internal lifecycle hook. A non-null IP must come from the verified
    /// EasyTier interface, never persisted Network JSON or an API caller.
    /// Production online calls additionally supply the verified TUN interface.
    pub(crate) async fn reconcile_service_network(
        &self,
        network: &str,
        address: Option<IpAddr>,
    ) -> Result<(), ApiError> {
        let mut state = self.services.0.lock().await;
        // Retire every obsolete listener before any fallible storage/rebind
        // work, so a partial failure cannot leave old-address entry points.
        let obsolete: Vec<String> = state
            .proxies
            .iter()
            .filter(|(_, proxy)| {
                proxy.network_id == network
                    && (Some(proxy.address.ip()) != address || proxy.task.is_finished())
            })
            .map(|(id, _)| id.clone())
            .collect();
        for id in obsolete {
            if let Some(proxy) = state.proxies.remove(&id) {
                proxy.stop().await;
            }
        }
        let Some(address) = address else {
            state.addresses.remove(network);
            state.interfaces.remove(network);
            return Ok(());
        };
        if address.is_unspecified()
            || address.is_multicast()
            || matches!(address,IpAddr::V4(ip) if ip.is_broadcast())
        {
            state.addresses.remove(network);
            return Err(ApiError::invalid(
                "Service listeners require a specific unicast interface address",
            ));
        }
        state.addresses.insert(network.to_owned(), address);
        self.network(network)?;
        let records: Vec<Value> = {
            let db = self.store.connection.lock().unwrap();
            let mut statement = db
                .prepare("SELECT record FROM services WHERE network_id=?1 ORDER BY id")
                .map_err(|e| storage_error(e.into()))?;
            let rows = statement
                .query_map([network], |r| r.get::<_, String>(0))
                .map_err(|e| storage_error(e.into()))?;
            let mut records = Vec::new();
            for row in rows {
                records.push(
                    serde_json::from_str(&row.map_err(|e| storage_error(e.into()))?)
                        .map_err(|e| storage_error(e.into()))?,
                );
            }
            records
        };
        for mut record in records {
            let id = record["service_id"].as_str().unwrap().to_owned();
            let desired = SocketAddr::new(address, record["listen_port"].as_u64().unwrap() as u16);
            if state
                .proxies
                .get(&id)
                .is_some_and(|p| p.address == desired && !p.task.is_finished())
            {
                continue;
            }
            if let Some(old) = state.proxies.remove(&id) {
                old.stop().await;
            }
            let destination = target(&record)?;
            let listener = bind_service(&state, network, desired).await;
            record["endpoints"] = json!([]);
            record["target_status"] = json!("unknown");
            record["updated_at"] = json!(rove_core::now());
            match &listener {
                Ok(_) => {
                    record["state"] = json!("published");
                    record["last_error"] = Value::Null;
                    record["endpoints"] = json!([format!(
                        "{}://{desired}",
                        record["protocol"].as_str().unwrap()
                    )]);
                }
                Err(_) => {
                    record["state"] = json!("failed");
                    record["last_error"] = json!(ApiError::new(
                        409,
                        "service_bind_failed",
                        "Saved service port is unavailable; original port and definition are retained"
                    ));
                }
            }
            AGENT.validate("Service", &record)?;
            self.store
                .connection
                .lock()
                .unwrap()
                .execute(
                    "UPDATE services SET record=?2 WHERE id=?1",
                    params![id, record.to_string()],
                )
                .map_err(|e| storage_error(e.into()))?;
            if let Ok(listener) = listener {
                state
                    .proxies
                    .insert(id, Proxy::start(listener, destination, network));
            }
        }
        Ok(())
    }
    pub(crate) async fn stop_services(&self, network: Option<&str>) -> Result<(), ApiError> {
        if let Some(network) = network {
            return self.reconcile_service_network(network, None).await;
        }
        let mut state = self.services.0.lock().await;
        let ids: Vec<String> = state.proxies.keys().cloned().collect();
        state.addresses.clear();
        state.interfaces.clear();
        for id in ids {
            if let Some(proxy) = state.proxies.remove(&id) {
                proxy.stop().await;
            }
        }
        Ok(())
    }
    pub(crate) async fn service_operation(
        &self,
        request: &Request,
    ) -> Result<(u16, Option<Value>), ApiError> {
        let id = request
            .path_parameters
            .get("service_id")
            .map(String::as_str)
            .unwrap_or("");
        let body = request.body.as_ref().unwrap_or(&Value::Null);
        let mut state = self.services.0.lock().await;
        match request.operation_id.as_str() {
            "list_services" => {
                let network = request.query_parameters["network_id"].as_str().unwrap();
                self.network(network)?;
                let db = self.store.connection.lock().unwrap();
                let mut statement = db
                    .prepare("SELECT record FROM services WHERE network_id=?1 ORDER BY id")
                    .map_err(|e| storage_error(e.into()))?;
                let rows = statement
                    .query_map([network], |r| r.get::<_, String>(0))
                    .map_err(|e| storage_error(e.into()))?;
                let mut items = Vec::new();
                for row in rows {
                    items.push(view(
                        serde_json::from_str(&row.map_err(|e| storage_error(e.into()))?)
                            .map_err(|e| storage_error(e.into()))?,
                        &state,
                    ));
                }
                Ok((200, Some(crate::page(request, items, "service_id")?)))
            }
            "get_service" => Ok((200, Some(view(read(self, id)?, &state)))),
            "unpublish_service" => {
                // Persist removal first. A failed database write keeps the old
                // publication intact; successful response waits for port close.
                self.store
                    .connection
                    .lock()
                    .unwrap()
                    .execute("DELETE FROM services WHERE id=?1", [id])
                    .map_err(|e| storage_error(e.into()))?;
                if let Some(proxy) = state.proxies.remove(id) {
                    proxy.stop().await;
                }
                Ok((204, None))
            }
            "publish_service" | "update_service" => {
                AGENT.validate("ServiceWrite", body)?;
                let destination = target(body)?;
                let updating = request.operation_id == "update_service";
                let old = if updating {
                    Some(read(self, id)?)
                } else {
                    None
                };
                let network = body["network_id"].as_str().unwrap();
                if old.as_ref().is_some_and(|r| r["network_id"] != network) {
                    return Err(ApiError::new(
                        409,
                        "service_network_conflict",
                        "An existing service cannot change network membership",
                    ));
                }
                self.network(network)?;
                let ip = *state.addresses.get(network).ok_or_else(|| {
                    ApiError::new(
                        503,
                        "overlay_unavailable",
                        "No verified overlay interface is available; no proxy was opened",
                    )
                })?;
                let port = body
                    .get("listen_port")
                    .or_else(|| old.as_ref().map(|r| &r["listen_port"]))
                    .and_then(Value::as_u64)
                    .unwrap_or(0) as u16;
                let requested = SocketAddr::new(ip, port);
                let reuse = state
                    .proxies
                    .get(id)
                    .is_some_and(|p| p.address == requested && !p.task.is_finished());
                let listener = if reuse {
                    None
                } else {
                    Some(bind_service(&state, network, requested).await.map_err(|_|ApiError::new(409,"service_bind_failed","Could not bind the requested service port; previous publication is unchanged"))?)
                };
                let address = listener
                    .as_ref()
                    .map(|l| l.local_addr().unwrap())
                    .unwrap_or(requested);
                let id = if updating {
                    id.to_owned()
                } else {
                    uuid::Uuid::new_v4().to_string()
                };
                let record = json!({"service_id":id,"network_id":network,"device_id":self.store.device_id,"name":body["name"],"target":body["target"],"protocol":body["protocol"],"listen_port":address.port(),"access_info":body.get("access_info").cloned().unwrap_or(json!("")),"state":"published","endpoints":[format!("{}://{address}",body["protocol"].as_str().unwrap())],"target_status":"unknown","last_error":null,"updated_at":rove_core::now()});
                AGENT.validate("Service", &record)?;
                self.store.connection.lock().unwrap().execute("INSERT INTO services(id,network_id,record) VALUES(?1,?2,?3) ON CONFLICT(id) DO UPDATE SET record=excluded.record",params![id,network,record.to_string()]).map_err(|e|storage_error(e.into()))?;
                // No fallible operations between commit and activation. A
                // failed commit drops the prepared listener and keeps the old.
                if let Some(listener) = listener {
                    let proxy = Proxy::start(listener, destination, network);
                    if let Some(previous) = state.proxies.insert(id, proxy) {
                        previous.stop().await;
                    }
                } else if let Some(proxy) = state.proxies.get(&id) {
                    *proxy.target.write().unwrap() = destination;
                }
                Ok((if updating { 200 } else { 201 }, Some(record)))
            }
            _ => Err(ApiError::new(
                501,
                "unsupported",
                "Unknown service operation",
            )),
        }
    }
}

async fn bind_service(
    state: &State,
    network: &str,
    address: SocketAddr,
) -> std::io::Result<TcpListener> {
    #[cfg(target_os = "linux")]
    if let (Some(interface), SocketAddr::V4(address)) = (state.interfaces.get(network), address) {
        return crate::http::bind_overlay(interface, address).map_err(std::io::Error::other);
    }
    #[cfg(not(target_os = "linux"))]
    let _ = (state, network);
    // Only private tests may inject loopback. There is no production escape
    // hatch from the interface-bound overlay listener.
    #[cfg(test)]
    if address.ip().is_loopback() {
        return TcpListener::bind(address).await;
    }
    let _ = address;
    Err(std::io::Error::other(
        "No verified overlay interface binding",
    ))
}

#[cfg(test)]
#[path = "service_tests.rs"]
mod tests;
