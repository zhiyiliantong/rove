#![cfg(unix)]
use rove_agent::Agent;
use rove_protocol::Request;
use serde_json::json;
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn two_sdk_clients_observe_persistent_archive_restore_and_delete() {
    let temp = tempfile::tempdir().unwrap();
    let data = temp.path().join("agent");
    let agent = Agent::open(&data).unwrap();
    let socket = rove_sdk::socket_path(&agent.store.data_dir);
    let stop = CancellationToken::new();
    let server = tokio::spawn(rove_agent::serve(agent.clone(), stop.clone()));
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while !socket.exists() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let a = rove_sdk::LocalClient::new(&socket);
    let b = rove_sdk::LocalClient::new(&socket);
    let session = a
        .call(Request::new("create_session").with_body(json!({"title":"keep identity"})))
        .await
        .unwrap()
        .body
        .unwrap();
    let id = session["session_id"].as_str().unwrap();
    let op = |name| Request::new(name).with_path("session_id", id);
    assert_eq!(
        a.call(op("archive_session")).await.unwrap().status_code,
        204
    );
    assert!(b.call(op("get_session")).await.unwrap().body.unwrap()["archived_at"].is_string());
    stop.cancel();
    server.await.unwrap().unwrap();
    drop(agent);
    let agent = Agent::open(&data).unwrap();
    let stop = CancellationToken::new();
    let server = tokio::spawn(rove_agent::serve(agent.clone(), stop.clone()));
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while !socket.exists() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(b.call(op("get_session")).await.unwrap().body.unwrap()["archived_at"].is_string());
    assert_eq!(
        b.call(op("restore_session")).await.unwrap().status_code,
        204
    );
    assert!(a.call(op("get_session")).await.unwrap().body.unwrap()["archived_at"].is_null());
    a.call(op("archive_session")).await.unwrap();
    assert_eq!(b.call(op("delete_session")).await.unwrap().status_code, 204);
    assert_eq!(a.call(op("get_session")).await.unwrap().status_code, 404);
    stop.cancel();
    server.await.unwrap().unwrap();
}
