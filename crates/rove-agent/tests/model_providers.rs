use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode, Uri},
    routing::get,
};
use rove_agent::Agent;
use rove_protocol::Request;
use serde_json::{Value, json};
use std::time::Duration;
use tokio::sync::mpsc;

#[tokio::test]
async fn anthropic_gemini_and_ollama_stream_into_real_run_history() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let app=Router::new().fallback(|uri:Uri|async move {
        let events=if uri.path()=="/v1/messages" {
            vec![
                json!({"type":"message_start","message":{"id":"msg-test","type":"message","role":"assistant","model":"adapter-fixture","content":[],"stop_reason":null,"stop_sequence":null,"usage":{"input_tokens":1,"output_tokens":0}}}),
                json!({"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}),
                json!({"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"native output"}}),
                json!({"type":"content_block_stop","index":0}),
                json!({"type":"message_delta","delta":{"stop_reason":"end_turn","stop_sequence":null},"usage":{"output_tokens":2}}),
                json!({"type":"message_stop"})
            ].into_iter().map(|v|format!("event: {}\ndata: {v}\n\n",v["type"].as_str().unwrap())).collect::<String>()
        } else if uri.path()=="/api/chat" {
            vec![json!({"model":"adapter-fixture","created_at":"2026-09-17T00:00:00Z","message":{"role":"assistant","content":"native output"},"done":false}),json!({"model":"adapter-fixture","created_at":"2026-09-17T00:00:01Z","message":{"role":"assistant","content":""},"done":true,"done_reason":"stop","prompt_eval_count":1,"eval_count":2})].into_iter().map(|v|format!("{v}\n")).collect()
        } else {
            assert!(uri.path().contains("streamGenerateContent"));
            format!("data: {}\n\n",json!({"candidates":[{"content":{"role":"model","parts":[{"text":"native output"}]},"finishReason":"STOP","index":0}],"usageMetadata":{"promptTokenCount":1,"candidatesTokenCount":2,"totalTokenCount":3},"modelVersion":"adapter-fixture"}))
        };
        ([(axum::http::header::CONTENT_TYPE,"text/event-stream")],events)
    });
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let temp = tempfile::tempdir().unwrap();
    let agent = Agent::open(&temp.path().join("agent")).unwrap();
    for provider in ["anthropic", "gemini", "ollama"] {
        assert_eq!(agent.handle(&Request::new("set_model_config").with_body(json!({"provider":provider,"base_url":base,"model":"adapter-fixture","api_key":"test-key"}))).await.status_code,200);
        let session = agent
            .handle(&Request::new("create_session").with_body(json!({})))
            .await
            .body
            .unwrap();
        let run = agent
            .handle(
                &Request::new("submit_run")
                    .with_path("session_id", session["session_id"].as_str().unwrap())
                    .with_body(json!({"request_id":uuid::Uuid::new_v4(),"message":"stream"})),
            )
            .await
            .body
            .unwrap();
        tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                let snapshot = agent
                    .handle(
                        &Request::new("get_run")
                            .with_path("run_id", run["run_id"].as_str().unwrap()),
                    )
                    .await
                    .body
                    .unwrap();
                assert_ne!(
                    snapshot["run"]["status"], "failed",
                    "{provider}: {snapshot}"
                );
                if snapshot["run"]["status"] == "succeeded" {
                    assert_eq!(snapshot["output_tail"], "native output");
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        let messages = agent
            .handle(
                &Request::new("list_messages")
                    .with_path("session_id", session["session_id"].as_str().unwrap()),
            )
            .await
            .body
            .unwrap();
        assert!(messages.to_string().contains("native output"));
    }
    agent.shutdown().await;
    server.abort();
}

#[tokio::test]
async fn all_rig_adapters_send_native_requests_and_redact_provider_failures() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let (tx, mut rx) = mpsc::unbounded_channel();
    let app = Router::new()
        .fallback(
            |State(tx): State<mpsc::UnboundedSender<(Uri, HeaderMap, Value)>>,
             uri: Uri,
             headers: HeaderMap,
             Json(body): Json<Value>| async move {
                tx.send((uri, headers, body)).unwrap();
                (
                    StatusCode::UNAUTHORIZED,
                    Json(json!({"error":{"message":"provider-secret-do-not-echo"}})),
                )
            },
        )
        .with_state(tx);
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let temp = tempfile::tempdir().unwrap();
    let agent = Agent::open(&temp.path().join("agent")).unwrap();
    for provider in [
        "openai",
        "openai_compatible",
        "deepseek",
        "anthropic",
        "gemini",
        "moonshot",
        "zai",
        "minimax",
        "mistral",
        "xai",
        "openrouter",
        "groq",
        "ollama",
    ] {
        let url = if matches!(provider, "anthropic" | "gemini" | "ollama") {
            base.clone()
        } else {
            format!("{base}/v1")
        };
        let configured=agent.handle(&Request::new("set_model_config").with_body(json!({"provider":provider,"base_url":url,"model":"adapter-fixture","api_key":"provider-secret-do-not-echo"}))).await;
        assert_eq!(configured.status_code, 200);
        let session = agent
            .handle(&Request::new("create_session").with_body(json!({})))
            .await
            .body
            .unwrap();
        let accepted = agent
            .handle(
                &Request::new("submit_run")
                    .with_path("session_id", session["session_id"].as_str().unwrap())
                    .with_body(json!({"request_id":uuid::Uuid::new_v4(),"message":"inspect only"})),
            )
            .await;
        assert_eq!(accepted.status_code, 202);
        let run = accepted.body.unwrap();
        let (uri, headers, body) = tokio::time::timeout(Duration::from_secs(10), rx.recv())
            .await
            .unwrap_or_else(|_| panic!("adapter {provider} did not send"))
            .unwrap();
        if provider == "anthropic" {
            assert_eq!(uri.path(), "/v1/messages");
            assert_eq!(headers["x-api-key"], "provider-secret-do-not-echo");
        } else if provider == "gemini" {
            assert!(uri.path().contains("streamGenerateContent"));
            assert!(body["contents"].is_array());
        } else if provider == "ollama" {
            assert_eq!(uri.path(), "/api/chat");
        } else if provider == "xai" {
            assert_eq!(uri.path(), "/v1/responses");
        } else {
            assert_eq!(uri.path(), "/v1/chat/completions", "{provider}");
            assert_eq!(
                headers["authorization"],
                "Bearer provider-secret-do-not-echo"
            );
        }
        assert!(body["tools"].is_array(), "{provider}");
        tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                let snapshot = agent
                    .handle(
                        &Request::new("get_run")
                            .with_path("run_id", run["run_id"].as_str().unwrap()),
                    )
                    .await
                    .body
                    .unwrap();
                if snapshot["run"]["status"] == "failed" {
                    assert_eq!(snapshot["run"]["error"]["code"], "model_provider_failed");
                    assert!(!snapshot.to_string().contains("provider-secret"));
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
    }
    agent.shutdown().await;
    server.abort();
}

async fn list_models(uri: Uri, headers: HeaderMap) -> Json<Value> {
    match uri.path() {
        "/v1/models" if headers.contains_key("x-api-key") => {
            assert_eq!(headers["x-api-key"], "listing-secret");
            assert_eq!(headers["anthropic-version"], "2023-06-01");
            if uri.query().is_some() {
                Json(json!({"data":[{"id":"chat-b"}],"has_more":false}))
            } else {
                Json(json!({"data":[{"id":"chat-a"}],"has_more":true,"last_id":"chat-a"}))
            }
        }
        "/v1/models" => {
            assert_eq!(headers["authorization"], "Bearer listing-secret");
            Json(json!({"data":[{"id":"chat-a"},{"id":"chat-a"},{"id":"chat-b"}]}))
        }
        "/v1beta/models" => {
            assert_eq!(headers["x-goog-api-key"], "listing-secret");
            if uri.query().is_some() {
                Json(json!({"models":[{"name":"models/chat-b"}]}))
            } else {
                Json(json!({"models":[{"name":"models/chat-a"}],"nextPageToken":"next"}))
            }
        }
        "/api/tags" => Json(json!({"models":[{"name":"chat-a"},{"name":"chat-b"}]})),
        _ => panic!("unexpected discovery path"),
    }
}

#[tokio::test]
async fn discovery_uses_native_auth_pagination_limits_and_never_saves_keys() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let app = Router::new()
        .route(
            "/large/models",
            get(|| async { "x".repeat(1024 * 1024 + 1) }),
        )
        .route(
            "/redirect/models",
            get(|| async { (StatusCode::TEMPORARY_REDIRECT, [("location", "/v1/models")]) }),
        )
        .fallback(list_models);
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let temp = tempfile::tempdir().unwrap();
    let agent = Agent::open(&temp.path().join("agent")).unwrap();
    let before = agent.store.get("model_catalog").unwrap();
    for provider in ["openai", "anthropic", "gemini", "ollama"] {
        let url = if provider == "openai" {
            format!("{base}/v1")
        } else {
            base.clone()
        };
        let response =
            agent
                .handle(&Request::new("discover_models").with_body(
                    json!({"provider":provider,"base_url":url,"api_key":"listing-secret"}),
                ))
                .await;
        assert_eq!(response.status_code, 200, "{provider}: {response:?}");
        assert_eq!(
            response.body.unwrap(),
            json!({"models":["chat-a","chat-b"],"truncated":false})
        );
    }
    for (path, status) in [("large", 413), ("redirect", 502)] {
        let response=agent.handle(&Request::new("discover_models").with_body(json!({"provider":"openai","base_url":format!("{base}/{path}"),"api_key":"listing-secret"}))).await;
        assert_eq!(response.status_code, status);
        assert!(
            !response
                .body
                .unwrap()
                .to_string()
                .contains("listing-secret")
        );
    }
    assert_eq!(agent.store.get("model_catalog").unwrap(), before);
    agent.shutdown().await;
    server.abort();
}
