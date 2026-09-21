use super::*;

#[tokio::test]
async fn message_titles_are_persistent_atomic_and_never_replace_manual_names() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("agent");
    let agent = Agent::open(&path).unwrap();
    let session = agent
        .session_operation(
            &Request::new("create_session").with_body(json!({"title":"新会话","auto_title":true})),
        )
        .unwrap()
        .1
        .unwrap();
    let id = session["session_id"].as_str().unwrap();
    let submit = Request::new("submit_run")
        .with_path("session_id", id)
        .with_body(
            json!({"request_id":Uuid::new_v4(),"message":"  建立私人影院\n 在家观看电影  "}),
        );
    assert_eq!(
        agent.session_operation(&submit).unwrap_err().code,
        "model_not_configured"
    );
    let read = || {
        agent
            .session_operation(&Request::new("get_session").with_path("session_id", id))
            .unwrap()
            .1
            .unwrap()
    };
    assert_eq!(read()["title_source"], "default");
    agent.store.set("model_config", &json!({"provider":"openai_compatible","base_url":"http://127.0.0.1:1/v1","model":"unused","api_key":null})).unwrap();
    agent.session_operation(&submit).unwrap();
    assert_eq!(read()["title"], "建立私人影院 在家观看电影");
    assert_eq!(read()["title_source"], "message");
    AGENT.validate("Session", &read()).unwrap();
    let patch = Request::new("update_session").with_path("session_id", id);
    agent
        .session_operation(&patch.clone().with_body(json!({"title":" 新会话 "})))
        .unwrap();
    agent.session_operation(&submit).unwrap();
    agent
        .session_operation(
            &Request::new("submit_run")
                .with_path("session_id", id)
                .with_body(json!({"request_id":Uuid::new_v4(),"message":"另一条消息"})),
        )
        .unwrap();
    assert_eq!(read()["title"], "新会话");
    assert_eq!(read()["title_source"], "manual");
    assert!(
        agent
            .session_operation(&patch.with_body(json!({"title":" \n "})))
            .is_err()
    );
    let untouched = agent
        .session_operation(&Request::new("create_session").with_body(json!({"title":"自定义"})))
        .unwrap()
        .1
        .unwrap();
    assert_eq!(untouched["title_source"], "manual");
    let fixed = agent
        .session_operation(
            &Request::new("create_session").with_body(json!({"auto_title":true,"title":"新会话"})),
        )
        .unwrap()
        .1
        .unwrap();
    let fid = fixed["session_id"].as_str().unwrap();
    agent
        .session_operation(
            &Request::new("update_session")
                .with_path("session_id", fid)
                .with_body(json!({"title":"新会话"})),
        )
        .unwrap();
    agent
        .session_operation(
            &Request::new("submit_run")
                .with_path("session_id", fid)
                .with_body(json!({"request_id":Uuid::new_v4(),"message":"不要覆盖我手动选的名称"})),
        )
        .unwrap();
    let fixed = agent
        .session_operation(&Request::new("get_session").with_path("session_id", fid))
        .unwrap()
        .1
        .unwrap();
    assert_eq!(fixed["title"], "新会话");
    assert_eq!(fixed["title_source"], "manual");
    let unnamed = agent
        .session_operation(&Request::new("create_session").with_body(json!({})))
        .unwrap()
        .1
        .unwrap();
    assert_eq!(unnamed["title_source"], "default");
    let uid = unnamed["session_id"].as_str().unwrap();
    agent
        .session_operation(
            &Request::new("submit_run")
                .with_path("session_id", uid)
                .with_body(json!({"request_id":Uuid::new_v4(),"message":"漫游🚀".repeat(20)})),
        )
        .unwrap();
    let generated = agent
        .session_operation(&Request::new("get_session").with_path("session_id", uid))
        .unwrap()
        .1
        .unwrap();
    assert_eq!(generated["title"].as_str().unwrap().chars().count(), 33);
    assert!(generated["title"].as_str().unwrap().ends_with('…'));
    // Old records have no reliable manual/automatic marker: never guess from text.
    agent
        .store
        .connection
        .lock()
        .unwrap()
        .execute(
            "UPDATE sessions SET record=json_remove(record,'$.title_source') WHERE id=?1",
            [id],
        )
        .unwrap();
    title_from_message(
        &agent.store.connection.lock().unwrap(),
        id,
        "不要覆盖旧标题",
    )
    .unwrap();
    assert_eq!(read()["title"], "新会话");
    let uid = uid.to_string();
    agent.shutdown().await;
    drop(agent);
    let reopened = Agent::open(&path).unwrap();
    let after = reopened
        .session_operation(&Request::new("get_session").with_path("session_id", uid))
        .unwrap()
        .1
        .unwrap();
    assert_eq!(after["title_source"], "message");
    assert_eq!(after["title"], generated["title"]);
    reopened.shutdown().await;
}

