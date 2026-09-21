use super::*;
use rove_protocol::contract::AGENT;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

async fn call(agent: &Arc<Agent>, request: Request) -> Value {
    let response = agent.handle(&request).await;
    assert_eq!(
        response.status_code, 200,
        "{}: {:?}",
        request.operation_id, response
    );
    AGENT.response(&request.operation_id, &response).unwrap();
    let value = response.body.unwrap();
    assert!(!value.to_string().contains("sync-test-secret"));
    assert!(!value.to_string().contains("sync_source"));
    value
}
fn config() -> Value {
    json!({"provider":"openai_compatible","base_url":"http://127.0.0.1:1/v1","api_key":"sync-test-secret","models":[{"model":"chat","name":"聊天"},{"model":"fast","name":"快速"}],"set_default":false})
}
fn receive(config: Value) -> Request {
    Request::new("receive_model_sync").with_body(
        json!({"source_device_id":Uuid::nil(),"source_connection_id":Uuid::nil(),"config":config}),
    )
}

#[tokio::test]
async fn api_sync_uses_remote_sdk_is_private_idempotent_and_survives_restart() {
    let temp = tempfile::tempdir().unwrap();
    let source = Agent::open(&temp.path().join("source")).unwrap();
    let target_path = temp.path().join("target");
    let target = Agent::open(&target_path).unwrap();
    let network = Uuid::new_v4();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    source.remote_test_routes.lock().unwrap().insert(
        target.store.device_id.0,
        format!("http://{}", listener.local_addr().unwrap())
            .parse()
            .unwrap(),
    );
    let stop = CancellationToken::new();
    let server = tokio::spawn(crate::http::serve_bound(
        target.clone(),
        network,
        listener,
        stop.clone(),
    ));
    let local = call(&source, Request::new("import_models").with_body(config())).await;
    let id = local["connections"][0]["connection_id"].as_str().unwrap();
    let mut request = Request::new("sync_model_connection")
        .with_path("connection_id", id)
        .with_body(json!({"target":{"network_id":network,"device_id":target.store.device_id}}));
    let saved = call(&source, request.clone()).await;
    assert_eq!(saved["models"].as_array().unwrap().len(), 2);
    assert!(saved["default_model_id"].is_null());
    assert_eq!(call(&source, request.clone()).await, saved);
    let target_connection = saved["connections"][0]["connection_id"].clone();
    assert_ne!(target_connection, local["connections"][0]["connection_id"]);
    {
        let db = target.store.connection.lock().unwrap();
        assert_eq!(
            connection_snapshot(&read(&db, "model_catalog").unwrap(), &target_connection).unwrap()
                ["api_key"],
            "sync-test-secret"
        );
    }
    // A local edit on the receiver is not overwritten merely by repeating sync.
    let renamed = call(
        &target,
        Request::new("rename_model")
            .with_path("model_id", saved["models"][0]["model_id"].as_str().unwrap())
            .with_body(json!({"name":"目标修改"})),
    )
    .await;
    assert_eq!(source.handle(&request).await.status_code, 409);
    assert_eq!(
        call(&target, Request::new("get_model_catalog")).await,
        renamed
    );
    call(
        &target,
        Request::new("set_default_model")
            .with_body(json!({"model_id":saved["models"][0]["model_id"]})),
    )
    .await;
    request.body.as_mut().unwrap()["overwrite"] = json!(true);
    let replaced = call(&source, request.clone()).await;
    assert_eq!(replaced["models"], saved["models"]);
    assert_eq!(replaced["default_model_id"], saved["models"][0]["model_id"]);
    assert_eq!(
        call(&source, Request::new("get_model_catalog")).await,
        local
    );
    stop.cancel();
    server.await.unwrap().unwrap();
    target.shutdown().await;
    drop(target);
    source.shutdown().await;
    drop(source);
    let target = Agent::open(&target_path).unwrap();
    assert_eq!(
        call(&target, Request::new("get_model_catalog")).await,
        replaced
    );
    assert_eq!(
        target.store.get("model_config").unwrap().unwrap()["api_key"],
        "sync-test-secret"
    );
    target.shutdown().await;
}

#[tokio::test]
async fn sync_conflicts_replace_atomically_and_reject_account_payloads() {
    let temp = tempfile::tempdir().unwrap();
    let agent = Agent::open(&temp.path().join("agent")).unwrap();
    let original = call(&agent, Request::new("import_models").with_body(config())).await;
    let mut request = receive(config());
    assert_eq!(agent.handle(&request).await.status_code, 409);
    request.body.as_mut().unwrap()["replace_connection_id"] =
        original["connections"][0]["connection_id"].clone();
    assert_eq!(agent.handle(&request).await.status_code, 409);
    assert_eq!(
        call(&agent, Request::new("get_model_catalog")).await,
        original
    );
    request.body.as_mut().unwrap()["overwrite"] = json!(true);
    let replaced = call(&agent, request.clone()).await;
    request
        .body
        .as_mut()
        .unwrap()
        .as_object_mut()
        .unwrap()
        .remove("replace_connection_id");
    request.body.as_mut().unwrap()["overwrite"] = json!(false);
    assert_eq!(call(&agent, request.clone()).await, replaced);
    for key in ["session", "refresh_token", "access_token", "auth_type"] {
        let mut invalid = request.clone();
        invalid.body.as_mut().unwrap()["config"][key] = json!("secret-token");
        assert_eq!(agent.handle(&invalid).await.status_code, 400);
    }
    let mut invalid = request.clone();
    invalid.body.as_mut().unwrap()["config"]["api_key"] = Value::Null;
    assert_eq!(agent.handle(&invalid).await.status_code, 409);
    let mut invalid = request.clone();
    invalid.body.as_mut().unwrap()["config"]["set_default"] = json!(true);
    assert_eq!(agent.handle(&invalid).await.status_code, 409);
    // A failure late in a replacement must not save earlier model changes.
    let mut invalid = request.clone();
    invalid.body.as_mut().unwrap()["overwrite"] = json!(true);
    invalid.body.as_mut().unwrap()["config"]["models"] =
        json!([{"model":"other","name":"new"},{"model":"other","name":"duplicate"}]);
    assert_eq!(agent.handle(&invalid).await.status_code, 400);
    assert_eq!(
        call(&agent, Request::new("get_model_catalog")).await,
        replaced
    );
    agent.shutdown().await;
}

#[tokio::test]
async fn sync_missing_route_never_modifies_source_or_falls_back_locally() {
    let temp = tempfile::tempdir().unwrap();
    let agent = Agent::open(&temp.path().join("source")).unwrap();
    let original = call(&agent, Request::new("import_models").with_body(config())).await;
    let mut request = Request::new("sync_model_connection")
        .with_path(
            "connection_id",
            original["connections"][0]["connection_id"]
                .as_str()
                .unwrap(),
        )
        .with_body(json!({"target":{"network_id":Uuid::new_v4(),"device_id":Uuid::new_v4()}}));
    assert_eq!(agent.handle(&request).await.status_code, 503);
    request.body.as_mut().unwrap()["target"]["device_id"] = json!(agent.store.device_id);
    assert_eq!(agent.handle(&request).await.status_code, 400);
    assert_eq!(
        call(&agent, Request::new("get_model_catalog")).await,
        original
    );
    agent.shutdown().await;
}
