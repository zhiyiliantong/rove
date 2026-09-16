use axum::{Json, Router, extract::State, http::header, response::IntoResponse, routing::post};
use rove_agent::Agent;
use rove_protocol::Request;
use serde_json::{Value, json};
use std::{sync::Arc, time::Duration};
use tokio::sync::{Semaphore, mpsc};
use uuid::Uuid;

struct Fake {
    entered: mpsc::UnboundedSender<Value>,
    gate: Arc<Semaphore>,
    tools: bool,
}
async fn completion(
    State(fake): State<Arc<Fake>>,
    Json(body): Json<Value>,
) -> axum::response::Response {
    fake.entered.send(body.clone()).unwrap();
    fake.gate.acquire().await.unwrap().forget();
    if body["messages"].as_array().unwrap().last().unwrap()["content"]
        .to_string()
        .contains("fail deliberately")
    {
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error":{"message":"Controlled fake provider failure"}})),
        )
            .into_response();
    }
    let answered = body["messages"]
        .as_array()
        .unwrap()
        .iter()
        .any(|m| m["role"] == "tool");
    let tools = fake.tools && !answered;
    let delta = if tools {
        json!({"role":"assistant","tool_calls":[
            {"index":0,"id":"call_first","type":"function","function":{"name":"system_exec","arguments":json!({"command":"printf first; sleep 1"}).to_string()}},
            {"index":1,"id":"call_second","type":"function","function":{"name":"system_exec","arguments":json!({"command":"printf second"}).to_string()}}
        ]})
    } else {
        json!({"role":"assistant","content":"model output"})
    };
    let first = json!({"id":"chatcmpl-test","object":"chat.completion.chunk","created":1,"model":"test","choices":[{"index":0,"delta":delta,"finish_reason":null}]});
    let last = json!({"id":"chatcmpl-test","object":"chat.completion.chunk","created":1,"model":"test","choices":[{"index":0,"delta":{},"finish_reason":if tools{"tool_calls"}else{"stop"}}]});
    (
        [(header::CONTENT_TYPE, "text/event-stream")],
        format!("data: {first}\n\ndata: {last}\n\ndata: [DONE]\n\n"),
    )
        .into_response()
}
async fn fake(
    tools: bool,
) -> (
    String,
    Arc<Semaphore>,
    mpsc::UnboundedReceiver<Value>,
    tokio::task::JoinHandle<()>,
) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}/v1", listener.local_addr().unwrap());
    let (tx, rx) = mpsc::unbounded_channel();
    let gate = Arc::new(Semaphore::new(0));
    let app = Router::new()
        .route("/v1/chat/completions", post(completion))
        .route(
            "/v1/blobs",
            post(
                |State(fake): State<Arc<Fake>>, Json(body): Json<Value>| async move {
                    fake.entered.send(body).unwrap();
                    fake.gate.acquire().await.unwrap().forget();
                    axum::http::StatusCode::SERVICE_UNAVAILABLE
                },
            ),
        )
        .with_state(Arc::new(Fake {
            entered: tx,
            gate: gate.clone(),
            tools,
        }));
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (base, gate, rx, server)
}
async fn setup(path: &std::path::Path, base: &str) -> Arc<Agent> {
    let agent = Agent::open(path).unwrap();
    let response = agent
        .handle(&Request::new("set_model_config").with_body(
            json!({"provider":"openai_compatible","base_url":base,"model":"test","api_key":null}),
        ))
        .await;
    assert_eq!(response.status_code, 200);
    agent
}
async fn session(agent: &Arc<Agent>) -> String {
    agent
        .handle(&Request::new("create_session").with_body(json!({})))
        .await
        .body
        .unwrap()["session_id"]
        .as_str()
        .unwrap()
        .to_string()
}
async fn submit(agent: &Arc<Agent>, session: &str, message: &str) -> (Request, Value) {
    let request = Request::new("submit_run")
        .with_path("session_id", session)
        .with_body(json!({"request_id":Uuid::new_v4(),"message":message}));
    let response = agent.handle(&request).await;
    assert_eq!(response.status_code, 202, "{:?}", response);
    (request, response.body.unwrap())
}
async fn snapshot(agent: &Arc<Agent>, run: &Value) -> Value {
    agent
        .handle(&Request::new("get_run").with_path("run_id", run["run_id"].as_str().unwrap()))
        .await
        .body
        .unwrap()
}
async fn wait_status(agent: &Arc<Agent>, run: &Value, status: &str) -> Value {
    tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            let view = snapshot(agent, run).await;
            if view["run"]["status"] == status {
                return view;
            }
            assert!(
                !matches!(
                    view["run"]["status"].as_str(),
                    Some("failed" | "interrupted")
                ),
                "unexpected run: {view}"
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap()
}
async fn entered(rx: &mut mpsc::UnboundedReceiver<Value>) -> Value {
    tokio::time::timeout(Duration::from_secs(15), rx.recv())
        .await
        .unwrap()
        .unwrap()
}

#[tokio::test]
#[cfg(unix)]
async fn another_sdk_client_continues_persisted_session_after_disconnect() {
    let temp = tempfile::tempdir().unwrap();
    let (base, gate, mut rx, provider) = fake(false).await;
    let agent = setup(&temp.path().join("agent"), &base).await;
    let socket = rove_sdk::socket_path(&agent.store.data_dir);
    let stop = tokio_util::sync::CancellationToken::new();
    let server = tokio::spawn(rove_agent::serve(agent.clone(), stop.clone()));
    tokio::time::timeout(Duration::from_secs(5), async {
        while !socket.exists() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let first = rove_sdk::LocalClient::new(&socket);
    let created = first
        .call(Request::new("create_session").with_body(json!({"title":"roaming conversation"})))
        .await
        .unwrap();
    assert_eq!(created.status_code, 201);
    let created = created.body.unwrap();
    let session_id = created["session_id"].as_str().unwrap();
    assert_eq!(created["device_id"], agent.store.device_id.to_string());
    let accepted = first
        .call(
            Request::new("submit_run")
                .with_path("session_id", session_id)
                .with_body(json!({"request_id":Uuid::new_v4(),"message":"first client input"})),
        )
        .await
        .unwrap();
    assert_eq!(accepted.status_code, 202);
    let run = accepted.body.unwrap();
    entered(&mut rx).await;
    drop(first);
    gate.add_permits(1);
    wait_status(&agent, &run, "succeeded").await;

    let second = rove_sdk::LocalClient::new(&socket);
    let sessions = second
        .call(Request::new("list_sessions"))
        .await
        .unwrap()
        .body
        .unwrap();
    assert!(
        sessions["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["session_id"] == session_id)
    );
    let history = second
        .call(Request::new("list_messages").with_path("session_id", session_id))
        .await
        .unwrap()
        .body
        .unwrap();
    assert!(history.to_string().contains("first client input"));
    assert!(history.to_string().contains("model output"));
    let continued = second
        .call(
            Request::new("submit_run")
                .with_path("session_id", session_id)
                .with_body(json!({"request_id":Uuid::new_v4(),"message":"second client input"})),
        )
        .await
        .unwrap();
    assert_eq!(continued.status_code, 202);
    let continued = continued.body.unwrap();
    assert_ne!(continued["run_id"], run["run_id"]);
    assert_eq!(continued["session_id"], session_id);
    let context = entered(&mut rx).await;
    let messages = context["messages"].as_array().unwrap();
    let first_input = messages
        .iter()
        .position(|m| m["content"].to_string().contains("first client input"))
        .unwrap();
    let answer = messages
        .iter()
        .position(|m| m["role"] == "assistant" && m["content"].to_string().contains("model output"))
        .unwrap();
    let second_input = messages
        .iter()
        .position(|m| m["content"].to_string().contains("second client input"))
        .unwrap();
    assert!(first_input < answer && answer < second_input);
    gate.add_permits(1);
    wait_status(&agent, &continued, "succeeded").await;
    stop.cancel();
    server.await.unwrap().unwrap();
    agent.shutdown().await;
    drop(agent);
    let reopened = Agent::open(&temp.path().join("agent")).unwrap();
    let persisted = reopened
        .handle(&Request::new("list_messages").with_path("session_id", session_id))
        .await
        .body
        .unwrap();
    assert!(persisted.to_string().contains("first client input"));
    assert!(persisted.to_string().contains("second client input"));
    reopened.shutdown().await;
    provider.abort();
    let _ = provider.await;
}

#[tokio::test]
async fn fifo_parallel_capacity_history_and_persistent_dedup() {
    let temp = tempfile::tempdir().unwrap();
    let (base, gate, mut rx, server) = fake(false).await;
    let path = temp.path().join("agent");
    let agent = setup(&path, &base).await;
    let first_session = session(&agent).await;
    let (req, first) = submit(&agent, &first_session, "first input").await;
    let initial = entered(&mut rx).await;
    assert!(!initial.to_string().contains("future input"));
    let (_, future) = submit(&agent, &first_session, "future input").await;
    let mut other = Vec::new();
    for i in 0..4 {
        let s = session(&agent).await;
        other.push(submit(&agent, &s, &format!("independent {i}")).await.1);
    }
    for _ in 0..3 {
        entered(&mut rx).await;
    }
    assert!(
        tokio::time::timeout(Duration::from_millis(100), rx.recv())
            .await
            .is_err()
    );
    assert_eq!(snapshot(&agent, &future).await["run"]["status"], "queued");
    assert_eq!(snapshot(&agent, &other[3]).await["run"]["status"], "queued");
    let history = agent
        .handle(&Request::new("list_messages").with_path("session_id", &first_session))
        .await
        .body
        .unwrap();
    assert!(!history.to_string().contains("future input"));
    let replay = agent.handle(&req).await;
    assert_eq!(replay.status_code, 200);
    assert_eq!(replay.body.unwrap()["run_id"], first["run_id"]);
    let mut conflict = req.clone();
    conflict.body.as_mut().unwrap()["message"] = json!("changed");
    assert_eq!(agent.handle(&conflict).await.status_code, 409);
    agent
        .handle(&Request::new("update_settings").with_body(json!({"max_active_runs":2})))
        .await;
    gate.add_permits(20);
    wait_status(&agent, &first, "succeeded").await;
    wait_status(&agent, &future, "succeeded").await;
    for run in &other {
        wait_status(&agent, run, "succeeded").await;
    }
    let mut saw_future = false;
    while let Ok(input) = rx.try_recv() {
        if input.to_string().contains("future input") {
            saw_future = true;
            assert!(input.to_string().contains("model output"));
        }
    }
    assert!(saw_future);
    agent.shutdown().await;
    drop(agent);
    let reopened = Agent::open(&path).unwrap();
    reopened.handle(&Request::new("clear_model_config")).await;
    let repeated = reopened.handle(&req).await;
    assert_eq!(repeated.status_code, 200);
    assert_eq!(repeated.body.unwrap()["run_id"], first["run_id"]);
    reopened.shutdown().await;
    server.abort();
}

#[tokio::test]
async fn cancel_isolated_queued_input_and_restart_interruption() {
    let temp = tempfile::tempdir().unwrap();
    let (base, gate, mut rx, server) = fake(false).await;
    let path = temp.path().join("agent");
    let agent = setup(&path, &base).await;
    let s1 = session(&agent).await;
    let (_, a) = submit(&agent, &s1, "blocked a").await;
    entered(&mut rx).await;
    let (_, queued) = submit(&agent, &s1, "never executed").await;
    let s2 = session(&agent).await;
    let (_, b) = submit(&agent, &s2, "blocked b").await;
    entered(&mut rx).await;
    let cancel = |run: &Value| {
        Request::new("cancel_run").with_path("run_id", run["run_id"].as_str().unwrap())
    };
    assert_eq!(
        agent.handle(&cancel(&queued)).await.body.unwrap()["status"],
        "cancelled"
    );
    assert_eq!(agent.handle(&cancel(&a)).await.status_code, 202);
    wait_status(&agent, &a, "cancelled").await;
    assert_eq!(snapshot(&agent, &b).await["run"]["status"], "running");
    let (_, pending) = submit(&agent, &s2, "pending after restart").await;
    agent.shutdown().await;
    drop(agent);
    let reopened = Agent::open(&path).unwrap();
    assert_eq!(
        snapshot(&reopened, &pending).await["run"]["status"],
        "interrupted"
    );
    assert_eq!(
        snapshot(&reopened, &b).await["run"]["status"],
        "interrupted"
    );
    assert!(
        tokio::time::timeout(Duration::from_millis(100), rx.recv())
            .await
            .is_err()
    );
    reopened.shutdown().await;
    gate.add_permits(20);
    server.abort();
}

#[cfg(unix)]
#[tokio::test]
async fn rig_tools_are_serial_per_run_and_parallel_between_runs() {
    let temp = tempfile::tempdir().unwrap();
    let (base, gate, mut rx, server) = fake(true).await;
    let agent = setup(&temp.path().join("agent"), &base).await;
    let socket = rove_sdk::socket_path(&agent.store.data_dir);
    let stop = tokio_util::sync::CancellationToken::new();
    let agent_server = tokio::spawn(rove_agent::serve(agent.clone(), stop.clone()));
    while !socket.exists() {
        tokio::task::yield_now().await;
    }
    let s1 = session(&agent).await;
    let (_, a) = submit(&agent, &s1, "tools a").await;
    let s2 = session(&agent).await;
    let (_, b) = submit(&agent, &s2, "tools b").await;
    entered(&mut rx).await;
    entered(&mut rx).await;
    let client = rove_sdk::LocalClient::new(socket);
    let mut subscription = client
        .subscribe(a["run_id"].as_str().unwrap().parse().unwrap(), 0)
        .await
        .unwrap();
    assert!(subscription.next().await.unwrap().is_some());
    subscription.unsubscribe().await.unwrap(); // Must not cancel a's execution.
    gate.add_permits(20);
    wait_status(&agent, &a, "succeeded").await;
    wait_status(&agent, &b, "succeeded").await;
    let mut per_run = Vec::new();
    for run in [&a, &b] {
        let mut stream = client
            .subscribe(run["run_id"].as_str().unwrap().parse().unwrap(), 0)
            .await
            .unwrap();
        let mut events = Vec::new();
        while let Some(event) = stream.next().await.unwrap() {
            events.push(event);
        }
        let started: Vec<_> = events
            .iter()
            .filter(|e| e["kind"] == "tool_started")
            .collect();
        let finished: Vec<_> = events
            .iter()
            .filter(|e| e["kind"] == "tool_finished")
            .collect();
        assert_eq!(started.len(), 2);
        assert_eq!(finished.len(), 2);
        assert!(finished[0]["seq"].as_i64() < started[1]["seq"].as_i64());
        assert_eq!(finished[0]["data"]["result"]["stdout"], "first");
        assert_eq!(finished[1]["data"]["result"]["stdout"], "second");
        let caught = client
            .subscribe(stream.run_id, stream.last_seq)
            .await
            .unwrap();
        assert!(caught.terminal);
        per_run.push((
            started[0]["created_at"].as_str().unwrap().to_owned(),
            finished[0]["created_at"].as_str().unwrap().to_owned(),
        ));
    }
    assert!(
        per_run[0].0 < per_run[1].1 && per_run[1].0 < per_run[0].1,
        "Commands must overlap across runs"
    );
    stop.cancel();
    agent_server.await.unwrap().unwrap();
    server.abort();
}

#[cfg(unix)]
#[tokio::test]
async fn cancelling_a_running_command_does_not_cancel_another_run() {
    let temp = tempfile::tempdir().unwrap();
    let (base, gate, mut rx, server) = fake(true).await;
    let agent = setup(&temp.path().join("agent"), &base).await;
    let s1 = session(&agent).await;
    let (_, a) = submit(&agent, &s1, "cancel this command").await;
    let s2 = session(&agent).await;
    let (_, b) = submit(&agent, &s2, "keep this command").await;
    entered(&mut rx).await;
    entered(&mut rx).await;
    gate.add_permits(20);
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if snapshot(&agent, &a).await["output_tail"]
                .as_str()
                .unwrap()
                .contains("first")
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    agent
        .handle(&Request::new("cancel_run").with_path("run_id", a["run_id"].as_str().unwrap()))
        .await;
    wait_status(&agent, &a, "cancelled").await;
    wait_status(&agent, &b, "succeeded").await;
    let messages = agent
        .handle(&Request::new("list_messages").with_path("session_id", &s1))
        .await
        .body
        .unwrap();
    let results: Vec<_> = messages["items"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|m| m["parts"].as_array().unwrap())
        .filter(|p| p["kind"] == "tool_result")
        .collect();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["result"]["result"]["status"], "cancelled");
    assert_eq!(
        results[0]["result"]["result"]["residual_processes"],
        json!([])
    );
    let (_, continued) = submit(&agent, &s1, "continue safely").await;
    wait_status(&agent, &continued, "succeeded").await;
    agent.shutdown().await;
    server.abort();
}

#[tokio::test]
async fn failure_advances_session_and_lowering_capacity_preserves_active_runs() {
    let temp = tempfile::tempdir().unwrap();
    let (base, gate, mut rx, server) = fake(false).await;
    let agent = setup(&temp.path().join("agent"), &base).await;
    let mut running = Vec::new();
    for _ in 0..4 {
        let s = session(&agent).await;
        running.push(submit(&agent, &s, "hold capacity").await.1);
        entered(&mut rx).await;
    }
    let extra_session = session(&agent).await;
    let (_, queued) = submit(&agent, &extra_session, "next slot").await;
    agent
        .handle(&Request::new("update_settings").with_body(json!({"max_active_runs":2})))
        .await;
    gate.add_permits(1);
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let mut done = 0;
            for run in &running {
                if snapshot(&agent, run).await["run"]["status"] == "succeeded" {
                    done += 1;
                }
            }
            if done == 1 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(snapshot(&agent, &queued).await["run"]["status"], "queued");
    assert!(
        tokio::time::timeout(Duration::from_millis(100), rx.recv())
            .await
            .is_err()
    );
    gate.add_permits(20);
    for run in &running {
        wait_status(&agent, run, "succeeded").await;
    }
    wait_status(&agent, &queued, "succeeded").await;
    let s = session(&agent).await;
    let (_, failed) = submit(&agent, &s, "fail deliberately").await;
    let (_, next) = submit(&agent, &s, "recover from failure").await;
    wait_status(&agent, &failed, "failed").await;
    wait_status(&agent, &next, "succeeded").await;
    assert_eq!(
        snapshot(&agent, &failed).await["run"]["error"]["code"],
        "model_provider_failed"
    );
    agent.shutdown().await;
    server.abort();
}

#[cfg(unix)]
#[tokio::test]
async fn socket_keeps_correlation_and_business_errors_isolated() {
    use rove_protocol::frame::{read_frame, write_frame};
    let temp = tempfile::tempdir().unwrap();
    let (base, gate, mut rx, provider) = fake(false).await;
    let agent = setup(&temp.path().join("agent"), &base).await;
    let socket = rove_sdk::socket_path(&agent.store.data_dir);
    let stop = tokio_util::sync::CancellationToken::new();
    let server = tokio::spawn(rove_agent::serve(agent.clone(), stop.clone()));
    while !socket.exists() {
        tokio::task::yield_now().await;
    }
    let network = agent
        .handle(&Request::new("create_network").with_body(json!({"display_name":"test"})))
        .await
        .body
        .unwrap();
    let mut stream = tokio::net::UnixStream::connect(&socket).await.unwrap();
    let slow = Request::new("create_network_share")
        .with_path("network_id", network["network_id"].as_str().unwrap())
        .with_body(json!({"config_server_url":base.trim_end_matches("/v1")}));
    write_frame(&mut stream, &json!(slow)).await.unwrap();
    entered(&mut rx).await; // The original request is still waiting for HTTP.
    write_frame(&mut stream, &json!(slow)).await.unwrap();
    let duplicate = read_frame(&mut stream).await.unwrap().unwrap();
    assert_eq!(duplicate["correlation_id"], slow.correlation_id.to_string());
    assert_eq!(duplicate["status_code"], 409);
    assert_eq!(duplicate["body"]["error"]["code"], "correlation_conflict");
    for (request, expected) in [
        (Request::new("not_an_operation"), 404),
        (Request::new("create_session"), 400),
        (Request::new("get_device"), 200),
    ] {
        write_frame(&mut stream, &json!(request)).await.unwrap();
        let response = read_frame(&mut stream).await.unwrap().unwrap();
        assert_eq!(
            response["correlation_id"],
            request.correlation_id.to_string()
        );
        assert_eq!(response["status_code"], expected);
    }
    gate.add_permits(1);
    let original = read_frame(&mut stream).await.unwrap().unwrap();
    assert_eq!(original["correlation_id"], slow.correlation_id.to_string());
    assert!(original["status_code"].as_u64().unwrap() >= 400);
    drop(stream);
    stop.cancel();
    server.await.unwrap().unwrap();
    provider.abort();
}

#[cfg(unix)]
#[tokio::test]
async fn two_subscriptions_share_socket_without_cancelling_runs() {
    use rove_protocol::frame::{FrameReader, write_frame};
    let temp = tempfile::tempdir().unwrap();
    let (base, gate, mut rx, provider) = fake(false).await;
    let agent = setup(&temp.path().join("agent"), &base).await;
    let socket = rove_sdk::socket_path(&agent.store.data_dir);
    let stop = tokio_util::sync::CancellationToken::new();
    let server = tokio::spawn(rove_agent::serve(agent.clone(), stop.clone()));
    while !socket.exists() {
        tokio::task::yield_now().await;
    }
    let mut runs = Vec::new();
    for _ in 0..2 {
        let session = session(&agent).await;
        runs.push(submit(&agent, &session, "stay running").await.1);
        entered(&mut rx).await;
    }
    let mut stream = tokio::net::UnixStream::connect(&socket).await.unwrap();
    let mut reader = FrameReader::default();
    for run in &runs {
        let request = Request::new("subscribe_run_events")
            .with_path("run_id", run["run_id"].as_str().unwrap());
        write_frame(&mut stream, &json!(request)).await.unwrap();
    }
    let mut subscriptions = std::collections::HashMap::new();
    let mut watermarks = std::collections::HashMap::new();
    tokio::time::timeout(Duration::from_secs(10), async {
        while subscriptions.len() < 2
            || watermarks.len() < 2
            || watermarks.values().any(|&seq| seq < 2)
        {
            let frame = reader.next(&mut stream).await.unwrap().unwrap();
            rove_protocol::contract::AGENT
                .validate("SocketFrame", &frame)
                .unwrap();
            if frame["kind"] == "response" {
                assert_eq!(frame["status_code"], 200);
                subscriptions.insert(
                    frame["body"]["run_id"].as_str().unwrap().to_owned(),
                    frame["body"]["subscription_id"].clone(),
                );
            } else {
                assert_eq!(frame["kind"], "event");
                let run = frame["event"]["run_id"].as_str().unwrap();
                assert_eq!(frame["subscription_id"], subscriptions[run]);
                let seq = frame["event"]["seq"].as_i64().unwrap();
                assert_eq!(seq, watermarks.get(run).copied().unwrap_or(0) + 1);
                watermarks.insert(run.to_owned(), seq);
            }
        }
    })
    .await
    .unwrap();
    let a = runs[0]["run_id"].as_str().unwrap();
    let b = runs[1]["run_id"].as_str().unwrap();
    let correlation = Uuid::new_v4();
    let unsubscribe = json!({"kind":"unsubscribe","correlation_id":correlation,"subscription_id":subscriptions[a]});
    // A different connection cannot release either subscription.
    let mut foreign = tokio::net::UnixStream::connect(&socket).await.unwrap();
    write_frame(&mut foreign, &unsubscribe).await.unwrap();
    let foreign_ack = rove_protocol::frame::read_frame(&mut foreign)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(foreign_ack["status_code"], 204);
    assert!(
        tokio::time::timeout(Duration::from_millis(150), reader.next(&mut stream))
            .await
            .is_err()
    );
    write_frame(&mut stream, &unsubscribe).await.unwrap();
    let mut acknowledged = false;
    let mut ended = false;
    tokio::time::timeout(Duration::from_secs(10), async {
        while !acknowledged || !ended {
            let frame = reader.next(&mut stream).await.unwrap().unwrap();
            if frame["kind"] == "response" {
                assert_eq!(frame["correlation_id"], correlation.to_string());
                assert_eq!(frame["status_code"], 204);
                acknowledged = true;
            } else {
                assert_eq!(frame["kind"], "stream_end");
                assert_eq!(frame["subscription_id"], subscriptions[a]);
                assert_eq!(frame["reason"], "unsubscribed");
                ended = true;
            }
        }
    })
    .await
    .unwrap();
    // Repeating unsubscribe stays idempotent and does not affect the other run.
    write_frame(&mut stream, &unsubscribe).await.unwrap();
    assert_eq!(
        reader.next(&mut stream).await.unwrap().unwrap()["status_code"],
        204
    );
    gate.add_permits(2);
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let frame = reader.next(&mut stream).await.unwrap().unwrap();
            assert_eq!(frame["subscription_id"], subscriptions[b]);
            if frame["kind"] == "stream_end" {
                assert_eq!(frame["reason"], "terminal");
                break;
            }
            assert_eq!(frame["event"]["run_id"], b);
        }
    })
    .await
    .unwrap();
    for run in &runs {
        wait_status(&agent, run, "succeeded").await;
    }
    drop(stream);
    drop(foreign);
    stop.cancel();
    server.await.unwrap().unwrap();
    provider.abort();
}
