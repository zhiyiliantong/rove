#![cfg(unix)]
use rove_agent::Agent;
use rove_protocol::Request;
use serde_json::json;
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn two_clients_share_state_and_retain_identity() {
    let temp = tempfile::tempdir().unwrap();
    let data = temp.path().join("device");
    let agent = Agent::open(&data).unwrap();
    let id = agent.store.device_id;
    let socket = rove_sdk::socket_path(&agent.store.data_dir);
    let shutdown = CancellationToken::new();
    let server = tokio::spawn(rove_agent::serve(agent.clone(), shutdown.clone()));
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while !socket.exists() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let first = rove_sdk::LocalClient::new(&socket);
    let second = rove_sdk::LocalClient::new(&socket);
    let (a, b) = tokio::join!(
        first.call(Request::new("get_device")),
        second.call(Request::new("get_device"))
    );
    assert_eq!(
        a.unwrap().body.unwrap()["device_id"],
        b.unwrap().body.unwrap()["device_id"]
    );
    first
        .call(Request::new("update_settings").with_body(json!({"max_active_runs":7})))
        .await
        .unwrap();
    assert_eq!(
        second
            .call(Request::new("get_settings"))
            .await
            .unwrap()
            .body
            .unwrap()["max_active_runs"],
        7
    );
    let created = first
        .call(Request::new("create_network").with_body(json!({"display_name":"test"})))
        .await
        .unwrap()
        .body
        .unwrap();
    let network_id = created["network_id"].as_str().unwrap();
    let join = first
        .call(Request::new("get_network_join_config").with_path("network_id", network_id))
        .await
        .unwrap()
        .body
        .unwrap();
    assert!(join.get("device_id").is_none());
    let repeat = first
        .call(Request::new("import_network").with_body(json!({"source":"manual","config":join})))
        .await
        .unwrap();
    assert_eq!(repeat.body.unwrap()["created"], false);
    let mut conflict = join.clone();
    conflict["easytier"]["network_secret"] = json!("different");
    assert_eq!(
        first
            .call(
                Request::new("import_network")
                    .with_body(json!({"source":"manual","config":conflict}))
            )
            .await
            .unwrap()
            .status_code,
        409
    );
    assert_eq!(
        first
            .call(Request::new("get_network_join_config").with_path("network_id", network_id))
            .await
            .unwrap()
            .body
            .unwrap(),
        join
    );
    shutdown.cancel();
    server.await.unwrap().unwrap();
    drop(agent);
    let reopened = Agent::open(&data).unwrap();
    assert_eq!(reopened.store.device_id, id);
    assert_eq!(reopened.store.settings().unwrap()["max_active_runs"], 7);
}
