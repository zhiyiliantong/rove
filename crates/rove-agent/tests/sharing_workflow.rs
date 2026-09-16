use rove_agent::Agent;
use rove_protocol::Request;
use serde_json::json;

#[tokio::test]
async fn real_http_encrypted_share_reusable_and_atomic_import() {
    let temp = tempfile::tempdir().unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let blobs = rove_config_server::BlobStore::open(
        &temp.path().join("blobs.db"),
        base.parse().unwrap(),
        1024 * 1024,
        10,
    )
    .unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, rove_config_server::router(blobs))
            .await
            .unwrap();
    });
    let a = Agent::open(&temp.path().join("a")).unwrap();
    let b = Agent::open(&temp.path().join("b")).unwrap();
    let c = Agent::open(&temp.path().join("c")).unwrap();
    let created = a
        .handle(&Request::new("create_network").with_body(json!({"display_name":"shared"})))
        .await;
    assert_eq!(created.status_code, 201);
    let network = created.body.unwrap();
    let id = network["network_id"].as_str().unwrap();
    let shared = a
        .handle(
            &Request::new("create_network_share")
                .with_path("network_id", id)
                .with_body(json!({"config_server_url":base})),
        )
        .await;
    assert_eq!(shared.status_code, 201);
    let share = shared.body.unwrap();
    let url = share["url"].as_str().unwrap();
    // A real camera decoder recovers the same import payload, including the key.
    let qr = rove_sdk::ShareQr::new(url).unwrap();
    let side = (qr.size() + 8) * 6;
    let pixels: Vec<u8> = (0..side * side)
        .map(|i| {
            if qr.is_dark(((i % side) / 6) as i32 - 4, ((i / side) / 6) as i32 - 4) {
                0
            } else {
                255
            }
        })
        .collect();
    let mut decoder = quircs::Quirc::default();
    let scanned = decoder
        .identify(side, side, &pixels)
        .next()
        .unwrap()
        .unwrap()
        .decode()
        .unwrap();
    let scanned_url = String::from_utf8(scanned.payload).unwrap();
    assert_eq!(scanned_url, url);
    let (download_url, _) = rove_agent::sharing::parse_share_url(url).unwrap();
    let raw = reqwest::get(download_url).await.unwrap();
    assert_eq!(raw.headers()["cache-control"], "no-store");
    let envelope = raw.text().await.unwrap();
    assert!(!envelope.contains("network_secret"));
    assert!(!envelope.contains("network_id"));
    for agent in [&b, &c] {
        let response = agent
            .handle(
                &Request::new("import_network")
                    .with_body(json!({"source":"url","url":scanned_url})),
            )
            .await;
        assert_eq!(response.status_code, 200);
        let imported = response.body.unwrap();
        assert_eq!(imported["network"]["network_id"], id);
        assert_ne!(imported["network"]["instance_id"], network["instance_id"]);
        assert_ne!(agent.store.device_id, a.store.device_id);
    }
    let original = b
        .handle(&Request::new("get_network_join_config").with_path("network_id", id))
        .await
        .body;
    let wrong = format!("{}#key={}", url.split('#').next().unwrap(), "A".repeat(43));
    let failed = b
        .handle(&Request::new("import_network").with_body(json!({"source":"url","url":wrong})))
        .await;
    assert_eq!(failed.status_code, 422);
    assert_eq!(
        b.handle(&Request::new("get_network_join_config").with_path("network_id", id))
            .await
            .body,
        original
    );
    let overflow = reqwest::Client::new()
        .post(format!("{base}/v1/blobs"))
        .body(vec![0; 262145])
        .send()
        .await
        .unwrap();
    assert_eq!(overflow.status(), 413);
    assert_eq!(overflow.headers()["cache-control"], "no-store");
    // Advance this test-owned blob's expiry without waiting seven real days.
    let blob_id = share["blob_id"].as_str();
    let parsed = reqwest::Url::parse(url).unwrap();
    let blob_id = blob_id.unwrap_or_else(|| parsed.path().rsplit('/').next().unwrap());
    let db = rusqlite::Connection::open(temp.path().join("blobs.db")).unwrap();
    assert_eq!(
        db.execute(
            "UPDATE blobs SET expires_at='2000-01-01T00:00:00+00:00' WHERE id=?1",
            [blob_id]
        )
        .unwrap(),
        1
    );
    let expired = reqwest::get(rove_agent::sharing::parse_share_url(url).unwrap().0)
        .await
        .unwrap();
    assert_eq!(expired.status(), 410);
    let failed = b
        .handle(
            &Request::new("import_network").with_body(json!({"source":"url","url":scanned_url})),
        )
        .await;
    assert!(failed.status_code >= 400);
    for agent in [&b, &c] {
        assert_eq!(
            agent
                .handle(&Request::new("get_network_join_config").with_path("network_id", id))
                .await
                .body,
            original
        );
    }
    server.abort();
    let _ = server.await;
    // Manual import does not contact the now unavailable public service.
    let manual = b
        .handle(
            &Request::new("import_network")
                .with_body(json!({"source":"manual","config":original.unwrap()})),
        )
        .await;
    assert_eq!(manual.status_code, 200);
}
