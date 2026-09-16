#![cfg(unix)]
use rove_protocol::{
    Request, Response,
    frame::{read_frame, write_frame},
};
use serde_json::json;
#[tokio::test]
async fn incompatible_peer_does_not_receive_business_request() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("agent.sock");
    let listener = tokio::net::UnixListener::bind(&path).unwrap();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let hello: Request =
            serde_json::from_value(read_frame(&mut stream).await.unwrap().unwrap()).unwrap();
        assert_eq!(hello.operation_id, "get_hello");
        let response = Response::new(
            &hello,
            200,
            Some(
                json!({"device_id":uuid::Uuid::new_v4(),"display_name":"newer agent","agent_version":"9.0.0","protocol":{"min":2,"max":3},"capabilities":[],"network_id":null}),
            ),
        );
        write_frame(&mut stream, &json!(response)).await.unwrap();
        assert!(read_frame(&mut stream).await.unwrap().is_none());
    });
    let error = rove_sdk::LocalClient::new(path)
        .call(Request::new("get_device"))
        .await
        .unwrap_err();
    assert!(error.to_string().contains("protocol_incompatible"));
    server.await.unwrap();
}
