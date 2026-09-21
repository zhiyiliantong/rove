use crate::{Agent, sharing, storage_error};
use rove_protocol::{ApiError, Request};
use rusqlite::{OptionalExtension, params};
use serde_json::{Value, json};
use uuid::Uuid;

impl Agent {
    /// Caller holds network_lifecycle. Static CIDRs reserve their range;
    /// automatic CIDRs are checked again against runtime facts before serving.
    fn check_network_join(&self, id: &str) -> Result<(), ApiError> {
        let db = self.store.connection.lock().unwrap();
        let mut stmt = db.prepare("SELECT record,join_config FROM networks WHERE id<>?1 AND (COALESCE(json_extract(record,'$.enabled'),1)<>0 OR COALESCE(json_extract(record,'$.state'),'unknown')<>'stopped')")
            .map_err(|e| storage_error(e.into()))?;
        let rows = stmt
            .query_map([id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|e| storage_error(e.into()))?;
        let own: String = db
            .query_row(
                "SELECT join_config FROM networks WHERE id=?1",
                [id],
                |row| row.get(0),
            )
            .map_err(|e| storage_error(e.into()))?;
        let own: Value = serde_json::from_str(&own).map_err(|e| storage_error(e.into()))?;
        let requested = (own["easytier"]["dhcp"] == false)
            .then(|| {
                own["easytier"]["ipv4_cidr"]
                    .as_str()
                    .and_then(|v| v.parse::<ipnet::Ipv4Net>().ok())
            })
            .flatten();
        for row in rows {
            let (record, config) = row.map_err(|e| storage_error(e.into()))?;
            let record: Value =
                serde_json::from_str(&record).map_err(|e| storage_error(e.into()))?;
            let config: Value =
                serde_json::from_str(&config).map_err(|e| storage_error(e.into()))?;
            let other = record["overlay_cidr"]
                .as_str()
                .or_else(|| {
                    (config["easytier"]["dhcp"] == false)
                        .then(|| config["easytier"]["ipv4_cidr"].as_str())
                        .flatten()
                })
                .and_then(|v| v.parse::<ipnet::Ipv4Net>().ok());
            let Some(other) = other else {
                return Err(ApiError::new(
                    409,
                    "network_address_pending",
                    "Another network has no confirmed subnet; wait for its address or stop it",
                ));
            };
            if requested.is_some_and(|own| subnets_overlap(own, other)) {
                return Err(ApiError::new(
                    409,
                    "network_subnet_conflict",
                    "Requested subnet overlaps another joined network",
                ));
            }
        }
        Ok(())
    }
    pub(crate) fn network(&self, id: &str) -> Result<(Value, Value), ApiError> {
        let result: Option<(String, String)> = self
            .store
            .connection
            .lock()
            .unwrap()
            .query_row(
                "SELECT record,join_config FROM networks WHERE id=?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .map_err(|e| storage_error(e.into()))?;
        let (record, config) =
            result.ok_or_else(|| ApiError::new(404, "network_not_found", "Network not found"))?;
        Ok((
            serde_json::from_str(&record).map_err(|e| storage_error(e.into()))?,
            serde_json::from_str(&config).map_err(|e| storage_error(e.into()))?,
        ))
    }
    fn import_join(&self, config: Value) -> Result<Value, ApiError> {
        sharing::validate_join(&config)?;
        let id = config["network_id"].as_str().unwrap();
        let mut db = self.store.connection.lock().unwrap();
        let tx = db.transaction().map_err(|e| storage_error(e.into()))?;
        let existing: Option<(String, String)> = tx
            .query_row(
                "SELECT record,join_config FROM networks WHERE id=?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .map_err(|e| storage_error(e.into()))?;
        if let Some((record, original)) = existing {
            let original: Value =
                serde_json::from_str(&original).map_err(|e| storage_error(e.into()))?;
            if original != config {
                return Err(ApiError::new(
                    409,
                    "network_config_conflict",
                    "Network already exists with different configuration; stop and update it explicitly",
                ));
            }
            return Ok(
                json!({"network":serde_json::from_str::<Value>(&record).map_err(|e| storage_error(e.into()))?,"created":false}),
            );
        }
        let record = json!({"network_id":id,"display_name":config["display_name"],"instance_id":Uuid::new_v4(),"enabled":false,"state":"stopped","overlay_addresses":[],"last_error":null,"updated_at":rove_core::now()});
        tx.execute(
            "INSERT INTO networks(id,record,join_config) VALUES(?1,?2,?3)",
            params![id, record.to_string(), config.to_string()],
        )
        .map_err(|e| storage_error(e.into()))?;
        tx.commit().map_err(|e| storage_error(e.into()))?;
        Ok(json!({"network":record,"created":true}))
    }
    pub(crate) async fn network_operation(
        &self,
        request: &Request,
    ) -> Result<(u16, Option<Value>), ApiError> {
        let _lifecycle = if matches!(
            request.operation_id.as_str(),
            "start_network" | "stop_network" | "update_network" | "delete_network"
        ) {
            Some(self.network_lifecycle.lock().await)
        } else {
            None
        };
        let body = request.body.as_ref().unwrap_or(&Value::Null);
        let id = request
            .path_parameters
            .get("network_id")
            .map(String::as_str)
            .unwrap_or("");
        let (status, value) = match request.operation_id.as_str() {
            "test_network_peer" => (
                200,
                crate::network_probe::probe(body["endpoint"].as_str().unwrap()).await?,
            ),
            "create_network" => {
                let network_id = Uuid::new_v4();
                let mut cfg = body.get("config").cloned().unwrap_or_else(|| json!({"network_name":format!("rove-{network_id}"),"network_secret":format!("{}{}",Uuid::new_v4().simple(),Uuid::new_v4().simple()),"bootstrap_peers":body.get("bootstrap_peers").cloned().unwrap_or(json!([])),"dhcp":true}));
                if let Some(legacy) = body.get("ipv4_cidr") {
                    cfg["ipv4_cidr"] = legacy.clone();
                }
                let config = json!({"schema_version":1,"network_id":network_id,"display_name":body["display_name"],"easytier":cfg});
                sharing::validate_join(&config)?;
                validate_local_address(&config, body.get("local_ipv4"), false)?;
                let result = self.import_join(config)?;
                let mut record = result["network"].clone();
                if let Some(address) = body.get("local_ipv4") {
                    record["local_ipv4"] = address.clone();
                }
                self.store
                    .connection
                    .lock()
                    .unwrap()
                    .execute(
                        "UPDATE networks SET record=?2 WHERE id=?1",
                        params![network_id.to_string(), record.to_string()],
                    )
                    .map_err(|e| storage_error(e.into()))?;
                (201, record)
            }
            "import_network" => {
                let config = if body["source"] == "manual" {
                    body["config"].clone()
                } else {
                    sharing::download(body["url"].as_str().unwrap()).await?
                };
                (200, self.import_join(config)?)
            }
            "get_network" => (200, self.network(id)?.0),
            "get_network_join_config" => (200, self.network(id)?.1),
            "list_networks" => {
                let db = self.store.connection.lock().unwrap();
                let mut stmt = db
                    .prepare("SELECT record FROM networks ORDER BY id")
                    .map_err(|e| storage_error(e.into()))?;
                let items: Result<Vec<String>, _> = stmt
                    .query_map([], |r| r.get(0))
                    .map_err(|e| storage_error(e.into()))?
                    .collect();
                let items: Result<Vec<Value>, _> = items
                    .map_err(|e| storage_error(e.into()))?
                    .iter()
                    .map(|s| serde_json::from_str(s))
                    .collect();
                (
                    200,
                    crate::page(
                        request,
                        items.map_err(|e| storage_error(e.into()))?,
                        "network_id",
                    )?,
                )
            }
            "update_network" => {
                let (mut record, mut config) = self.network(id)?;
                if record["state"] != "stopped" {
                    return Err(ApiError::new(
                        409,
                        "network_must_be_stopped",
                        "Stop the network before updating it",
                    ));
                }
                config["display_name"] = body["display_name"].clone();
                config["easytier"] = body["easytier"].clone();
                sharing::validate_join(&config)?;
                let local = body.get("local_ipv4").cloned().unwrap_or_else(|| {
                    if config["easytier"]["dhcp"] == true {
                        Value::Null
                    } else {
                        record["local_ipv4"].clone()
                    }
                });
                validate_local_address(&config, Some(&local), false)?;
                record["local_ipv4"] = local;
                record["overlay_cidr"] = Value::Null;
                record["display_name"] = body["display_name"].clone();
                record["updated_at"] = json!(rove_core::now());
                self.store
                    .connection
                    .lock()
                    .unwrap()
                    .execute(
                        "UPDATE networks SET record=?2,join_config=?3 WHERE id=?1",
                        params![id, record.to_string(), config.to_string()],
                    )
                    .map_err(|e| storage_error(e.into()))?;
                (200, record)
            }
            "delete_network" => {
                match self.network(id) {
                    Ok((record, _)) if record["state"] != "stopped" => {
                        return Err(ApiError::new(
                            409,
                            "network_must_be_stopped",
                            "Stop the network before deleting it",
                        ));
                    }
                    Err(e) if e.status == 404 => return Ok((204, None)),
                    Err(e) => return Err(e),
                    _ => {}
                }
                self.stop_services(Some(id)).await?;
                let mut db = self.store.connection.lock().unwrap();
                let tx = db.transaction().map_err(|e| storage_error(e.into()))?;
                tx.execute("DELETE FROM services WHERE network_id=?1", [id])
                    .map_err(|e| storage_error(e.into()))?;
                tx.execute("DELETE FROM networks WHERE id=?1", [id])
                    .map_err(|e| storage_error(e.into()))?;
                tx.commit().map_err(|e| storage_error(e.into()))?;
                return Ok((204, None));
            }
            "stop_network" => {
                #[cfg(all(feature = "easytier", target_os = "linux"))]
                if self.network_driver.get().is_some() {
                    let mut network = self.network(id)?.0;
                    if network["state"] == "stopped" && network["enabled"] == false {
                        return Ok((200, Some(network)));
                    }
                    network["enabled"] = json!(false);
                    network["state"] = json!("stopping");
                    network["updated_at"] = json!(rove_core::now());
                    self.save_network_state(&network)?;
                    return Ok((202, Some(network)));
                }
                let network = self.network(id)?.0;
                if network["state"] != "stopped" || network["enabled"] != false {
                    return Err(ApiError::new(
                        503,
                        "overlay_unavailable",
                        "EasyTier adapter is not configured; network exit cannot be confirmed",
                    ));
                }
                self.stop_services(Some(id)).await?;
                (200, network)
            }
            "start_network" => {
                let (record, config) = self.network(id)?;
                validate_local_address(&config, record.get("local_ipv4"), true)?;
                self.check_network_join(id)?;
                #[cfg(all(feature = "easytier", target_os = "linux"))]
                if self.network_driver.get().is_some() {
                    if self.runtime.stopping.is_cancelled() {
                        return Err(ApiError::new(503, "agent_stopping", "Agent is stopping"));
                    }
                    let mut network = self.network(id)?.0;
                    if network["enabled"] == true
                        && matches!(network["state"].as_str(), Some("starting" | "running"))
                    {
                        return Ok((200, Some(network)));
                    }
                    network["enabled"] = json!(true);
                    network["state"] = json!("starting");
                    network["overlay_addresses"] = json!([]);
                    network["overlay_cidr"] = Value::Null;
                    network["last_error"] = Value::Null;
                    network["updated_at"] = json!(rove_core::now());
                    self.save_network_state(&network)?;
                    return Ok((202, Some(network)));
                }
                return Err(ApiError::new(
                    503,
                    "overlay_unavailable",
                    "EasyTier adapter is not configured; network remains stopped",
                ));
            }
            "create_network_share" => {
                let (_, config) = self.network(id)?;
                let settings = self.store.settings().map_err(storage_error)?;
                let base = body
                    .get("config_server_url")
                    .unwrap_or(&settings["config_server_url"])
                    .as_str()
                    .ok_or_else(|| {
                        ApiError::invalid("Configure a ciphertext hosting service first")
                    })?;
                (201, sharing::upload(&config, base).await?)
            }
            "list_network_devices" => {
                self.network(id)?;
                #[cfg(all(feature = "easytier", target_os = "linux"))]
                return Ok((
                    200,
                    Some(crate::page(request, self.network_peers(id), "device_id")?),
                ));
                #[cfg(not(all(feature = "easytier", target_os = "linux")))]
                (200, json!({"items":[],"next_cursor":null}))
            }
            _ => {
                return Err(ApiError::new(
                    501,
                    "unsupported",
                    "Network operation not implemented",
                ));
            }
        };
        Ok((status, Some(value)))
    }
}

pub(crate) fn subnets_overlap(a: ipnet::Ipv4Net, b: ipnet::Ipv4Net) -> bool {
    a.contains(&b.network()) || b.contains(&a.network())
}

pub(crate) fn validate_local_address(
    config: &Value,
    local: Option<&Value>,
    required: bool,
) -> Result<Option<std::net::Ipv4Addr>, ApiError> {
    let local = local.and_then(Value::as_str);
    if config["easytier"]["dhcp"] == true {
        if local.is_some() {
            return Err(ApiError::invalid(
                "Automatic addressing does not accept a fixed local IPv4",
            ));
        }
        return Ok(None);
    }
    let Some(local) = local else {
        return if required {
            Err(ApiError::new(
                422,
                "local_address_required",
                "Set this device's static IPv4 before starting the manual network",
            ))
        } else {
            Ok(None)
        };
    };
    let address: std::net::Ipv4Addr = local
        .parse()
        .map_err(|_| ApiError::invalid("Invalid local IPv4"))?;
    let subnet: ipnet::Ipv4Net = config["easytier"]["ipv4_cidr"]
        .as_str()
        .unwrap_or("")
        .parse()
        .map_err(|_| ApiError::invalid("Manual addressing requires a network CIDR"))?;
    if !subnet.contains(&address)
        || address == subnet.network()
        || address == subnet.broadcast()
        || address.is_loopback()
        || address.is_unspecified()
        || address.is_multicast()
        || address.is_link_local()
    {
        return Err(ApiError::invalid(
            "Local IPv4 must be a usable host address in the manual subnet",
        ));
    }
    Ok(Some(address))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn manual_addresses_stay_local_and_overlap_checks_include_containment() {
        let directory = tempfile::tempdir().unwrap();
        let agent = Agent::open(&directory.path().join("agent")).unwrap();
        let config = json!({"network_name":"manual", "network_secret":"test-only", "bootstrap_peers":[], "dhcp":false, "ipv4_cidr":"192.168.100.0/24"});
        let body = json!({"display_name":"manual", "config":config, "local_ipv4":"192.168.100.1"});
        let response = agent
            .handle(&Request::new("create_network").with_body(body.clone()))
            .await;
        assert_eq!(response.status_code, 201, "{response:?}");
        let network = response.body.unwrap();
        let id = network["network_id"].as_str().unwrap();
        assert_eq!(network["local_ipv4"], "192.168.100.1");
        let (_, join) = agent.network(id).unwrap();
        assert!(join.get("local_ipv4").is_none());
        assert!(join["easytier"].get("local_ipv4").is_none());
        let other_directory = tempfile::tempdir().unwrap();
        let other = Agent::open(&other_directory.path().join("agent")).unwrap();
        assert_eq!(
            other
                .handle(
                    &Request::new("import_network")
                        .with_body(json!({"source":"manual", "config":join}))
                )
                .await
                .status_code,
            200
        );
        let response = other
            .handle(&Request::new("start_network").with_path("network_id", id))
            .await;
        assert_eq!(
            response.body.unwrap()["error"]["code"],
            "local_address_required"
        );
        for address in ["192.168.100.0", "192.168.100.255", "192.168.101.1", "bad"] {
            let mut invalid = body.clone();
            invalid["local_ipv4"] = json!(address);
            assert!(
                agent
                    .handle(&Request::new("create_network").with_body(invalid))
                    .await
                    .status_code
                    >= 400
            );
        }
        agent.store.connection.lock().unwrap().execute("UPDATE networks SET record=json_set(record,'$.enabled',json('true'),'$.state','running','$.overlay_cidr','192.168.100.0/24') WHERE id=?1", [id]).unwrap();
        for (subnet, conflict) in [
            ("192.168.0.0/16", true),
            ("192.168.100.128/25", true),
            ("192.168.101.0/24", false),
        ] {
            let mut cfg = config.clone();
            cfg["ipv4_cidr"] = json!(subnet);
            let created = agent
                .handle(
                    &Request::new("create_network")
                        .with_body(json!({"display_name":"other", "config":cfg})),
                )
                .await
                .body
                .unwrap();
            assert_eq!(
                agent
                    .check_network_join(created["network_id"].as_str().unwrap())
                    .is_err(),
                conflict
            );
        }
        other.shutdown().await;
        agent.shutdown().await;
    }

    #[tokio::test]
    async fn saved_networks_are_not_memberships_and_new_shares_need_no_address_pool() {
        let directory = tempfile::tempdir().unwrap();
        let agent = Agent::open(&directory.path().join("agent")).unwrap();
        let device = agent.store.device_id;
        let mut ids = Vec::new();
        for name in ["home", "friends"] {
            let response = agent
                .handle(&Request::new("create_network").with_body(json!({"display_name":name})))
                .await;
            assert_eq!(response.status_code, 201);
            let record = response.body.unwrap();
            assert_eq!(record["state"], "stopped");
            assert_eq!(record["enabled"], false);
            let id = record["network_id"].as_str().unwrap().to_owned();
            let (_, join) = agent.network(&id).unwrap();
            assert!(join["easytier"].get("ipv4_cidr").is_none());
            sharing::validate_join(&join).unwrap();
            let imported = agent
                .handle(
                    &Request::new("import_network")
                        .with_body(json!({"source":"manual","config":join})),
                )
                .await;
            assert_eq!(imported.body.unwrap()["created"], false);
            ids.push(id);
        }
        assert_eq!(agent.store.device_id, device);
        for state in ["starting", "running", "stopping", "failed"] {
            agent.store.connection.lock().unwrap().execute(
                "UPDATE networks SET record=json_set(record,'$.state',?2,'$.enabled',json('true')) WHERE id=?1",
                params![ids[0], state],
            ).unwrap();
            let request = Request::new("start_network").with_path("network_id", &ids[1]);
            let (first, second) = tokio::join!(agent.handle(&request), agent.handle(&request));
            for response in [first, second] {
                assert_eq!(response.status_code, 409);
                assert_eq!(
                    response.body.unwrap()["error"]["code"],
                    "network_address_pending"
                );
            }
            assert_eq!(agent.network(&ids[1]).unwrap().0["state"], "stopped");
            let stopped = agent
                .handle(&Request::new("stop_network").with_path("network_id", &ids[0]))
                .await;
            assert_eq!(stopped.status_code, 503);
            assert_eq!(agent.network(&ids[0]).unwrap().0["state"], state);
        }
        agent.store.connection.lock().unwrap().execute(
            "UPDATE networks SET record=json_set(record,'$.state','stopped','$.enabled',json('false')) WHERE id=?1", [&ids[0]],
        ).unwrap();
        // Slot is available, but an unconfigured adapter must not fake success.
        let response = agent
            .handle(&Request::new("start_network").with_path("network_id", &ids[1]))
            .await;
        assert_eq!(response.status_code, 503);
        assert_eq!(
            response.body.unwrap()["error"]["code"],
            "overlay_unavailable"
        );
        assert!(agent.network(&ids[0]).is_ok());
        agent.shutdown().await;
    }
}
