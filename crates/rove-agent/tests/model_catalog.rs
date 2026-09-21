use rove_agent::Agent;
use rove_protocol::{Request, contract::AGENT};
use serde_json::{Value, json};
use std::sync::Arc;

async fn call(agent: &Arc<Agent>, request: Request) -> Value {
    let response = agent.handle(&request).await;
    assert_eq!(response.status_code, 200, "{response:?}");
    AGENT.response(&request.operation_id, &response).unwrap();
    response.body.unwrap()
}
fn import() -> Request {
    Request::new("import_models").with_body(json!({"provider":"openai_compatible","base_url":"http://127.0.0.1:1/v1","api_key":"catalog-secret-do-not-echo","models":[{"model":"chat"},{"model":"fast","name":"日常助手"}],"set_default":true}))
}

#[tokio::test]
async fn catalog_is_atomic_private_editable_and_persistent() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("agent");
    let agent = Agent::open(&path).unwrap();
    let device = agent.store.device_id;
    let catalog = call(&agent, import()).await;
    assert!(!catalog.to_string().contains("catalog-secret"));
    assert_eq!(catalog["models"][0]["name"], "openai-compatible-chat-01");
    assert_eq!(catalog["models"][1]["name"], "日常助手");
    let first = catalog["models"][0]["model_id"].as_str().unwrap();
    let second = catalog["models"][1]["model_id"].as_str().unwrap();
    let connection = catalog["connections"][0]["connection_id"].as_str().unwrap();
    assert_eq!(catalog["default_model_id"], first);
    let mut invalid = import();
    invalid.body.as_mut().unwrap()["models"] = json!([{"model":"new"},{"model":"new"}]);
    assert_eq!(agent.handle(&invalid).await.status_code, 400);
    assert_eq!(agent.handle(&import()).await.status_code, 409); // second name conflicts; no partial insert
    assert_eq!(
        call(&agent, Request::new("get_model_catalog")).await,
        catalog
    );
    let renamed = call(
        &agent,
        Request::new("rename_model")
            .with_path("model_id", second)
            .with_body(json!({"name":"我的助手"})),
    )
    .await;
    assert_eq!(renamed["models"][1]["name"], "我的助手");
    call(
        &agent,
        Request::new("set_default_model").with_body(json!({"model_id":second})),
    )
    .await;
    assert_eq!(
        call(&agent, Request::new("get_model_config")).await["config"]["model"],
        "fast"
    );
    let updated=call(&agent,Request::new("update_model_connection").with_path("connection_id",connection).with_body(json!({"provider":"openai_compatible","base_url":"http://127.0.0.1:2/v1","api_key":null}))).await;
    assert_eq!(updated["connections"][0]["api_key_configured"], false);
    assert_eq!(
        call(&agent, Request::new("get_model_config")).await["config"]["base_url"],
        "http://127.0.0.1:2/v1"
    );
    agent.shutdown().await;
    drop(agent);
    let agent = Agent::open(&path).unwrap();
    assert_eq!(agent.store.device_id, device);
    assert_eq!(
        call(&agent, Request::new("get_model_catalog")).await,
        updated
    );
    let removed = call(
        &agent,
        Request::new("delete_model_connection").with_path("connection_id", connection),
    )
    .await;
    assert_eq!(removed["models"], json!([]));
    assert!(removed["default_model_id"].is_null());
    assert_eq!(
        call(&agent, Request::new("get_model_config")).await["configured"],
        false
    );
    agent.shutdown().await;
}

#[tokio::test]
async fn v3_upgrade_preserves_legacy_model_identity_and_private_backup() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("agent");
    let agent = Agent::open(&path).unwrap();
    let device = agent.store.device_id;
    agent.shutdown().await;
    drop(agent);
    let db = rusqlite::Connection::open(path.join("rove.db")).unwrap();
    let config = json!({"provider":"openai_compatible","base_url":"http://127.0.0.1:1/v1","model":"legacy-model","api_key":"legacy-secret"});
    db.execute("INSERT INTO metadata(key,value) VALUES('model_config',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value",[config.to_string()]).unwrap();
    db.execute_batch("DELETE FROM metadata WHERE key='model_catalog'; PRAGMA user_version=3;")
        .unwrap();
    drop(db);
    let agent = Agent::open(&path).unwrap();
    assert_eq!(agent.store.device_id, device);
    let catalog = call(&agent, Request::new("get_model_catalog")).await;
    assert_eq!(catalog["models"][0]["model"], "legacy-model");
    assert!(!catalog.to_string().contains("legacy-secret"));
    assert_eq!(agent.store.get("model_config").unwrap().unwrap(), config);
    assert!(std::fs::read_dir(&path).unwrap().any(|p| {
        p.unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with("rove-before-v4-")
    }));
    let imported = call(&agent, import()).await;
    assert_eq!(imported["models"].as_array().unwrap().len(), 3);
    // Compatibility writes must never remove a modern connection.
    call(&agent, Request::new("set_model_config").with_body(config)).await;
    assert_eq!(
        call(&agent, Request::new("get_model_catalog")).await["models"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    assert_eq!(
        agent
            .handle(&Request::new("clear_model_config"))
            .await
            .status_code,
        204
    );
    assert_eq!(
        call(&agent, Request::new("get_model_catalog")).await["models"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    agent.shutdown().await;
}

#[tokio::test]
async fn first_model_creates_one_durable_onboarding_and_session_choice_survives_restart() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("agent");
    let agent = Agent::open(&path).unwrap();
    let catalog = call(&agent, import()).await;
    let session_id = catalog["onboarding_session_id"].as_str().unwrap();
    let model_id = catalog["models"][1]["model_id"].as_str().unwrap();
    let get = Request::new("get_session").with_path("session_id", session_id);
    let session = call(&agent, get.clone()).await;
    assert_eq!(session["title"], "初始网络的会话");
    assert_eq!(session["kind"], "network_onboarding");
    let updated = call(
        &agent,
        Request::new("update_session")
            .with_path("session_id", session_id)
            .with_body(json!({"model_id":model_id})),
    )
    .await;
    assert_eq!(updated["title"], session["title"]);
    assert_eq!(updated["model_id"], model_id);
    agent.shutdown().await;
    drop(agent);
    let agent = Agent::open(&path).unwrap();
    assert_eq!(call(&agent, get.clone()).await, updated);
    let mut extra = import();
    extra.body.as_mut().unwrap()["models"] = json!([{"model":"new-model"}]);
    assert_eq!(
        call(&agent, extra).await["onboarding_session_id"],
        session_id
    );
    let sessions = call(&agent, Request::new("list_sessions")).await;
    assert_eq!(sessions["items"].as_array().unwrap().len(), 1);
    let connection = catalog["connections"][0]["connection_id"].as_str().unwrap();
    call(
        &agent,
        Request::new("delete_model_connection").with_path("connection_id", connection),
    )
    .await;
    let request = Request::new("submit_run")
        .with_path("session_id", session_id)
        .with_body(json!({"request_id":uuid::Uuid::new_v4(),"message":"do not fall back"}));
    assert_eq!(agent.handle(&request).await.status_code, 422); // selected deleted, other default exists
    assert_eq!(call(&agent, get).await["model_id"], model_id);
    agent.shutdown().await;
}
