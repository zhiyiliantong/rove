#![cfg(unix)]

use rove_protocol::{
    Request, Response,
    frame::{read_frame, write_frame},
};
use rove_sdk::LocalClient;
use serde_json::{Value, json};
use tokio::net::{UnixListener, UnixStream};
use uuid::Uuid;

async fn accept(listener: &UnixListener) -> (UnixStream, Request) {
    let (mut stream, _) = listener.accept().await.unwrap();
    let hello: Request =
        serde_json::from_value(read_frame(&mut stream).await.unwrap().unwrap()).unwrap();
    assert_eq!(hello.operation_id, "get_hello");
    let response = Response::new(
        &hello,
        200,
        Some(json!({
            "device_id":Uuid::new_v4(),"display_name":"test agent","agent_version":"0.1.0",
            "protocol":{"min":1,"max":1},"capabilities":[],"network_id":null
        })),
    );
    write_frame(&mut stream, &json!(response)).await.unwrap();
    let request = serde_json::from_value(read_frame(&mut stream).await.unwrap().unwrap()).unwrap();
    (stream, request)
}

async fn open(stream: &mut UnixStream, request: &Request, run: Uuid, subscription: Uuid) {
    assert_eq!(request.operation_id, "subscribe_run_events");
    write_frame(
        stream,
        &json!(Response::new(
            request,
            200,
            Some(json!({
                "run_id":run,"subscription_id":subscription
            }))
        )),
    )
    .await
    .unwrap();
}

fn event(run: Uuid, subscription: Uuid, seq: i64, status: &str) -> Value {
    json!({"kind":"event","subscription_id":subscription,"event":{
        "run_id":run,"seq":seq,"created_at":"2026-09-08T00:00:00Z",
        "kind":"status","data":{"status":status}
    }})
}

#[tokio::test]
async fn bounded_event_batches_resume_without_resubmission_or_cancellation() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("agent.sock");
    let listener = UnixListener::bind(&path).unwrap();
    let run = Uuid::new_v4();
    let server = tokio::spawn(async move {
        let (mut stream, request) = accept(&listener).await;
        let id = Uuid::new_v4();
        open(&mut stream, &request, run, id).await;
        for seq in 1..=64 {
            write_frame(&mut stream, &event(run, id, seq, "running"))
                .await
                .unwrap();
        }
        assert!(read_frame(&mut stream).await.unwrap().is_none());
        let (mut stream, request) = accept(&listener).await;
        assert_eq!(request.query_parameters["after_seq"], 64);
        write_frame(&mut stream, &json!(Response::new(&request, 204, None)))
            .await
            .unwrap();
    });
    let client = LocalClient::new(path);
    let batch = client.poll_events(run, 0, None).await.unwrap();
    assert_eq!(batch["events"].as_array().unwrap().len(), 64);
    assert_eq!(batch["last_seq"], 64);
    assert_eq!(batch["terminal"], false);
    let end = client.poll_events(run, 64, None).await.unwrap();
    assert_eq!(end["terminal"], true);
    assert!(end["events"].as_array().unwrap().is_empty());
    server.await.unwrap();
}

#[tokio::test]
async fn reconnect_uses_watermark_deduplicates_and_handles_terminal_204() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("agent.sock");
    let listener = UnixListener::bind(&path).unwrap();
    let run = Uuid::new_v4();
    let target = rove_protocol::SocketTarget {
        network_id: Uuid::new_v4(),
        device_id: Uuid::new_v4(),
    };
    let expected_target = target.clone();
    let server = tokio::spawn(async move {
        let (mut stream, request) = accept(&listener).await;
        assert_eq!(json!(request.target), json!(expected_target));
        assert_eq!(request.query_parameters["after_seq"], 0);
        let id = Uuid::new_v4();
        open(&mut stream, &request, run, id).await;
        write_frame(&mut stream, &event(run, id, 1, "running"))
            .await
            .unwrap();
        drop(stream);

        let (mut stream, request) = accept(&listener).await;
        assert_eq!(json!(request.target), json!(expected_target));
        assert_eq!(request.path_parameters["run_id"], run.to_string());
        assert_eq!(request.query_parameters["after_seq"], 1);
        let id = Uuid::new_v4();
        open(&mut stream, &request, run, id).await;
        write_frame(&mut stream, &event(run, id, 1, "running"))
            .await
            .unwrap();
        write_frame(&mut stream, &event(run, id, 2, "succeeded"))
            .await
            .unwrap();
        write_frame(
            &mut stream,
            &json!({"kind":"stream_end","subscription_id":id,"reason":"terminal","error":null}),
        )
        .await
        .unwrap();
        drop(stream);

        let (mut stream, request) = accept(&listener).await;
        assert_eq!(json!(request.target), json!(expected_target));
        assert_eq!(request.query_parameters["after_seq"], 2);
        write_frame(&mut stream, &json!(Response::new(&request, 204, None)))
            .await
            .unwrap();
    });
    let client = LocalClient::new(path);
    let mut subscription = client.subscribe_target(run, 0, Some(target)).await.unwrap();
    assert_eq!(subscription.next().await.unwrap().unwrap()["seq"], 1);
    assert!(
        subscription
            .next()
            .await
            .unwrap_err()
            .to_string()
            .contains("disconnected")
    );
    let mut resumed = client.resume(&subscription).await.unwrap();
    assert_eq!(resumed.next().await.unwrap().unwrap()["seq"], 2);
    assert!(resumed.next().await.unwrap().is_none());
    assert!(resumed.terminal);
    let mut ended = client.resume(&resumed).await.unwrap();
    assert!(ended.terminal && ended.next().await.unwrap().is_none());
    server.await.unwrap();
}

