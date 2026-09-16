use super::*;
use tokio::net::{UnixListener, UnixStream};

async fn hello(stream: &mut UnixStream) {
    let request: Request =
        serde_json::from_value(read_frame(stream).await.unwrap().unwrap()).unwrap();
    assert_eq!(request.operation_id, "get_hello");
    let response = Response::new(
        &request,
        200,
        Some(
            json!({"device_id":uuid::Uuid::new_v4(),"display_name":"test","agent_version":"0.1.0","protocol":{"min":1,"max":1},"capabilities":[],"network_id":null}),
        ),
    );
    write_frame(stream, &json!(response)).await.unwrap();
}

#[tokio::test]
async fn stalled_hello_closes_without_sending_business_or_secrets() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("agent.sock");
    let listener = UnixListener::bind(&path).unwrap();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let request: Request =
            serde_json::from_value(read_frame(&mut stream).await.unwrap().unwrap()).unwrap();
        assert_eq!(request.operation_id, "get_hello");
        // No hello response: deadline must drop the connection, not send a call.
        assert!(read_frame(&mut stream).await.unwrap().is_none());
    });
    let request=Request::new("set_model_config").with_body(json!({"provider":"openai_compatible","base_url":"http://127.0.0.1:1/v1","model":"test","api_key":"must-not-leak"}));
    let error = LocalClient::new(path)
        .call_with_deadlines(request, Duration::from_millis(100), Duration::from_secs(1))
        .await
        .unwrap_err()
        .to_string();
    assert!(error.contains("handshake timed out"));
    assert!(!error.contains("must-not-leak"));
    tokio::time::timeout(Duration::from_secs(2), server)
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn accepted_call_timeout_sends_no_retry_or_cancel_and_retains_request_identity() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("agent.sock");
    let listener = UnixListener::bind(&path).unwrap();
    let request = Request::new("submit_run")
        .with_path("session_id", uuid::Uuid::new_v4())
        .with_body(json!({"request_id":uuid::Uuid::new_v4(),"message":"private-input"}));
    let expected = json!(request);
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        hello(&mut stream).await;
        assert_eq!(read_frame(&mut stream).await.unwrap().unwrap(), expected);
        // A response that starts but never completes must have the same deadline.
        use tokio::io::AsyncWriteExt;
        stream.write_all(&200_u32.to_be_bytes()).await.unwrap();
        stream.write_all(b"{\"kind\":").await.unwrap();
        assert!(read_frame(&mut stream).await.unwrap().is_none());
    });
    let error = LocalClient::new(path)
        .call_with_deadlines(request, Duration::from_secs(2), Duration::from_millis(100))
        .await
        .unwrap_err()
        .to_string();
    assert!(error.contains("response timed out"));
    assert!(error.contains("original request_id"));
    assert!(!error.contains("private-input"));
    tokio::time::timeout(Duration::from_secs(2), server)
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn stalled_subscription_ack_closes_observer_without_cancelling_run() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("agent.sock");
    let listener = UnixListener::bind(&path).unwrap();
    let run = uuid::Uuid::new_v4();
    let target = rove_protocol::SocketTarget {
        network_id: uuid::Uuid::new_v4(),
        device_id: uuid::Uuid::new_v4(),
    };
    let expected_target = json!(target);
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        hello(&mut stream).await;
        let request = read_frame(&mut stream).await.unwrap().unwrap();
        assert_eq!(request["operation_id"], "subscribe_run_events");
        assert_eq!(request["target"], expected_target);
        assert_eq!(request["query_parameters"]["after_seq"], 42);
        assert!(read_frame(&mut stream).await.unwrap().is_none());
    });
    let result = LocalClient::new(path)
        .subscribe_with_deadline(run, 42, Some(target), Duration::from_millis(200))
        .await;
    let error = result
        .err()
        .expect("acknowledgement must time out")
        .to_string();
    assert!(error.contains("Event handshake timed out"));
    tokio::time::timeout(Duration::from_secs(2), server)
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn established_subscription_can_wait_longer_than_the_handshake_deadline() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("agent.sock");
    let listener = UnixListener::bind(&path).unwrap();
    let run = uuid::Uuid::new_v4();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        hello(&mut stream).await;
        let request: Request =
            serde_json::from_value(read_frame(&mut stream).await.unwrap().unwrap()).unwrap();
        let subscription = uuid::Uuid::new_v4();
        write_frame(
            &mut stream,
            &json!(Response::new(
                &request,
                200,
                Some(json!({"run_id":run,"subscription_id":subscription}))
            )),
        )
        .await
        .unwrap();
        tokio::time::sleep(Duration::from_millis(500)).await;
        write_frame(&mut stream,&json!({"kind":"event","subscription_id":subscription,"event":{"run_id":run,"seq":1,"created_at":"2026-09-09T00:00:00Z","kind":"status","data":{"status":"running"}}})).await.unwrap();
        assert!(read_frame(&mut stream).await.unwrap().is_none());
    });
    let mut stream = LocalClient::new(path)
        .subscribe_with_deadline(run, 0, None, Duration::from_millis(250))
        .await
        .unwrap();
    let event = tokio::time::timeout(Duration::from_secs(2), stream.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(event["seq"], 1);
    assert_eq!(stream.last_seq, 1);
    drop(stream);
    tokio::time::timeout(Duration::from_secs(2), server)
        .await
        .unwrap()
        .unwrap();
}
