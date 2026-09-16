use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use rove_protocol::{Request, SocketTarget};
use rove_sdk::RemoteClient;
use serde_json::json;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use uuid::Uuid;

struct Fixture {
    network: Uuid,
    device: Uuid,
    mode: u8,
    calls: AtomicUsize,
}
async fn hello(State(state): State<Arc<Fixture>>, headers: HeaderMap) -> Response {
    assert_eq!(headers["X-Rove-Network-Id"], state.network.to_string());
    match state.mode {
        1=>(StatusCode::OK," ".repeat(65537)).into_response(),
        2=>(StatusCode::TEMPORARY_REDIRECT,[(header::LOCATION,"/business")]).into_response(),
        _=>Json(json!({"device_id":if state.mode==3{Uuid::new_v4()}else{state.device},"display_name":"remote fixture","agent_version":"0.1.0","protocol":{"min":if state.mode==4{2}else{1},"max":2},"capabilities":[],"network_id":state.network})).into_response(),
    }
}
async fn device(State(state): State<Arc<Fixture>>, headers: HeaderMap) -> Response {
    state.calls.fetch_add(1, Ordering::SeqCst);
    assert_eq!(headers["X-Rove-Network-Id"], state.network.to_string());
    assert_eq!(headers["X-Rove-Target-Device-Id"], state.device.to_string());
    Json(json!({"device_id":state.device,"display_name":"remote fixture","agent_version":"0.1.0","os":"linux","arch":"x86_64","capabilities":[],"started_at":"2026-09-08T00:00:00Z"})).into_response()
}

#[tokio::test]
async fn remote_handshake_is_bounded_checks_identity_and_never_follows_redirects() {
    for mode in 0..=4 {
        let state = Arc::new(Fixture {
            network: Uuid::new_v4(),
            device: Uuid::new_v4(),
            mode,
            calls: AtomicUsize::new(0),
        });
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let app = Router::new()
            .route("/v1/hello", get(hello))
            .route("/v1/device", get(device))
            .route("/business", get(device))
            .with_state(state.clone());
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let client = RemoteClient::new(base.parse().unwrap(), state.network, state.device).unwrap();
        let mut mismatched = Request::new("get_device");
        mismatched.target = Some(SocketTarget {
            network_id: state.network,
            device_id: Uuid::new_v4(),
        });
        assert!(
            client
                .call(mismatched)
                .await
                .unwrap_err()
                .to_string()
                .contains("target_mismatch")
        );
        let result = client.call(Request::new("get_device")).await;
        if mode == 0 {
            let result = result.unwrap();
            assert_eq!(result.status_code, 200);
            assert_eq!(result.body.unwrap()["device_id"], state.device.to_string());
            assert_eq!(state.calls.load(Ordering::SeqCst), 1);
        } else {
            assert!(result.is_err());
            assert_eq!(state.calls.load(Ordering::SeqCst), 0);
        }
        server.abort();
        let _ = server.await;
    }
}

fn event(run: Uuid, seq: i64, status: &str) -> String {
    let value = json!({"run_id":run,"seq":seq,"created_at":"2026-09-08T00:00:00Z","kind":"status","data":{"status":status}});
    format!("id: {seq}\r\nevent: run_event\r\ndata: {value}\r\n\r\n")
}

#[tokio::test]
async fn sse_reconnect_retains_target_deduplicates_and_stops_on_terminal() {
    use axum::extract::Query;
    use std::collections::HashMap;
    let state = Arc::new(Fixture {
        network: Uuid::new_v4(),
        device: Uuid::new_v4(),
        mode: 0,
        calls: AtomicUsize::new(0),
    });
    let run = Uuid::new_v4();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let app = Router::new()
        .route("/v1/hello", get(hello))
        .route(
            &format!("/v1/runs/{run}/events"),
            get(
                move |State(state): State<Arc<Fixture>>,
                      Query(query): Query<HashMap<String, String>>,
                      headers: HeaderMap| async move {
                    assert_eq!(headers["X-Rove-Network-Id"], state.network.to_string());
                    assert_eq!(headers["X-Rove-Target-Device-Id"], state.device.to_string());
                    let visit = state.calls.fetch_add(1, Ordering::SeqCst);
                    match visit {
                        0 => {
                            assert_eq!(query["after_seq"], "0");
                            (
                                [(header::CONTENT_TYPE, "text/event-stream")],
                                event(run, 1, "running"),
                            )
                                .into_response()
                        }
                        1 => {
                            assert_eq!(query["after_seq"], "1");
                            (
                                [(header::CONTENT_TYPE, "text/event-stream; charset=utf-8")],
                                format!(
                                    "{}{}",
                                    event(run, 1, "running"),
                                    event(run, 2, "succeeded")
                                ),
                            )
                                .into_response()
                        }
                        2 => {
                            assert_eq!(query["after_seq"], "2");
                            StatusCode::NO_CONTENT.into_response()
                        }
                        _ => panic!("Unexpected resubmission"),
                    }
                },
            ),
        )
        .with_state(state.clone());
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let client = RemoteClient::new(base.parse().unwrap(), state.network, state.device).unwrap();
    let mut first = client.subscribe(run, 0).await.unwrap();
    assert_eq!(first.next().await.unwrap().unwrap()["seq"], 1);
    assert!(
        first
            .next()
            .await
            .unwrap_err()
            .to_string()
            .contains("disconnected")
    );
    assert_eq!(first.last_seq, 1);
    let mut resumed = client.resume(&first).await.unwrap();
    assert_eq!(resumed.next().await.unwrap().unwrap()["seq"], 2);
    assert!(resumed.terminal && resumed.next().await.unwrap().is_none());
    let mut terminal = client.resume(&resumed).await.unwrap();
    assert!(terminal.terminal && terminal.next().await.unwrap().is_none());
    let wrong = RemoteClient::new(base.parse().unwrap(), state.network, Uuid::new_v4()).unwrap();
    assert!(
        wrong
            .resume(&resumed)
            .await
            .err()
            .unwrap()
            .to_string()
            .contains("target_mismatch")
    );
    assert_eq!(state.calls.load(Ordering::SeqCst), 3);
    server.abort();
    let _ = server.await;
}

