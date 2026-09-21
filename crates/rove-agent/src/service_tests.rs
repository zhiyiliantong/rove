use super::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

async fn setup() -> (tempfile::TempDir, Arc<Agent>, String) {
    let temp = tempfile::tempdir().unwrap();
    let agent = Agent::open(&temp.path().join("agent")).unwrap();
    let network = agent
        .handle(&Request::new("create_network").with_body(json!({"display_name":"proxy fixture"})))
        .await
        .body
        .unwrap();
    (
        temp,
        agent,
        network["network_id"].as_str().unwrap().to_owned(),
    )
}
async fn echo() -> (SocketAddr, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let task = tokio::spawn(async move {
        let mut clients = JoinSet::new();
        loop {
            tokio::select! {
                accepted=listener.accept()=>{let (mut stream,_)=accepted.unwrap();clients.spawn(async move{let (mut r,mut w)=stream.split();let _=tokio::io::copy(&mut r,&mut w).await;});},
                _=clients.join_next(),if !clients.is_empty()=>{},
            }
        }
    });
    (address, task)
}
async fn roundtrip(address: SocketAddr) {
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        let mut stream = TcpStream::connect(address).await.unwrap();
        let data = "TCP 双向转发，不修改应用认证".as_bytes();
        stream.write_all(data).await.unwrap();
        stream.shutdown().await.unwrap();
        let mut response = Vec::new();
        stream.read_to_end(&mut response).await.unwrap();
        assert_eq!(response, data);
    })
    .await
    .unwrap();
}
fn body(network: &str, address: SocketAddr) -> Value {
    json!({"network_id":network,"name":"test application","protocol":"http","target":{"host":address.ip().to_string(),"port":address.port()},"access_info":"application credentials stay opaque"})
}
fn endpoint(service: &Value) -> SocketAddr {
    let url = url::Url::parse(service["endpoints"][0].as_str().unwrap()).unwrap();
    SocketAddr::new(
        url.host_str().unwrap().parse().unwrap(),
        url.port().unwrap(),
    )
}

#[tokio::test]
async fn publication_is_fail_closed_then_forwards_and_unpublishes_idempotently() {
    let (_temp, agent, network) = setup().await;
    let (application, server) = echo().await;
    let request = Request::new("publish_service").with_body(body(&network, application));
    assert_eq!(agent.handle(&request).await.status_code, 503);
    // Test-only injection exercises proxy I/O, not EasyTier or overlay isolation.
    agent
        .services
        .0
        .lock()
        .await
        .addresses
        .insert(network.clone(), "127.0.0.1".parse().unwrap());
    let response = agent.handle(&request).await;
    assert_eq!(response.status_code, 201, "{response:?}");
    let service = response.body.unwrap();
    let address = endpoint(&service);
    assert_ne!(address.port(), application.port());
    assert_eq!(service["target_status"], "unknown");
    roundtrip(address).await;
    let id = service["service_id"].as_str().unwrap();
    let get = Request::new("get_service").with_path("service_id", id);
    assert_eq!(
        agent.handle(&get).await.body.unwrap()["access_info"],
        service["access_info"]
    );
    for _ in 0..2 {
        assert_eq!(
            agent
                .handle(&Request::new("unpublish_service").with_path("service_id", id))
                .await
                .status_code,
            204
        );
    }
    assert!(TcpStream::connect(address).await.is_err());
    assert_eq!(agent.handle(&get).await.status_code, 404);
    roundtrip(application).await;
    server.abort();
    let _ = server.await;
    agent.shutdown().await;
}