#[tokio::test]
async fn descending_session_pages_are_stable_across_insertions_and_deletions() {
    let temp = tempfile::tempdir().unwrap();
    let agent = Agent::open(&temp.path().join("agent")).unwrap();
    let create = |id: &str, date: &str| {
        let value = json!({"session_id":id,"created_at":date,"title":id,"archived_at":null});
        agent
            .store
            .connection
            .lock()
            .unwrap()
            .execute(
                "INSERT INTO sessions(id,record) VALUES(?1,?2)",
                params![id, value.to_string()],
            )
            .unwrap();
    };
    create("a", "2026-09-18T00:00:00Z");
    create("b", "2026-09-18T00:00:00Z");
    create("c", "2026-09-19T00:00:00Z");
    let mut request = Request::new("list_sessions");
    request
        .query_parameters
        .insert("order".into(), json!("desc"));
    request
        .query_parameters
        .insert("archived".into(), json!(false));
    request.query_parameters.insert("limit".into(), json!(1));
    let first = agent.session_operation(&request).unwrap().1.unwrap();
    assert_eq!(first["items"][0]["session_id"], "c");
    request
        .query_parameters
        .insert("cursor".into(), first["next_cursor"].clone());
    create("d", "2026-09-20T00:00:00Z");
    agent
        .store
        .connection
        .lock()
        .unwrap()
        .execute("DELETE FROM sessions WHERE id='c'", [])
        .unwrap();
    let second = agent.session_operation(&request).unwrap().1.unwrap();
    assert_eq!(second["items"][0]["session_id"], "b");
    request
        .query_parameters
        .insert("cursor".into(), second["next_cursor"].clone());
    let third = agent.session_operation(&request).unwrap().1.unwrap();
    assert_eq!(third["items"][0]["session_id"], "a");
    assert!(third["next_cursor"].is_null());
    request
        .query_parameters
        .insert("order".into(), json!("asc"));
    assert_eq!(
        agent.session_operation(&request).unwrap_err().code,
        "invalid_cursor"
    );
    request.query_parameters.remove("cursor");
    let ascending = agent.session_operation(&request).unwrap().1.unwrap();
    assert_eq!(ascending["items"][0]["session_id"], "a");
    agent
        .session_operation(&Request::new("archive_session").with_path("session_id", "b"))
        .unwrap();
    request
        .query_parameters
        .insert("order".into(), json!("desc"));
    request
        .query_parameters
        .insert("archived".into(), json!(true));
    let archived = agent.session_operation(&request).unwrap().1.unwrap();
    assert_eq!(archived["items"][0]["session_id"], "b");
    assert!(archived["next_cursor"].is_null());
    agent.shutdown().await;
}

#[tokio::test]
async fn archive_protects_live_jobs_and_deletion_retains_only_request_tombstones() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("agent");
    let agent = Agent::open(&path).unwrap();
    agent.store.set("model_config", &json!({"provider":"openai_compatible","base_url":"http://127.0.0.1:1/v1","model":"unused","api_key":null})).unwrap();
    let session = agent
        .session_operation(&Request::new("create_session").with_body(json!({"title":"archive"})))
        .unwrap()
        .1
        .unwrap();
    let id = session["session_id"].as_str().unwrap();
    let op = |name| Request::new(name).with_path("session_id", id);
    assert_eq!(
        agent
            .session_operation(&op("delete_session"))
            .unwrap_err()
            .code,
        "session_not_archived"
    );
    let request =
        op("submit_run").with_body(json!({"request_id":Uuid::new_v4(),"message":"private input"}));
    let run = agent.session_operation(&request).unwrap().1.unwrap();
    assert_eq!(
        agent.session_operation(&op("archive_session")).unwrap(),
        (204, None)
    );
    let archived = agent
        .session_operation(&op("get_session"))
        .unwrap()
        .1
        .unwrap();
    assert!(archived["archived_at"].is_string());
    agent.session_operation(&op("archive_session")).unwrap();
    assert_eq!(
        agent
            .session_operation(&op("get_session"))
            .unwrap()
            .1
            .unwrap()["archived_at"],
        archived["archived_at"]
    );
    assert_eq!(
        agent.session_operation(&request).unwrap().1.unwrap()["run_id"],
        run["run_id"]
    );
    let new = op("submit_run").with_body(json!({"request_id":Uuid::new_v4(),"message":"new"}));
    assert_eq!(
        agent.session_operation(&new).unwrap_err().code,
        "session_archived"
    );
    assert_eq!(
        agent
            .session_operation(&op("update_session").with_body(json!({"title":"no"})))
            .unwrap_err()
            .code,
        "session_archived"
    );
    assert_eq!(
        agent
            .session_operation(&op("delete_session"))
            .unwrap_err()
            .code,
        "session_has_active_runs"
    );
    let mut list = Request::new("list_sessions");
    list.query_parameters
        .insert("archived".into(), json!(false));
    assert!(
        agent.session_operation(&list).unwrap().1.unwrap()["items"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    agent.session_operation(&op("restore_session")).unwrap();
    assert_eq!(
        agent.session_operation(&list).unwrap().1.unwrap()["items"][0]["session_id"],
        id
    );
    agent
        .session_operation(
            &Request::new("cancel_run").with_path("run_id", run["run_id"].as_str().unwrap()),
        )
        .unwrap();
    agent.session_operation(&op("archive_session")).unwrap();
    assert_eq!(
        agent.session_operation(&op("delete_session")).unwrap(),
        (204, None)
    );
    assert_eq!(
        agent.session_operation(&request).unwrap_err().code,
        "request_deleted"
    );
    for name in ["get_session", "restore_session", "delete_session"] {
        assert_eq!(agent.session_operation(&op(name)).unwrap_err().status, 404);
    }
    {
        let db = agent.store.connection.lock().unwrap();
        for table in [
            "sessions",
            "runs",
            "messages",
            "rig_messages",
            "run_events",
            "run_output",
        ] {
            let count: i64 = db
                .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
                .unwrap();
            assert_eq!(count, 0, "{table}");
        }
        let count: i64 = db
            .query_row("SELECT count(*) FROM deleted_requests", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }
    agent.shutdown().await;
    drop(agent);
    let reopened = Agent::open(&path).unwrap();
    assert_eq!(
        reopened.session_operation(&request).unwrap_err().code,
        "request_deleted"
    );
    reopened.shutdown().await;
}

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
