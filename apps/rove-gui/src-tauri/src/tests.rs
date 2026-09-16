use rove_protocol::Request;
use serde_json::{Value, json};
use tauri::Manager;
use tauri::test::{MockRuntime, get_ipc_response, mock_builder};
use tokio_util::sync::CancellationToken;

fn invoke(
    window: &tauri::WebviewWindow<MockRuntime>,
    cmd: &str,
    body: Value,
) -> Result<Value, Value> {
    invoke_from(window, cmd, body, "tauri://localhost")
}
fn invoke_from(
    window: &tauri::WebviewWindow<MockRuntime>,
    cmd: &str,
    body: Value,
    origin: &str,
) -> Result<Value, Value> {
    get_ipc_response(
        window,
        tauri::webview::InvokeRequest {
            cmd: cmd.into(),
            callback: tauri::ipc::CallbackFn(0),
            error: tauri::ipc::CallbackFn(1),
            url: origin.parse().unwrap(),
            body: tauri::ipc::InvokeBody::Json(body),
            headers: Default::default(),
            invoke_key: tauri::test::INVOKE_KEY.to_string(),
        },
    )
    .map(|body| body.deserialize().unwrap())
}

#[test]
fn windows_run_independent_sessions_and_closing_them_does_not_cancel_work() {
    use axum::{Router, routing::post};
    use std::{sync::Arc, time::Duration};
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let gate = Arc::new(tokio::sync::Semaphore::new(0));
    let (entered_tx, mut entered_rx) = tokio::sync::mpsc::unbounded_channel();
    let (base, provider) = runtime.block_on(async {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}/v1", listener.local_addr().unwrap());
        let gate = gate.clone();
        let app = Router::new().route("/v1/chat/completions", post(move || {
            let gate = gate.clone(); let entered = entered_tx.clone();
            async move {
                let _ = entered.send(());
                let permit = gate.acquire().await.unwrap(); permit.forget();
                let delta = json!({"id":"window-test","object":"chat.completion.chunk","created":1,"model":"test","choices":[{"index":0,"delta":{"role":"assistant","content":"independent window output"},"finish_reason":null}]});
                let end = json!({"id":"window-test","object":"chat.completion.chunk","created":1,"model":"test","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]});
                ([(axum::http::header::CONTENT_TYPE,"text/event-stream")],format!("data: {delta}\n\ndata: {end}\n\ndata: [DONE]\n\n"))
            }
        }));
        (base, tokio::spawn(async move { axum::serve(listener, app).await.unwrap(); }))
    });
    let temp = tempfile::tempdir().unwrap();
    let agent = {
        let _entered = runtime.enter();
        rove_agent::Agent::open(&temp.path().join("device")).unwrap()
    };
    let socket = rove_sdk::socket_path(&agent.store.data_dir);
    let stop = CancellationToken::new();
    let server = runtime.spawn(rove_agent::serve(agent.clone(), stop.clone()));
    runtime.block_on(async {
        tokio::time::timeout(Duration::from_secs(5), async {
            while !socket.exists() {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
    });
    let client = rove_sdk::LocalClient::new(&socket);
    let app = mock_builder()
        .manage(client.clone())
        .invoke_handler(tauri::generate_handler![
            super::agent_call,
            super::agent_events
        ])
        .build(tauri::generate_context!())
        .unwrap();
    let first = tauri::WebviewWindowBuilder::new(&app, "parallel-first", Default::default())
        .build()
        .unwrap();
    let second = tauri::WebviewWindowBuilder::new(&app, "parallel-second", Default::default())
        .build()
        .unwrap();
    let call = |window: &tauri::WebviewWindow<MockRuntime>, request: Request| {
        invoke(window, "agent_call", json!({"request":request})).unwrap()
    };
    assert_eq!(call(&first,Request::new("set_model_config").with_body(json!({"provider":"openai_compatible","base_url":base,"model":"test","api_key":null})))["status_code"],200);
    let mut runs = Vec::new();
    for window in [&first, &second] {
        let session = call(window, Request::new("create_session").with_body(json!({})));
        let session_id = session["body"]["session_id"].as_str().unwrap();
        let accepted = call(
            window,
            Request::new("submit_run")
                .with_path("session_id", session_id)
                .with_body(
                    json!({"request_id":uuid::Uuid::new_v4(),"message":"independent conversation"}),
                ),
        );
        assert_eq!(accepted["status_code"], 202);
        runs.push(accepted["body"]["run_id"].as_str().unwrap().to_owned());
        // Both model requests must enter before any response is released.
        runtime.block_on(async {
            tokio::time::timeout(Duration::from_secs(5), entered_rx.recv())
                .await
                .unwrap()
                .unwrap();
        });
    }
    for run in &runs {
        assert_eq!(
            call(&second, Request::new("get_run").with_path("run_id", run))["body"]["run"]["status"],
            "running"
        );
    }
    let events = invoke(
        &second,
        "agent_events",
        json!({"runId":runs[1],"afterSeq":0,"target":null}),
    )
    .unwrap();
    assert!(events["last_seq"].as_i64().unwrap() > 0);
    assert_eq!(
        call(
            &first,
            Request::new("cancel_run").with_path("run_id", &runs[0])
        )["status_code"],
        202
    );
    first.close().unwrap();
    second.close().unwrap();
    drop(app);
    gate.add_permits(2);
    runtime.block_on(async {
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let a = client
                    .call(Request::new("get_run").with_path("run_id", &runs[0]))
                    .await
                    .unwrap()
                    .body
                    .unwrap();
                let b = client
                    .call(Request::new("get_run").with_path("run_id", &runs[1]))
                    .await
                    .unwrap()
                    .body
                    .unwrap();
                if a["run"]["status"] == "cancelled" && b["run"]["status"] == "succeeded" {
                    assert!(
                        b["output_tail"]
                            .as_str()
                            .unwrap()
                            .contains("independent window output")
                    );
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        stop.cancel();
        server.await.unwrap().unwrap();
        agent.shutdown().await;
        provider.abort();
        let _ = provider.await;
    });
}

#[test]
fn ipc_commands_share_the_sdk_and_preserve_targets_and_agent_lifetime() {
    // IPC waits synchronously, so keep the real socket agent on runtime workers.
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let agent = {
        let _entered = runtime.enter();
        rove_agent::Agent::open(&temp.path().join("device")).unwrap()
    };
    let socket = rove_sdk::socket_path(&agent.store.data_dir);
    let stop = CancellationToken::new();
    let server = runtime.spawn(rove_agent::serve(agent.clone(), stop.clone()));
    runtime.block_on(async {
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            while !socket.exists() {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
    });
    let client = rove_sdk::LocalClient::new(&socket);
    let app = mock_builder()
        .plugin(
            tauri_plugin_opener::Builder::new()
                .open_js_links_on_click(false)
                .build(),
        )
        .manage(client.clone())
        .invoke_handler(tauri::generate_handler![
            super::agent_call,
            super::share_qr,
            super::agent_events
        ])
        .build(tauri::generate_context!())
        .unwrap();
    let first = tauri::WebviewWindowBuilder::new(&app, "first", Default::default())
        .build()
        .unwrap();
    let second = tauri::WebviewWindowBuilder::new(&app, "second", Default::default())
        .build()
        .unwrap();
    let main = app.get_webview_window("main").unwrap_or_else(|| {
        tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .unwrap()
    });
    // Exercise the production capability file, without opening an OS app.
    for body in [
        json!({"url":"file:///tmp/forbidden","with":null}),
        json!({"url":"javascript:alert(1)","with":null}),
        json!({"url":"http://127.0.0.1:1","with":"sh"}),
    ] {
        let rejected = invoke(&main, "plugin:opener|open_url", body).unwrap_err();
        assert!(
            rejected
                .as_str()
                .unwrap()
                .contains("Not allowed to open url"),
            "{rejected}"
        );
    }
    assert!(
        invoke(
            &main,
            "plugin:opener|open_path",
            json!({"path":"/tmp/forbidden","with":null})
        )
        .is_err()
    );
    let call = |window: &tauri::WebviewWindow<MockRuntime>, request: Request| {
        invoke(window, "agent_call", json!({"request":request})).unwrap()
    };
    let device = call(&first, Request::new("get_device"));
    assert!(
        invoke_from(
            &first,
            "agent_call",
            json!({"request":Request::new("get_device")}),
            "https://untrusted.invalid"
        )
        .is_err()
    );
    assert_eq!(
        device["body"]["device_id"],
        agent.store.device_id.to_string()
    );
    call(
        &first,
        Request::new("update_settings").with_body(json!({"max_active_runs":7})),
    );
    assert_eq!(
        call(&second, Request::new("get_settings"))["body"]["max_active_runs"],
        7
    );
    assert!(
        invoke(
            &first,
            "agent_call",
            json!({"request":Request::new("no_such_operation")})
        )
        .is_err()
    );
    let mut remote =
        Request::new("create_session").with_body(json!({"title":"must not run locally"}));
    remote.target = Some(rove_protocol::SocketTarget {
        network_id: uuid::Uuid::new_v4(),
        device_id: uuid::Uuid::new_v4(),
    });
    assert_eq!(call(&first, remote.clone())["status_code"], 503);
    assert_eq!(
        call(&second, Request::new("list_sessions"))["body"]["items"],
        json!([])
    );
    let session = call(
        &first,
        Request::new("create_session").with_body(json!({"title":"shared"})),
    );
    let session_id = session["body"]["session_id"].as_str().unwrap();
    assert_eq!(call(&first, Request::new("set_model_config").with_body(json!({"provider":"openai_compatible","base_url":"http://127.0.0.1:1/v1","model":"test","api_key":null})))["status_code"], 200);
    let run = call(
        &second,
        Request::new("submit_run")
            .with_path("session_id", session_id)
            .with_body(json!({"request_id":uuid::Uuid::new_v4(),"message":"hello"})),
    );
    assert_eq!(run["status_code"], 202);
    let run_id = run["body"]["run_id"].as_str().unwrap();
    // camelCase matches the real Vue invoke payload. Only a container-local,
    // unavailable provider is configured; no external model call is possible.
    let events = invoke(
        &first,
        "agent_events",
        json!({"runId":run_id,"afterSeq":0,"target":null}),
    )
    .unwrap();
    assert!(!events["events"].as_array().unwrap().is_empty());
    assert!(events["last_seq"].as_i64().unwrap() > 0);
    // The unavailable provider can fail before cancellation, or cancellation
    // can win while the model request is active. Both responses are contractual.
    let cancelled = call(
        &second,
        Request::new("cancel_run").with_path("run_id", run_id),
    );
    assert!(matches!(cancelled["status_code"].as_u64(), Some(200 | 202)));
    runtime.block_on(async {
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                let snapshot = client
                    .call(Request::new("get_run").with_path("run_id", run_id))
                    .await
                    .unwrap()
                    .body
                    .unwrap();
                if matches!(
                    snapshot["run"]["status"].as_str(),
                    Some("failed" | "cancelled")
                ) {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
    });
    assert!(
        invoke(
            &first,
            "agent_events",
            json!({"runId":run_id,"afterSeq":-1,"target":null})
        )
        .is_err()
    );
    assert!(
        invoke(
            &first,
            "agent_events",
            json!({"runId":run_id,"afterSeq":0,"target":remote.target})
        )
        .is_err()
    );
    let url = format!(
        "https://example.invalid/c/{}#key={}",
        "a".repeat(32),
        "b".repeat(43)
    );
    let svg = invoke(&first, "share_qr", json!({"url":url})).unwrap();
    assert!(svg.as_str().unwrap().starts_with("<svg"));
    assert!(!svg.as_str().unwrap().contains("example.invalid"));
    first.close().unwrap();
    second.close().unwrap();
    main.close().unwrap();
    drop(app);
    let settings = runtime
        .block_on(client.call(Request::new("get_settings")))
        .unwrap();
    assert_eq!(settings.body.unwrap()["max_active_runs"], 7);
    stop.cancel();
    runtime.block_on(server).unwrap().unwrap();
    runtime.block_on(agent.shutdown());
}
