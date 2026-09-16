use super::*;

#[tokio::test]
async fn scheduling_queries_exclude_completed_history() {
    let temp = tempfile::tempdir().unwrap();
    let agent = Agent::open(&temp.path().join("agent")).unwrap();
    agent.store.set("model_config",&json!({"provider":"openai_compatible","base_url":"http://127.0.0.1:1/v1","model":"unused","api_key":null})).unwrap();
    let session = agent
        .session_operation(&Request::new("create_session").with_body(json!({})))
        .unwrap()
        .1
        .unwrap();
    let make = || {
        Request::new("submit_run")
            .with_path("session_id", session["session_id"].as_str().unwrap())
            .with_body(json!({"request_id":Uuid::new_v4(),"message":"test"}))
    };
    let finished = agent.session_operation(&make()).unwrap().1.unwrap();
    let waiting = agent.session_operation(&make()).unwrap().1.unwrap();
    agent
        .session_operation(
            &Request::new("cancel_run").with_path("run_id", finished["run_id"].as_str().unwrap()),
        )
        .unwrap();
    {
        let db = agent.store.connection.lock().unwrap();
        assert_eq!(all_records(&db, "runs").unwrap().len(), 2);
        let unfinished = all_records(&db, "unfinished_runs").unwrap();
        assert_eq!(unfinished.len(), 1);
        assert_eq!(unfinished[0]["run_id"], waiting["run_id"]);
    }
    agent.shutdown().await;
}

#[tokio::test]
async fn queue_limit_is_atomic_retries_survive_and_cancellation_releases_capacity() {
    let temp = tempfile::tempdir().unwrap();
    let agent = Agent::open(&temp.path().join("agent")).unwrap();
    agent.store.set("model_config", &json!({"provider":"openai_compatible","base_url":"http://127.0.0.1:1/v1","model":"unused","api_key":null})).unwrap();
    let session = agent
        .session_operation(&Request::new("create_session").with_body(json!({})))
        .unwrap()
        .1
        .unwrap();
    let session_id = session["session_id"].as_str().unwrap();
    let first_request = Request::new("submit_run")
        .with_path("session_id", session_id)
        .with_body(json!({"request_id":Uuid::new_v4(),"message":"first"}));
    let first = agent.session_operation(&first_request).unwrap().1.unwrap();
    // No await until shutdown: the single-thread scheduler cannot drain this
    // queue, and the placeholder provider can never receive an actual request.
    for _ in 1..MAX_QUEUED_RUNS {
        let request = Request::new("submit_run")
            .with_path("session_id", session_id)
            .with_body(json!({"request_id":Uuid::new_v4(),"message":"waiting"}));
        assert_eq!(agent.session_operation(&request).unwrap().0, 202);
    }
    let overflow = Request::new("submit_run")
        .with_path("session_id", session_id)
        .with_body(json!({"request_id":Uuid::new_v4(),"message":"one too many"}));
    let error = agent.session_operation(&overflow).unwrap_err();
    assert_eq!((error.status, error.code.as_str()), (429, "queue_full"));
    let retry = agent.session_operation(&first_request).unwrap();
    assert_eq!(retry.0, 200);
    assert_eq!(retry.1.unwrap()["run_id"], first["run_id"]);
    let mut conflict = first_request.clone();
    conflict.body.as_mut().unwrap()["message"] = json!("changed");
    assert_eq!(
        agent.session_operation(&conflict).unwrap_err().code,
        "request_conflict"
    );
    let count: i64 = agent
        .store
        .connection
        .lock()
        .unwrap()
        .query_row("SELECT count(*) FROM runs", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, MAX_QUEUED_RUNS as i64);
    let cancel = Request::new("cancel_run").with_path("run_id", first["run_id"].as_str().unwrap());
    assert_eq!(
        agent.session_operation(&cancel).unwrap().1.unwrap()["status"],
        "cancelled"
    );
    assert_eq!(agent.session_operation(&overflow).unwrap().0, 202);
    let history = agent
        .session_operation(&Request::new("list_messages").with_path("session_id", session_id))
        .unwrap()
        .1
        .unwrap();
    assert!(history["items"].as_array().unwrap().is_empty());
    agent.shutdown().await;
}

#[tokio::test]
async fn retained_cursor_snapshot_and_terminal_watermark() {
    let temp = tempfile::tempdir().unwrap();
    let agent = Agent::open(&temp.path().join("agent")).unwrap();
    agent.store.set("model_config",&json!({"provider":"openai_compatible","base_url":"http://127.0.0.1:1/v1","model":"unused","api_key":null})).unwrap();
    let session = agent
        .session_operation(&Request::new("create_session").with_body(json!({})))
        .unwrap()
        .1
        .unwrap();
    let input = Request::new("submit_run")
        .with_path("session_id", session["session_id"].as_str().unwrap())
        .with_body(json!({"request_id":Uuid::new_v4(),"message":"queued"}));
    let mut run = agent.session_operation(&input).unwrap().1.unwrap();
    let id = run["run_id"].as_str().unwrap().to_owned();
    {
        let mut db = agent.store.connection.lock().unwrap();
        let tx = db.transaction().unwrap();
        for _ in 0..4100 {
            event_tx(
                &tx,
                &mut run,
                "assistant_delta",
                json!({"text":"01234567890123456789"}),
            )
            .unwrap();
        }
        transition(&tx, &mut run, "interrupted").unwrap();
        tx.commit().unwrap();
    }
    assert_eq!(
        agent.events_after(&id, 0).unwrap_err().code,
        "event_history_expired"
    );
    assert_eq!(
        agent.events_after(&id, 99999).unwrap_err().code,
        "event_cursor_ahead"
    );
    let snapshot = agent.run_snapshot(&id).unwrap();
    assert_eq!(snapshot["snapshot_seq"], snapshot["run"]["last_seq"]);
    assert_eq!(
        snapshot["output_tail"].as_str().unwrap().chars().count(),
        65536
    );
    assert_eq!(snapshot["output_truncated"], true);
    let (events, terminal) = agent
        .events_after(&id, snapshot["snapshot_seq"].as_i64().unwrap())
        .unwrap();
    assert!(events.is_empty() && terminal);
    agent.shutdown().await;
}