#[tokio::test]
async fn sse_rejects_expired_cursor_wrong_run_ids_and_sequence_gaps() {
    for mode in 0..5 {
        let state = Arc::new(Fixture {
            network: Uuid::new_v4(),
            device: Uuid::new_v4(),
            mode: 0,
            calls: AtomicUsize::new(0),
        });
        let run = Uuid::new_v4();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let app = Router::new()
            .route("/v1/hello", get(hello))
            .route(
                &format!("/v1/runs/{run}/events"),
                get(move || async move {
                    match mode {
                        0 => (
                            StatusCode::GONE,
                            Json(
                                rove_protocol::ApiError::new(
                                    410,
                                    "event_history_expired",
                                    "Restore snapshot",
                                )
                                .body(),
                            ),
                        )
                            .into_response(),
                        1 => (
                            [(header::CONTENT_TYPE, "text/plain")],
                            event(run, 1, "running"),
                        )
                            .into_response(),
                        2 => (
                            [(header::CONTENT_TYPE, "text/event-stream")],
                            event(Uuid::new_v4(), 1, "running"),
                        )
                            .into_response(),
                        3 => (
                            [(header::CONTENT_TYPE, "text/event-stream")],
                            event(run, 2, "running"),
                        )
                            .into_response(),
                        _ => (
                            [(header::CONTENT_TYPE, "text/event-stream")],
                            event(run, 1, "running").replacen("id: 1", "id: 2", 1),
                        )
                            .into_response(),
                    }
                }),
            )
            .with_state(state.clone());
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let client = RemoteClient::new(base.parse().unwrap(), state.network, state.device).unwrap();
        let opened = client.subscribe(run, 0).await;
        if mode <= 1 {
            let error = opened.err().unwrap().to_string();
            assert!(error.contains(if mode == 0 {
                "event_history_expired"
            } else {
                "text/event-stream"
            }));
        } else {
            let mut opened = opened.unwrap();
            assert!(opened.next().await.is_err());
            assert_eq!(opened.last_seq, 0);
            assert!(opened.next().await.is_err());
        }
        server.abort();
        let _ = server.await;
    }
}

#[tokio::test]
async fn cancelled_sse_read_preserves_partial_utf8_and_event_state() {
    use axum::body::{Body, Bytes};
    let state = Arc::new(Fixture {
        network: Uuid::new_v4(),
        device: Uuid::new_v4(),
        mode: 0,
        calls: AtomicUsize::new(0),
    });
    let run = Uuid::new_v4();
    let (tx, rx) = tokio::sync::mpsc::channel::<Bytes>(4);
    let receiver = Arc::new(std::sync::Mutex::new(Some(rx)));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let app = Router::new()
        .route("/v1/hello", get(hello))
        .route(
            &format!("/v1/runs/{run}/events"),
            get(move || {
                let rx = receiver.lock().unwrap().take().unwrap();
                async move {
                    let stream = futures_util::stream::unfold(rx, |mut rx| async move {
                        rx.recv()
                            .await
                            .map(|bytes| (Ok::<_, std::convert::Infallible>(bytes), rx))
                    });
                    (
                        [(header::CONTENT_TYPE, "text/event-stream")],
                        Body::from_stream(stream),
                    )
                }
            }),
        )
        .with_state(state.clone());
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let client = RemoteClient::new(base.parse().unwrap(), state.network, state.device).unwrap();
    let value = json!({"run_id":run,"seq":1,"created_at":"2026-09-08T00:00:00Z","kind":"assistant_delta","data":{"text":"漫游者"}});
    let message = format!("id: 1\nevent: run_event\ndata: {value}\n\n");
    let split = message.find("漫").unwrap() + 1;
    tx.send(Bytes::copy_from_slice(&message.as_bytes()[..split]))
        .await
        .unwrap();
    let mut stream = client.subscribe(run, 0).await.unwrap();
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(30), stream.next())
            .await
            .is_err()
    );
    assert_eq!(stream.last_seq, 0);
    tx.send(Bytes::copy_from_slice(&message.as_bytes()[split..]))
        .await
        .unwrap();
    assert_eq!(
        stream.next().await.unwrap().unwrap()["data"]["text"],
        "漫游者"
    );
    tx.send(Bytes::from(event(run, 2, "succeeded")))
        .await
        .unwrap();
    assert_eq!(stream.next().await.unwrap().unwrap()["seq"], 2);
    assert!(stream.terminal && stream.next().await.unwrap().is_none());
    server.abort();
    let _ = server.await;
}
