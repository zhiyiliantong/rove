use super::*;
use axum::{Router, http::header, routing::post};
use tokio::{
    sync::{Semaphore, mpsc},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

async fn call(agent: &Arc<Agent>, request: Request) -> Value {
    let response = agent.handle(&request).await;
    assert!(
        response.status_code < 300,
        "{}: {} {:?}",
        request.operation_id,
        response.status_code,
        response.body
    );
    response.body.unwrap_or(Value::Null)
}
async fn listen(agent: Arc<Agent>, network: Uuid) -> (url::Url, JoinHandle<()>, CancellationToken) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap())
        .parse()
        .unwrap();
    let stop = CancellationToken::new();
    let signal = stop.clone();
    let task = tokio::spawn(async move {
        crate::http::serve_bound(agent, network, listener, signal)
            .await
            .unwrap();
    });
    (url, task, stop)
}
fn route(origin: &Agent, peer: &Agent, url: url::Url) {
    origin
        .remote_test_routes
        .lock()
        .unwrap()
        .insert(peer.store.device_id.0, url);
}
async fn model(peer: &Arc<Agent>) -> (Arc<Semaphore>, mpsc::UnboundedReceiver<()>, JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}/v1", listener.local_addr().unwrap());
    let gate = Arc::new(Semaphore::new(0));
    let ready = gate.clone();
    let (tx, rx) = mpsc::unbounded_channel();
    let app=Router::new().route("/v1/chat/completions",post(move||{
        let gate=ready.clone();let tx=tx.clone();async move{
            let _=tx.send(());gate.acquire().await.unwrap().forget();
            let first=json!({"id":"test","object":"chat.completion.chunk","created":1,"model":"test","choices":[{"index":0,"delta":{"role":"assistant","content":"remote progress 中文"},"finish_reason":null}]});
            let last=json!({"id":"test","object":"chat.completion.chunk","created":1,"model":"test","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]});
            ([(header::CONTENT_TYPE,"text/event-stream")],format!("data: {first}\n\ndata: {last}\n\ndata: [DONE]\n\n"))
        }
    }));
    let task = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    call(peer,Request::new("set_model_config").with_body(json!({"provider":"openai_compatible","base_url":base,"model":"test","api_key":"private-peer-key"}))).await;
    (gate, rx, task)
}
async fn link(origin: &Arc<Agent>, peer: &Agent, network: Uuid) -> Value {
    call(origin,Request::new("create_session").with_body(json!({"session_id":Uuid::new_v4(),"title":"Origin conversation","execution_target":{"network_id":network,"device_id":peer.store.device_id}}))).await
}
fn for_session(op: &str, session: &Value) -> Request {
    Request::new(op).with_path("session_id", session["session_id"].as_str().unwrap())
}
fn list_runs(session: &Value) -> Request {
    let mut request = Request::new("list_runs");
    request
        .query_parameters
        .insert("session_id".into(), session["session_id"].clone());
    request
}
async fn finished(agent: &Arc<Agent>, run: &str) -> Value {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let snap = call(agent, Request::new("get_run").with_path("run_id", run)).await;
            if sessions::terminal(&snap["run"]) {
                return snap;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap()
}

#[tokio::test]
async fn linked_auto_title_waits_for_acceptance_and_cache_preserves_manual_rename() {
    let temp = tempfile::tempdir().unwrap();
    let origin = Agent::open(&temp.path().join("origin")).unwrap();
    let peer = Agent::open(&temp.path().join("peer")).unwrap();
    let network = Uuid::new_v4();
    let (url, server, stop) = listen(peer.clone(), network).await;
    route(&origin, &peer, url);
    let (gate, _entered, provider) = model(&peer).await;
    let session = call(&origin,Request::new("create_session").with_body(json!({"title":"新会话","auto_title":true,"execution_target":{"network_id":network,"device_id":peer.store.device_id}}))).await;
    let body = json!({"request_id":Uuid::new_v4(),"message":"在远端安装音乐服务"});
    origin
        .remote_drop_submit_response
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let response = origin
        .handle(&for_session("submit_run", &session).with_body(body.clone()))
        .await;
    assert!(response.status_code >= 400);
    assert_eq!(
        call(&origin, for_session("get_session", &session)).await["title_source"],
        "default"
    );
    call(&origin, list_runs(&session)).await;
    assert_eq!(
        call(&origin, for_session("get_session", &session)).await["title"],
        "在远端安装音乐服务"
    );
    call(
        &origin,
        for_session("update_session", &session).with_body(json!({"title":"我的音乐"})),
    )
    .await;
    call(&origin, for_session("submit_run", &session).with_body(body)).await;
    call(&origin, list_runs(&session)).await;
    assert_eq!(
        call(&origin, for_session("get_session", &session)).await["title"],
        "我的音乐"
    );
    // Peer creation input stays stable even after local automatic/manual renames.
    assert_eq!(
        call(&peer, for_session("get_session", &session)).await["title"],
        "新会话"
    );
    gate.add_permits(1);
    stop.cancel();
    server.await.unwrap();
    origin.shutdown().await;
    peer.shutdown().await;
    provider.abort();
}

#[tokio::test]
async fn origin_restart_keeps_remote_execution_and_cached_progress_with_sdk_streams() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("origin");
    let origin = Agent::open(&path).unwrap();
    let peer = Agent::open(&temp.path().join("peer")).unwrap();
    let network = Uuid::new_v4();
    let (url, server, stop) = listen(peer.clone(), network).await;
    route(&origin, &peer, url.clone());
    let (gate, mut entered, provider) = model(&peer).await;
    let session = link(&origin, &peer, network).await;
    assert_eq!(session["device_id"], json!(origin.store.device_id));
    assert!(origin.store.get("model_config").unwrap().is_none());
    let catalog = call(&peer, Request::new("get_model_catalog")).await;
    call(
        &origin,
        for_session("update_session", &session)
            .with_body(json!({"model_id":catalog["default_model_id"]})),
    )
    .await;
    let body = json!({"request_id":Uuid::new_v4(),"message":"Work on peer"});
    let run = call(
        &origin,
        for_session("submit_run", &session).with_body(body.clone()),
    )
    .await;
    let id = run["run_id"].as_str().unwrap();
    tokio::time::timeout(Duration::from_secs(5), entered.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(run["device_id"], json!(peer.store.device_id));
    assert_eq!(run["model_id"], catalog["default_model_id"]);
    let first = call(&origin, Request::new("get_run").with_path("run_id", id)).await;
    assert_eq!(first["run"]["status"], "running");
    let owner_id = origin.store.device_id;
    origin.shutdown().await;
    drop(origin);
    assert_eq!(
        call(&peer, Request::new("get_run").with_path("run_id", id)).await["run"]["status"],
        "running"
    );
    let origin = Agent::open(&path).unwrap();
    assert_eq!(origin.store.device_id, owner_id);
    let cached = call(&origin, Request::new("get_run").with_path("run_id", id)).await;
    assert_eq!(cached["run"]["status"], "running");
    assert_eq!(cached["sync_error"]["code"], "target_unreachable");
    let listed = call(&origin, Request::new("list_sessions")).await;
    assert_eq!(listed["items"][0]["session_id"], session["session_id"]);
    let offline = call(&origin, list_runs(&session)).await;
    assert!(offline["sync_error"].is_object());
    assert_eq!(offline["items"][0]["run_id"], id);
    route(&origin, &peer, url);
    // Real SDK -> origin socket -> SDK -> peer HTTP/SSE, no explicit target.
    let signal = CancellationToken::new();
    let owned = origin.clone();
    let shutdown = signal.clone();
    let runtime = tokio::spawn(async move {
        crate::serve(owned, shutdown).await.unwrap();
    });
    let client = rove_sdk::LocalClient::new(rove_sdk::socket_path(&path));
    for _ in 0..100 {
        if rove_sdk::socket_path(&path).exists() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let mut stream = client
        .subscribe(id.parse().unwrap(), first["snapshot_seq"].as_i64().unwrap())
        .await
        .unwrap();
    gate.add_permits(1);
    let events = tokio::time::timeout(Duration::from_secs(10), async {
        let mut values = Vec::new();
        while let Some(event) = stream.next().await.unwrap() {
            values.push(event);
        }
        values
    })
    .await
    .unwrap();
    assert!(events.iter().any(|e| e["kind"] == "assistant_delta"));
    let final_snapshot = finished(&origin, id).await;
    assert_eq!(final_snapshot["output_tail"], "remote progress 中文");
    origin
        .cache_run(&session, &first["run"], Some(&first))
        .unwrap();
    let cached: String = origin
        .store
        .connection
        .lock()
        .unwrap()
        .query_row("SELECT snapshot FROM remote_runs WHERE id=?1", [id], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(
        decode(cached).unwrap()["snapshot_seq"],
        final_snapshot["snapshot_seq"]
    );
    let mut wrong = final_snapshot["run"].clone();
    wrong["request_id"] = json!(Uuid::new_v4());
    assert_eq!(
        origin.cache_run(&session, &wrong, None).unwrap_err().code,
        "target_mismatch"
    );
    // The origin's HTTP/SSE surface provides the same linked event history.
    let (origin_url, origin_http, origin_stop) = listen(origin.clone(), network).await;
    let remote =
        rove_sdk::RemoteClient::new(origin_url, network, origin.store.device_id.0).unwrap();
    let mut replay = remote.subscribe(id.parse().unwrap(), 0).await.unwrap();
    let mut seq = 0;
    while let Some(event) = tokio::time::timeout(Duration::from_secs(5), replay.next())
        .await
        .unwrap()
        .unwrap()
    {
        seq = event["seq"].as_i64().unwrap();
    }
    assert_eq!(seq, final_snapshot["snapshot_seq"].as_i64().unwrap());
    origin_stop.cancel();
    origin_http.await.unwrap();
    assert_eq!(
        call(&origin, for_session("submit_run", &session).with_body(body)).await["run_id"],
        id
    );
    assert!(entered.try_recv().is_err());
    assert!(
        call(&origin, for_session("list_session_submissions", &session)).await["items"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    call(&origin, for_session("archive_session", &session)).await;
    call(&origin, for_session("delete_session", &session)).await;
    let retry = origin
        .handle(
            &for_session("submit_run", &session)
                .with_body(json!({"request_id":run["request_id"],"message":"Work on peer"})),
        )
        .await;
    assert_eq!(retry.body.unwrap()["error"]["code"], "request_deleted");
    assert_eq!(
        call(&peer, for_session("get_session", &session)).await["session_id"],
        session["session_id"]
    );
    signal.cancel();
    runtime.await.unwrap();
    drop(client);
    drop(stream);
    drop(origin);
    stop.cancel();
    server.await.unwrap();
    peer.shutdown().await;
    provider.abort();
}

#[tokio::test]
async fn uncertain_acceptance_is_persistent_conflict_safe_and_never_replayed_on_reads() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("origin");
    let origin = Agent::open(&path).unwrap();
    let peer = Agent::open(&temp.path().join("peer")).unwrap();
    let network = Uuid::new_v4();
    let (url, server, stop) = listen(peer.clone(), network).await;
    route(&origin, &peer, url.clone());
    let (gate, mut entered, provider) = model(&peer).await;
    let session = link(&origin, &peer, network).await;
    let body = json!({"request_id":Uuid::new_v4(),"message":"Only execute once"});
    origin
        .remote_drop_submit_response
        .store(true, std::sync::atomic::Ordering::SeqCst);
    assert_eq!(
        origin
            .handle(&for_session("submit_run", &session).with_body(body.clone()))
            .await
            .status_code,
        503
    );
    tokio::time::timeout(Duration::from_secs(5), entered.recv())
        .await
        .unwrap()
        .unwrap();
    let pending = call(&origin, for_session("list_session_submissions", &session)).await;
    assert_eq!(
        pending["items"][0]["request"]["request_id"],
        body["request_id"]
    );
    call(&origin, for_session("archive_session", &session)).await;
    assert_eq!(
        origin
            .handle(&for_session("delete_session", &session))
            .await
            .status_code,
        409
    );
    assert_eq!(
        origin
            .handle(&for_session("submit_run", &session).with_body(body.clone()))
            .await
            .status_code,
        409
    );
    call(&origin, for_session("restore_session", &session)).await;
    origin.shutdown().await;
    drop(origin);
    let origin = Agent::open(&path).unwrap();
    route(&origin, &peer, url);
    call(&origin, for_session("list_session_submissions", &session)).await;
    assert!(entered.try_recv().is_err());
    let mut changed = body.clone();
    changed["message"] = json!("different input");
    assert_eq!(
        origin
            .handle(&for_session("submit_run", &session).with_body(changed))
            .await
            .status_code,
        409
    );
    let (_, wire) = origin
        .store
        .connection
        .lock()
        .unwrap()
        .query_row("SELECT request,wire FROM remote_submissions", [], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })
        .unwrap();
    let accepted = call(
        &origin,
        for_session("submit_run", &session).with_body(decode(wire).unwrap()),
    )
    .await;
    assert_eq!(accepted["request_id"], body["request_id"]);
    assert!(entered.try_recv().is_err());
    gate.add_permits(1);
    finished(&origin, accepted["run_id"].as_str().unwrap()).await;
    // A second lost response is reconciled by querying, without a submit retry.
    let body2 = json!({"request_id":Uuid::new_v4(),"message":"Second request"});
    origin
        .remote_drop_submit_response
        .store(true, std::sync::atomic::Ordering::SeqCst);
    assert_eq!(
        origin
            .handle(&for_session("submit_run", &session).with_body(body2))
            .await
            .status_code,
        503
    );
    tokio::time::timeout(Duration::from_secs(5), entered.recv())
        .await
        .unwrap()
        .unwrap();
    call(&origin, list_runs(&session)).await;
    assert!(
        call(&origin, for_session("list_session_submissions", &session)).await["items"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let jobs = call(&peer, Request::new("list_runs")).await;
    assert_eq!(jobs["items"].as_array().unwrap().len(), 2);
    let second = jobs["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["run_id"] != accepted["run_id"])
        .unwrap();
    call(
        &origin,
        Request::new("cancel_run").with_path("run_id", second["run_id"].as_str().unwrap()),
    )
    .await;
    assert_eq!(
        finished(&peer, second["run_id"].as_str().unwrap()).await["run"]["status"],
        "cancelled"
    );
    gate.add_permits(1);
    origin.shutdown().await;
    stop.cancel();
    server.await.unwrap();
    peer.shutdown().await;
    provider.abort();
}

#[tokio::test]
async fn creation_is_idempotent_and_missing_peer_never_executes_locally() {
    let temp = tempfile::tempdir().unwrap();
    let origin = Agent::open(&temp.path().join("a")).unwrap();
    let body = json!({"session_id":Uuid::new_v4(),"title":"stable","execution_target":{"network_id":Uuid::new_v4(),"device_id":Uuid::new_v4()}});
    let session = call(
        &origin,
        Request::new("create_session").with_body(body.clone()),
    )
    .await;
    assert_eq!(
        call(
            &origin,
            Request::new("create_session").with_body(body.clone())
        )
        .await,
        session
    );
    call(
        &origin,
        for_session("update_session", &session).with_body(json!({"title":"renamed"})),
    )
    .await;
    assert_eq!(
        call(
            &origin,
            Request::new("create_session").with_body(body.clone())
        )
        .await["title"],
        "renamed"
    );
    let mut conflict = body.clone();
    conflict["title"] = json!("different");
    assert_eq!(
        origin
            .handle(&Request::new("create_session").with_body(conflict))
            .await
            .status_code,
        409
    );
    let input = json!({"request_id":Uuid::new_v4(),"message":"No fallback"});
    assert_eq!(
        origin
            .handle(&for_session("submit_run", &session).with_body(input.clone()))
            .await
            .status_code,
        503
    );
    assert_eq!(
        origin
            .store
            .connection
            .lock()
            .unwrap()
            .query_row("SELECT count(*) FROM runs", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
    let pending = call(&origin, for_session("list_session_submissions", &session)).await;
    assert_eq!(pending["items"][0]["request"]["message"], input["message"]);
    origin.shutdown().await;
}