#[tokio::test]
async fn management_tool_updates_and_unpublishes_without_stopping_application() {
    let (_temp, agent, network) = setup().await;
    let (application, server) = echo().await;
    agent
        .services
        .0
        .lock()
        .await
        .addresses
        .insert(network.clone(), "127.0.0.1".parse().unwrap());
    let published = crate::management::call(
        &agent,
        &json!({"operation_id":"publish_service","body":body(&network,application)}),
    )
    .await
    .unwrap()
    .unwrap();
    let id = published["service_id"].as_str().unwrap();
    let mut update = body(&network, application);
    update["name"] = json!("Renamed through dialogue");
    let updated = crate::management::call(
        &agent,
        &json!({"operation_id":"update_service","path_parameters":{"service_id":id},"body":update}),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(updated["name"], "Renamed through dialogue");
    roundtrip(endpoint(&updated)).await;
    let listed = crate::management::call(
        &agent,
        &json!({"operation_id":"list_services","query_parameters":{"network_id":network}}),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(listed["items"][0]["service_id"], id);
    crate::management::call(
        &agent,
        &json!({"operation_id":"unpublish_service","path_parameters":{"service_id":id}}),
    )
    .await
    .unwrap();
    assert!(TcpStream::connect(endpoint(&updated)).await.is_err());
    roundtrip(application).await;
    server.abort();
    agent.shutdown().await;
}

#[tokio::test]
async fn update_conflicts_and_failed_persistence_keep_old_listener() {
    let (_temp, agent, network) = setup().await;
    agent
        .services
        .0
        .lock()
        .await
        .addresses
        .insert(network.clone(), "127.0.0.1".parse().unwrap());
    let (application, server) = echo().await;
    let original = body(&network, application);
    let service = agent
        .handle(&Request::new("publish_service").with_body(original.clone()))
        .await
        .body
        .unwrap();
    let address = endpoint(&service);
    let id = service["service_id"].as_str().unwrap();
    let occupied = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mut replacement = original.clone();
    replacement["listen_port"] = json!(occupied.local_addr().unwrap().port());
    replacement["name"] = json!("new name");
    let update = |body| {
        Request::new("update_service")
            .with_path("service_id", id)
            .with_body(body)
    };
    assert_eq!(
        agent.handle(&update(replacement.clone())).await.status_code,
        409
    );
    roundtrip(address).await;
    let candidate = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let candidate_address = candidate.local_addr().unwrap();
    drop(candidate);
    replacement["listen_port"] = json!(candidate_address.port());
    agent.store.connection.lock().unwrap().execute_batch("CREATE TRIGGER reject_service_update BEFORE UPDATE ON services BEGIN SELECT RAISE(ABORT,'test persistence fault'); END;").unwrap();
    assert_eq!(
        agent.handle(&update(replacement.clone())).await.status_code,
        500
    );
    let released = TcpListener::bind(candidate_address).await.unwrap();
    roundtrip(address).await;
    agent
        .store
        .connection
        .lock()
        .unwrap()
        .execute_batch("DROP TRIGGER reject_service_update;")
        .unwrap();
    drop(released);
    let updated = agent.handle(&update(replacement)).await;
    assert_eq!(updated.status_code, 200, "{updated:?}");
    let updated = updated.body.unwrap();
    assert_eq!(updated["service_id"], id);
    assert_eq!(updated["network_id"], network);
    assert_eq!(updated["name"], "new name");
    assert!(TcpStream::connect(address).await.is_err());
    roundtrip(endpoint(&updated)).await;
    let mut foreign = original.clone();
    foreign["network_id"] = json!(uuid::Uuid::new_v4());
    assert_eq!(agent.handle(&update(foreign)).await.status_code, 409);
    // Same-port update does not close the listener while replacing metadata.
    let mut same = original;
    same["access_info"] = json!("replacement app account");
    let same = agent.handle(&update(same)).await;
    assert_eq!(same.status_code, 200);
    assert_eq!(same.body.unwrap()["listen_port"], updated["listen_port"]);
    agent.shutdown().await;
    server.abort();
    let _ = server.await;
}

#[tokio::test]
async fn stopping_and_restarting_never_report_stale_endpoints() {
    let (temp, agent, network) = setup().await;
    agent
        .services
        .0
        .lock()
        .await
        .addresses
        .insert(network.clone(), "127.0.0.1".parse().unwrap());
    let (application, server) = echo().await;
    let service = agent
        .handle(&Request::new("publish_service").with_body(body(&network, application)))
        .await
        .body
        .unwrap();
    let address = endpoint(&service);
    let id = service["service_id"].as_str().unwrap();
    assert_eq!(
        agent
            .handle(&Request::new("stop_network").with_path("network_id", &network))
            .await
            .status_code,
        200
    );
    assert!(TcpStream::connect(address).await.is_err());
    let get = Request::new("get_service").with_path("service_id", id);
    let view = agent.handle(&get).await.body.unwrap();
    assert_eq!(view["state"], "unavailable");
    assert_eq!(view["endpoints"], json!([]));
    assert_eq!(view["listen_port"], address.port());
    agent.shutdown().await;
    drop(agent);
    let restarted = Agent::open(&temp.path().join("agent")).unwrap();
    let view = restarted.handle(&get).await.body.unwrap();
    assert_eq!(view["service_id"], id);
    assert_eq!(view["state"], "unavailable");
    assert_eq!(view["endpoints"], json!([]));
    // The network-ready hook reuses the saved port; a conflict stays explicit.
    let conflict = TcpListener::bind(address).await.unwrap();
    restarted
        .reconcile_service_network(&network, Some(address.ip()))
        .await
        .unwrap();
    let view = restarted.handle(&get).await.body.unwrap();
    assert_eq!(view["state"], "failed");
    assert_eq!(view["last_error"]["code"], "service_bind_failed");
    assert_eq!(view["listen_port"], address.port());
    assert_eq!(view["endpoints"], json!([]));
    drop(conflict);
    restarted
        .reconcile_service_network(&network, Some(address.ip()))
        .await
        .unwrap();
    let view = restarted.handle(&get).await.body.unwrap();
    assert_eq!(view["service_id"], id);
    assert_eq!(endpoint(&view), address);
    roundtrip(address).await;
    #[cfg(target_os = "linux")]
    {
        // Linux provides 127/8 without creating or modifying an interface.
        let changed: IpAddr = "127.0.0.2".parse().unwrap();
        restarted
            .reconcile_service_network(&network, Some(changed))
            .await
            .unwrap();
        let view = restarted.handle(&get).await.body.unwrap();
        assert_eq!(view["service_id"], id);
        assert_eq!(endpoint(&view).ip(), changed);
        assert_eq!(endpoint(&view).port(), address.port());
        assert!(TcpStream::connect(address).await.is_err());
        roundtrip(endpoint(&view)).await;
    }
    roundtrip(application).await;
    restarted.shutdown().await;
    server.abort();
    let _ = server.await;
}

#[tokio::test]
async fn create_failure_releases_port_and_never_persists_a_phantom_service() {
    let (_temp, agent, network) = setup().await;
    agent
        .services
        .0
        .lock()
        .await
        .addresses
        .insert(network.clone(), "127.0.0.1".parse().unwrap());
    let reserved = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = reserved.local_addr().unwrap();
    drop(reserved);
    let mut input = body(&network, "127.0.0.1:1".parse().unwrap());
    input["listen_port"] = json!(address.port());
    agent.store.connection.lock().unwrap().execute_batch("CREATE TRIGGER reject_service_insert BEFORE INSERT ON services BEGIN SELECT RAISE(ABORT,'test persistence fault'); END;").unwrap();
    assert_eq!(
        agent
            .handle(&Request::new("publish_service").with_body(input.clone()))
            .await
            .status_code,
        500
    );
    let _released = TcpListener::bind(address).await.unwrap();
    let mut list = Request::new("list_services");
    list.query_parameters
        .insert("network_id".into(), json!(network));
    assert_eq!(agent.handle(&list).await.body.unwrap()["items"], json!([]));
    input["target"]["host"] = json!("192.0.2.1");
    assert_eq!(
        agent
            .handle(&Request::new("publish_service").with_body(input))
            .await
            .status_code,
        400
    );
    agent.shutdown().await;
}

#[tokio::test]
async fn rig_publication_uses_the_same_catalog_and_proxy_as_the_api() {
    use axum::{Json, Router, http::header, routing::post};
    let (_temp, agent, network) = setup().await;
    agent
        .services
        .0
        .lock()
        .await
        .addresses
        .insert(network.clone(), "127.0.0.1".parse().unwrap());
    let (application, app_server) = echo().await;
    let arguments = body(&network, application);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}/v1", listener.local_addr().unwrap());
    let provider = tokio::spawn(async move {
        let router=Router::new().route("/v1/chat/completions",post(move |Json(input):Json<Value>| {
            let arguments=arguments.clone();
            async move {
                let done=input["messages"].as_array().unwrap().iter().any(|m|m["role"]=="tool");
                let delta=if done{json!({"role":"assistant","content":"service publication complete"})}else{json!({"role":"assistant","tool_calls":[{"index":0,"id":"service_call","type":"function","function":{"name":"publish_service","arguments":arguments.to_string()}}]})};
                let first=json!({"id":"service-test","object":"chat.completion.chunk","created":1,"model":"test","choices":[{"index":0,"delta":delta,"finish_reason":null}]});
                let end=json!({"id":"service-test","object":"chat.completion.chunk","created":1,"model":"test","choices":[{"index":0,"delta":{},"finish_reason":if done{"stop"}else{"tool_calls"}}]});
                ([(header::CONTENT_TYPE,"text/event-stream")],format!("data: {first}\n\ndata: {end}\n\ndata: [DONE]\n\n"))
            }
        }));
        axum::serve(listener, router).await.unwrap();
    });
    assert_eq!(agent.handle(&Request::new("set_model_config").with_body(json!({"provider":"openai_compatible","base_url":base,"model":"test","api_key":null}))).await.status_code,200);
    let session = agent
        .handle(&Request::new("create_session").with_body(json!({})))
        .await
        .body
        .unwrap();
    let session_id = session["session_id"].as_str().unwrap();
    let run=agent.handle(&Request::new("submit_run").with_path("session_id",session_id).with_body(json!({"request_id":uuid::Uuid::new_v4(),"message":"publish the test application"}))).await.body.unwrap();
    let run_id = run["run_id"].as_str().unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            let snapshot = agent.run_snapshot(run_id).unwrap();
            if crate::sessions::terminal(&snapshot["run"]) {
                assert_eq!(snapshot["run"]["status"], "succeeded", "{snapshot}");
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let mut list = Request::new("list_services");
    list.query_parameters
        .insert("network_id".into(), json!(network));
    let services = agent.handle(&list).await.body.unwrap();
    assert_eq!(services["items"].as_array().unwrap().len(), 1);
    let service = &services["items"][0];
    roundtrip(endpoint(service)).await;
    let history = agent
        .handle(&Request::new("list_messages").with_path("session_id", session_id))
        .await
        .body
        .unwrap();
    assert!(
        history
            .to_string()
            .contains(service["service_id"].as_str().unwrap())
    );
    agent.shutdown().await;
    provider.abort();
    app_server.abort();
    let _ = provider.await;
    let _ = app_server.await;
}
