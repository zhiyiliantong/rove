#![cfg(unix)]
use serde_json::{Value, json};
use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio_util::sync::CancellationToken;

async fn invoke(
    path: &std::path::Path,
    args: &[&str],
    input: Option<&str>,
) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_rove"))
        .arg("--data-dir")
        .arg(path)
        .arg("--json")
        .args(args)
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    if let Some(input) = input {
        child
            .stdin
            .take()
            .unwrap()
            .write_all(input.as_bytes())
            .await
            .unwrap();
    }
    tokio::time::timeout(std::time::Duration::from_secs(10), child.wait_with_output())
        .await
        .unwrap()
        .unwrap()
}
fn success(output: std::process::Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    serde_json::from_slice(&output.stdout).unwrap()
}

#[tokio::test]
async fn ai_commands_continue_watch_and_cancel_the_same_device_session() {
    use axum::{Router, response::IntoResponse, routing::post};
    use std::{sync::Arc, time::Duration};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}/v1", listener.local_addr().unwrap());
    let gate = Arc::new(tokio::sync::Semaphore::new(1));
    let handler_gate = gate.clone();
    let router = Router::new().route("/v1/chat/completions", post(move || {
        let gate = handler_gate.clone();
        async move {
            let Ok(permit) = gate.acquire().await else { return axum::http::StatusCode::SERVICE_UNAVAILABLE.into_response(); };
            permit.forget();
            let event = json!({"id":"cli-test","object":"chat.completion.chunk","created":1,"model":"test","choices":[{"index":0,"delta":{"role":"assistant","content":"CLI conversation output"},"finish_reason":null}]});
            let end = json!({"id":"cli-test","object":"chat.completion.chunk","created":1,"model":"test","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]});
            ([(axum::http::header::CONTENT_TYPE, "text/event-stream")], format!("data: {event}\n\ndata: {end}\n\ndata: [DONE]\n\n")).into_response()
        }
    }));
    let provider = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("agent");
    let agent = rove_agent::Agent::open(&path).unwrap();
    let socket = rove_sdk::socket_path(&agent.store.data_dir);
    let stop = CancellationToken::new();
    let server = tokio::spawn(rove_agent::serve(agent.clone(), stop.clone()));
    tokio::time::timeout(Duration::from_secs(5), async {
        while !socket.exists() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let model =
        json!({"provider":"openai_compatible","base_url":base,"model":"test","api_key":null});
    success(invoke(&path, &["model", "set"], Some(&model.to_string())).await);
    let session = success(
        invoke(
            &path,
            &["session", "create", "--title", "first title"],
            None,
        )
        .await,
    );
    let id = session["session_id"].as_str().unwrap();
    success(invoke(&path, &["session", "rename", id, "continued title"], None).await);
    assert_eq!(
        success(invoke(&path, &["session", "show", id], None).await)["title"],
        "continued title"
    );
    let request_id = uuid::Uuid::new_v4().to_string();
    let accepted = success(
        invoke(
            &path,
            &["session", "send", id, "-", "--request-id", &request_id],
            Some("CLI first input"),
        )
        .await,
    );
    let run = accepted["run_id"].as_str().unwrap();
    assert_eq!(accepted["device_id"], agent.store.device_id.to_string());
    let watch = invoke(&path, &["run", "watch", run], None).await;
    assert!(
        watch.status.success(),
        "{}",
        String::from_utf8_lossy(&watch.stderr)
    );
    assert!(watch.stderr.is_empty());
    let events: Vec<Value> = String::from_utf8(watch.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert!(
        events
            .iter()
            .any(|event| event.to_string().contains("CLI conversation output"))
    );
    let snapshot = success(invoke(&path, &["run", "show", run], None).await);
    assert_eq!(snapshot["run"]["status"], "succeeded");
    let retry = success(
        invoke(
            &path,
            &["session", "send", id, "-", "--request-id", &request_id],
            Some("CLI first input"),
        )
        .await,
    );
    assert_eq!(retry["run_id"], run);
    let history = success(invoke(&path, &["session", "messages", id], None).await);
    assert!(history.to_string().contains("CLI first input"));
    assert!(history.to_string().contains("CLI conversation output"));
    let continued = success(
        invoke(
            &path,
            &["session", "send", id, "continue from another CLI process"],
            None,
        )
        .await,
    );
    let cancelled = continued["run_id"].as_str().unwrap();
    success(invoke(&path, &["run", "cancel", cancelled], None).await);
    let client = rove_sdk::LocalClient::new(socket);
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let state = client
                .call(rove_protocol::Request::new("get_run").with_path("run_id", cancelled))
                .await
                .unwrap()
                .body
                .unwrap();
            if state["run"]["status"] == "cancelled" {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let listed = success(
        invoke(
            &path,
            &["run", "list", "--session-id", id, "--status", "cancelled"],
            None,
        )
        .await,
    );
    assert_eq!(listed["items"].as_array().unwrap().len(), 1);
    assert_eq!(listed["items"][0]["run_id"], cancelled);
    let sessions = success(invoke(&path, &["session", "list"], None).await);
    assert_eq!(sessions["items"][0]["session_id"], id);
    stop.cancel();
    server.await.unwrap().unwrap();
    agent.shutdown().await;
    gate.close();
    provider.abort();
    let _ = provider.await;
}

#[tokio::test]
async fn command_groups_stdin_pagination_and_target_isolation() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("agent");
    let agent = rove_agent::Agent::open(&path).unwrap();
    let socket = rove_sdk::socket_path(&agent.store.data_dir);
    let stop = CancellationToken::new();
    let server = tokio::spawn(rove_agent::serve(agent, stop.clone()));
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while !socket.exists() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let a = success(invoke(&path, &["network", "create", "one"], None).await);
    success(invoke(&path, &["network", "create", "two"], None).await);
    let id = a["network_id"].as_str().unwrap();
    let page = success(invoke(&path, &["network", "list", "--limit", "1"], None).await);
    assert_eq!(page["items"].as_array().unwrap().len(), 1);
    let next = success(
        invoke(
            &path,
            &[
                "network",
                "list",
                "--limit",
                "1",
                "--cursor",
                page["next_cursor"].as_str().unwrap(),
            ],
            None,
        )
        .await,
    );
    assert_eq!(next["items"].as_array().unwrap().len(), 1);
    assert_ne!(
        page["items"][0]["network_id"],
        next["items"][0]["network_id"]
    );
    assert!(next["next_cursor"].is_null());
    let config = success(invoke(&path, &["network", "export", id], None).await);
    let imported = success(invoke(&path, &["network", "import"], Some(&config.to_string())).await);
    assert_eq!(imported["network"]["instance_id"], a["instance_id"]);
    let update = json!({"display_name":"renamed", "easytier":config["easytier"]});
    success(invoke(&path, &["network", "update", id], Some(&update.to_string())).await);
    assert_eq!(
        success(invoke(&path, &["network", "show", id], None).await)["display_name"],
        "renamed"
    );
    assert!(
        success(invoke(&path, &["device", "list", id], None).await)["items"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        success(invoke(&path, &["service", "list", "--network-id", id], None).await)["items"],
        json!([])
    );
    let publication = json!({"network_id":id,"name":"test service","protocol":"http","target":{"host":"127.0.0.1","port":4533}});
    let unavailable = invoke(
        &path,
        &["service", "publish"],
        Some(&publication.to_string()),
    )
    .await;
    assert!(!unavailable.status.success());
    assert!(String::from_utf8_lossy(&unavailable.stderr).contains("overlay_unavailable"));
    let missing = invoke(
        &path,
        &["service", "open", "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa"],
        None,
    )
    .await;
    assert!(!missing.status.success());
    assert!(missing.stdout.is_empty());
    assert!(String::from_utf8_lossy(&missing.stderr).contains("service_not_found"));
    success(
        invoke(
            &path,
            &[
                "service",
                "unpublish",
                "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
            ],
            None,
        )
        .await,
    );
    let model = json!({"provider":"openai_compatible", "base_url":"http://127.0.0.1:1/v1", "model":"test", "api_key":"test-cli-private-key"});
    for output in [
        invoke(&path, &["model", "set"], Some(&model.to_string())).await,
        invoke(&path, &["model", "show"], None).await,
    ] {
        assert!(!String::from_utf8_lossy(&output.stdout).contains("test-cli-private-key"));
        assert!(!String::from_utf8_lossy(&output.stderr).contains("test-cli-private-key"));
        success(output);
    }
    let session = success(
        invoke(
            &path,
            &["session", "create", "--title", "CLI workflow"],
            None,
        )
        .await,
    );
    let session_id = session["session_id"].as_str().unwrap();
    success(
        invoke(
            &path,
            &["session", "rename", session_id, "renamed conversation"],
            None,
        )
        .await,
    );
    assert_eq!(
        success(invoke(&path, &["session", "show", session_id], None).await)["title"],
        "renamed conversation"
    );
    assert_eq!(
        success(invoke(&path, &["session", "list", "--limit", "1"], None).await)["items"][0]["session_id"],
        session_id
    );
    let request_id = "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb";
    let run = success(
        invoke(
            &path,
            &[
                "session",
                "send",
                session_id,
                "-",
                "--request-id",
                request_id,
            ],
            Some("test submission"),
        )
        .await,
    );
    let run_id = run["run_id"].as_str().unwrap();
    assert_eq!(
        success(
            invoke(
                &path,
                &[
                    "session",
                    "send",
                    session_id,
                    "-",
                    "--request-id",
                    request_id
                ],
                Some("test submission")
            )
            .await
        )["run_id"],
        run_id
    );
    // The intentionally unavailable local model makes a terminal failure, not
    // a successful fake AI run. Following it still exits and emits valid NDJSON.
    let watched = invoke(&path, &["run", "watch", run_id], None).await;
    assert!(
        watched.status.success(),
        "{}",
        String::from_utf8_lossy(&watched.stderr)
    );
    assert!(watched.stderr.is_empty());
    for line in String::from_utf8(watched.stdout).unwrap().lines() {
        serde_json::from_str::<Value>(line).unwrap();
    }
    assert_eq!(
        success(invoke(&path, &["run", "show", run_id], None).await)["run"]["status"],
        "failed"
    );
    assert_eq!(
        success(invoke(&path, &["run", "cancel", run_id], None).await)["status"],
        "failed"
    );
    assert_eq!(
        success(
            invoke(
                &path,
                &[
                    "run",
                    "list",
                    "--session-id",
                    session_id,
                    "--status",
                    "failed",
                    "--limit",
                    "1"
                ],
                None
            )
            .await
        )["items"][0]["run_id"],
        run_id
    );
    assert!(
        !success(
            invoke(
                &path,
                &["session", "messages", session_id, "--limit", "1"],
                None
            )
            .await
        )["items"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    success(invoke(&path, &["agent", "status"], None).await);
    success(invoke(&path, &["model", "clear"], None).await);
    assert_eq!(
        success(
            invoke(
                &path,
                &[
                    "config",
                    "set",
                    "--max-active-runs",
                    "6",
                    "--config-server",
                    "https://config.example.com"
                ],
                None
            )
            .await
        )["max_active_runs"],
        6
    );
    assert!(success(invoke(&path, &["config", "set", "--clear-config-server"], None).await)["config_server_url"].is_null());
    let remote = invoke(
        &path,
        &[
            "--network",
            id,
            "--device",
            "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
            "config",
            "set",
            "--max-active-runs",
            "2",
        ],
        None,
    )
    .await;
    assert!(!remote.status.success() && remote.stdout.is_empty());
    assert!(String::from_utf8_lossy(&remote.stderr).contains("target_unreachable"));
    assert_eq!(
        success(invoke(&path, &["config", "show"], None).await)["max_active_runs"],
        6
    );
    for args in [
        vec!["network", "list", "--limit", "0"],
        vec!["config", "set"],
        vec!["model", "set"],
        vec!["--network", id, "status"],
    ] {
        let output = invoke(&path, &args, None).await;
        assert!(!output.status.success() && output.stdout.is_empty());
    }
    success(invoke(&path, &["network", "stop", id], None).await);
    success(invoke(&path, &["network", "delete", id], None).await);
    stop.cancel();
    server.await.unwrap().unwrap();
}

#[tokio::test]
async fn help_and_non_tty_interaction_do_not_need_a_running_agent() {
    let temp = tempfile::tempdir().unwrap();
    for args in [vec![], vec!["interactive"]] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_rove"));
        command
            .arg("--data-dir")
            .arg(temp.path())
            .args(args)
            .stdin(Stdio::null());
        let output = tokio::time::timeout(std::time::Duration::from_secs(2), command.output())
            .await
            .unwrap()
            .unwrap();
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("needs a terminal"));
    }
    for group in [
        "network", "device", "model", "config", "service", "session", "run",
    ] {
        let output = invoke(temp.path(), &[group, "--help"], None).await;
        assert!(output.status.success());
        assert!(String::from_utf8_lossy(&output.stdout).contains("Usage:"));
    }
}

#[tokio::test]
async fn cli_attaches_reports_json_and_leaves_runtime_alive() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("device");
    let agent = rove_agent::Agent::open(&path).unwrap();
    let socket = rove_sdk::socket_path(&agent.store.data_dir);
    let stop = CancellationToken::new();
    let server = tokio::spawn(rove_agent::serve(agent, stop.clone()));
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while !socket.exists() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let status = || {
        let mut c = Command::new(env!("CARGO_BIN_EXE_rove"));
        c.arg("--data-dir")
            .arg(&path)
            .args(["--json", "status"])
            .stdin(Stdio::null());
        c
    };
    let (a, b) = tokio::join!(status().output(), status().output());
    let a = a.unwrap();
    let b = b.unwrap();
    assert!(a.status.success() && b.status.success());
    assert!(a.stderr.is_empty() && b.stderr.is_empty());
    let a: serde_json::Value = serde_json::from_slice(&a.stdout).unwrap();
    let b: serde_json::Value = serde_json::from_slice(&b.stdout).unwrap();
    assert_eq!(a["device_id"], b["device_id"]);
    assert!(socket.exists());
    let invalid = Command::new(env!("CARGO_BIN_EXE_rove"))
        .arg("--data-dir")
        .arg(&path)
        .args([
            "call",
            "update_settings",
            "--body",
            "{\"max_active_runs\":0}",
        ])
        .output()
        .await
        .unwrap();
    assert!(!invalid.status.success());
    assert!(invalid.stdout.is_empty());
    assert!(!invalid.stderr.is_empty());
    stop.cancel();
    server.await.unwrap().unwrap();
    let offline = status().output().await.unwrap();
    assert!(!offline.status.success());
    assert!(offline.stdout.is_empty());
    assert!(
        String::from_utf8(offline.stderr)
            .unwrap()
            .contains("Cannot connect to rove-agent")
    );
}
