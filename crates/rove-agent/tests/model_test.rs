use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode, Uri},
    response::IntoResponse,
};
use rove_agent::Agent;
use rove_protocol::Request;
use serde_json::{Value, json};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tokio::sync::mpsc;

type ProbeState = (
    mpsc::UnboundedSender<(String, HeaderMap, Value)>,
    Arc<AtomicBool>,
);

#[tokio::test]
async fn probe_uses_native_protocol_without_tools_and_binds_verification_to_private_config() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let (tx, mut rx) = mpsc::unbounded_channel::<(String, HeaderMap, Value)>();
    let fail = Arc::new(AtomicBool::new(false));
    let state = (tx, fail.clone());
    let app=Router::new().fallback(|State((tx,fail)):State<ProbeState>,uri:Uri,headers:HeaderMap,Json(body):Json<Value>|async move{
        tx.send((uri.path().into(),headers,body)).unwrap();
        if fail.load(Ordering::SeqCst){return (StatusCode::UNAUTHORIZED,"provider-echoes-fixture-secret").into_response();}
        let events=if uri.path().ends_with("/messages") {
            [json!({"type":"message_start","message":{"id":"msg-test","type":"message","role":"assistant","model":"fixture","content":[],"stop_reason":null,"stop_sequence":null,"usage":{"input_tokens":1,"output_tokens":0}}}),json!({"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}),json!({"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"OK"}})].into_iter().map(|v|format!("event: {}\ndata: {v}\n\n",v["type"].as_str().unwrap())).collect::<String>()
        } else {
            format!("data: {}\n\ndata: [DONE]\n\n",json!({"id":"test","object":"chat.completion.chunk","created":1,"model":"fixture","choices":[{"index":0,"delta":{"role":"assistant","content":"OK"},"finish_reason":null}]}))
        };
        ([("content-type","text/event-stream")],events).into_response()
    }).with_state(state);
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("agent");
    let agent = Agent::open(&path).unwrap();
    // One local provider fixture exercises both DeepSeek wire protocols and URL normalization.
    for suffix in ["", "/v1", "/anthropic", "/anthropic/v1/"] {
        let config = json!({"provider":"deepseek","base_url":format!("{base}{suffix}"),"api_key":"fixture-secret","model":"fixture"});
        let response = agent
            .handle(&Request::new("test_model").with_body(config.clone()))
            .await;
        assert_eq!(response.status_code, 200, "{response:?}");
        assert_eq!(response.body.unwrap()["available"], true);
        let (route, headers, body) = rx.recv().await.unwrap();
        if suffix.contains("anthropic") {
            assert_eq!(route, "/anthropic/v1/messages");
            assert_eq!(headers["x-api-key"], "fixture-secret");
        } else {
            assert_eq!(route, format!("{suffix}/chat/completions"));
            assert_eq!(headers["authorization"], "Bearer fixture-secret");
        }
        assert!(
            body.get("tools")
                .is_none_or(|v| v.as_array().is_some_and(Vec::is_empty))
        );
        assert_eq!(body["max_tokens"], 256);
        assert_eq!(body["thinking"]["type"], "disabled");
        assert!(!body.to_string().contains("fixture-secret"));
        let mut import = config;
        import.as_object_mut().unwrap().remove("model");
        import["models"] = json!([{"model":"fixture"}]);
        import["set_default"] = json!(true);
        let imported = agent
            .handle(&Request::new("import_models").with_body(import))
            .await;
        assert_eq!(imported.status_code, 200);
        let catalog = imported.body.unwrap();
        let entry = catalog["models"].as_array().unwrap().last().unwrap();
        assert!(entry["tested_at"].is_string());
        assert!(!catalog.to_string().contains("fixture-secret"));
    }
    let catalog = agent
        .handle(&Request::new("get_model_catalog"))
        .await
        .body
        .unwrap();
    let id = catalog["models"][0]["model_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let connection = catalog["models"][0]["connection_id"].as_str().unwrap();
    let changed = agent
        .handle(
            &Request::new("update_model_connection")
                .with_path("connection_id", connection)
                .with_body(
                    json!({"provider":"deepseek","base_url":base,"api_key":"changed-fixture"}),
                ),
        )
        .await
        .body
        .unwrap();
    assert!(changed["models"][0]["tested_at"].is_null());
    let request = Request::new("test_model").with_body(json!({"model_id":id}));
    assert_eq!(agent.handle(&request).await.status_code, 200);
    rx.recv().await.unwrap();
    let before = agent.handle(&Request::new("list_sessions")).await.body;
    fail.store(true, Ordering::SeqCst);
    let failed = agent.handle(&request).await;
    assert_eq!(failed.status_code, 502);
    assert!(!failed.body.unwrap().to_string().contains("fixture-secret"));
    assert_eq!(
        agent.handle(&Request::new("list_sessions")).await.body,
        before
    );
    assert!(
        agent
            .handle(&Request::new("get_model_catalog"))
            .await
            .body
            .unwrap()["models"][0]["tested_at"]
            .is_null()
    );
    agent.shutdown().await;
    drop(agent);
    let agent = Agent::open(&path).unwrap();
    let persisted = agent
        .handle(&Request::new("get_model_catalog"))
        .await
        .body
        .unwrap();
    assert!(persisted["models"][0]["tested_at"].is_null());
    assert!(persisted["models"][1]["tested_at"].is_string());
    // Model-id and raw credentials are mutually exclusive; unexpected fields cannot bypass validation.
    assert_eq!(
        agent
            .handle(
                &Request::new("test_model").with_body(json!({"model_id":id,"api_key":"ignored"}))
            )
            .await
            .status_code,
        400
    );
    agent.shutdown().await;
    server.abort();
}