#[tokio::test]
async fn wrong_opened_run_and_sequence_gaps_are_rejected() {
    for wrong_run in [true, false] {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("agent.sock");
        let listener = UnixListener::bind(&path).unwrap();
        let run = Uuid::new_v4();
        let server = tokio::spawn(async move {
            let (mut stream, request) = accept(&listener).await;
            let id = Uuid::new_v4();
            open(
                &mut stream,
                &request,
                if wrong_run { Uuid::new_v4() } else { run },
                id,
            )
            .await;
            if !wrong_run {
                write_frame(&mut stream, &event(run, id, 2, "running"))
                    .await
                    .unwrap();
            }
        });
        let result = LocalClient::new(path).subscribe(run, 0).await;
        if wrong_run {
            assert!(
                result
                    .err()
                    .unwrap()
                    .to_string()
                    .contains("Unexpected run_id")
            );
        } else {
            let mut subscription = result.unwrap();
            assert!(
                subscription
                    .next()
                    .await
                    .unwrap_err()
                    .to_string()
                    .contains("sequence gap")
            );
            assert_eq!(subscription.last_seq, 0);
        }
        server.await.unwrap();
    }
}

#[tokio::test]
async fn unsubscribe_requires_acknowledgement() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("agent.sock");
    let listener = UnixListener::bind(&path).unwrap();
    let run = Uuid::new_v4();
    let server = tokio::spawn(async move {
        let (mut stream, request) = accept(&listener).await;
        open(&mut stream, &request, run, Uuid::new_v4()).await;
        let unsubscribe = read_frame(&mut stream).await.unwrap().unwrap();
        assert_eq!(unsubscribe["kind"], "unsubscribe");
        // Simulate the connection dropping before acknowledgement.
    });
    let mut subscription = LocalClient::new(path).subscribe(run, 0).await.unwrap();
    assert!(
        subscription
            .unsubscribe()
            .await
            .unwrap_err()
            .to_string()
            .contains("acknowledgement")
    );
    server.await.unwrap();
}

#[tokio::test]
async fn cancelled_partial_event_read_can_be_followed_by_unsubscribe() {
    use tokio::io::AsyncWriteExt;
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("agent.sock");
    let listener = UnixListener::bind(&path).unwrap();
    let run = Uuid::new_v4();
    let server = tokio::spawn(async move {
        let (mut stream, request) = accept(&listener).await;
        let id = Uuid::new_v4();
        open(&mut stream, &request, run, id).await;
        let mut wire = Vec::new();
        write_frame(&mut wire, &event(run, id, 1, "running"))
            .await
            .unwrap();
        stream.write_all(&wire[..2]).await.unwrap();
        // Leave a partial prefix pending until the client changes its mind.
        let unsubscribe = read_frame(&mut stream).await.unwrap().unwrap();
        assert_eq!(unsubscribe["kind"], "unsubscribe");
        stream.write_all(&wire[2..]).await.unwrap();
        write_frame(&mut stream, &json!({"kind":"response", "correlation_id":unsubscribe["correlation_id"], "status_code":204})).await.unwrap();
    });
    let mut subscription = LocalClient::new(path).subscribe(run, 0).await.unwrap();
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(25), subscription.next())
            .await
            .is_err()
    );
    tokio::time::timeout(
        std::time::Duration::from_secs(5),
        subscription.unsubscribe(),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(subscription.last_seq, 0); // No event was delivered to the caller.
    server.await.unwrap();
}
