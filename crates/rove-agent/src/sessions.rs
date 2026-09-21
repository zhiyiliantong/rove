use crate::{Agent, storage_error};
use rove_protocol::{ApiError, Request, contract::AGENT};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

const MAX_QUEUED_RUNS: usize = 1024;
const RETAIN_EVENTS: i64 = 4096;
#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;
pub struct Runtime {
    pub active: Mutex<HashMap<String, CancellationToken>>,
    pub wake: Arc<Notify>,
    pub stopping: CancellationToken,
}
impl Default for Runtime {
    fn default() -> Self {
        Self {
            active: Mutex::new(HashMap::new()),
            wake: Arc::new(Notify::new()),
            stopping: CancellationToken::new(),
        }
    }
}
fn db_error(e: rusqlite::Error) -> ApiError {
    storage_error(e.into())
}
fn decode(s: String) -> Result<Value, ApiError> {
    serde_json::from_str(&s).map_err(|e| storage_error(e.into()))
}
pub(crate) fn record(db: &Connection, table: &str, id: &str) -> Result<Value, ApiError> {
    let sql = match table {
        "sessions" => "SELECT record FROM sessions WHERE id=?1",
        "runs" => "SELECT record FROM runs WHERE id=?1",
        _ => unreachable!(),
    };
    let value: Option<String> = db
        .query_row(sql, [id], |r| r.get(0))
        .optional()
        .map_err(db_error)?;
    value.map(decode).transpose()?.ok_or_else(|| {
        ApiError::new(
            404,
            if table == "sessions" {
                "session_not_found"
            } else {
                "run_not_found"
            },
            "Resource not found on this device",
        )
    })
}
fn all_records(db: &Connection, table: &str) -> Result<Vec<Value>, ApiError> {
    let sql = match table {
        "sessions" => "SELECT record FROM sessions ORDER BY json_extract(record,'$.created_at'),id",
        "runs" => "SELECT record FROM runs ORDER BY rowid",
        "unfinished_runs" => {
            "SELECT record FROM runs WHERE json_extract(record,'$.status') IN ('queued','running','cancelling') ORDER BY rowid"
        }
        _ => unreachable!(),
    };
    let mut stmt = db.prepare(sql).map_err(db_error)?;
    stmt.query_map([], |r| r.get::<_, String>(0))
        .map_err(db_error)?
        .map(|r| decode(r.map_err(db_error)?))
        .collect()
}
pub fn terminal(run: &Value) -> bool {
    matches!(
        run["status"].as_str(),
        Some("succeeded" | "failed" | "cancelled" | "interrupted")
    )
}
fn save_run(db: &Connection, run: &Value) -> Result<(), ApiError> {
    AGENT.validate("Run", run)?;
    db.execute(
        "UPDATE runs SET record=?2 WHERE id=?1",
        params![run["run_id"].as_str().unwrap(), run.to_string()],
    )
    .map_err(db_error)?;
    Ok(())
}
pub(crate) fn event_tx(
    db: &Connection,
    run: &mut Value,
    kind: &str,
    data: Value,
) -> Result<Value, ApiError> {
    let id = run["run_id"].as_str().unwrap().to_owned();
    let seq = run["last_seq"].as_i64().unwrap() + 1;
    let event =
        json!({"run_id":id,"seq":seq,"created_at":rove_core::now(),"kind":kind,"data":data});
    AGENT.validate("RunEvent", &event)?;
    db.execute(
        "INSERT INTO run_events(run_id,seq,record) VALUES(?1,?2,?3)",
        params![id, seq, event.to_string()],
    )
    .map_err(db_error)?;
    if matches!(kind, "assistant_delta" | "tool_output") {
        let (mut tail, truncated): (String, i64) = db
            .query_row(
                "SELECT tail,truncated FROM run_output WHERE run_id=?1",
                [&id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(db_error)?;
        tail.push_str(data["text"].as_str().unwrap());
        let count = tail.chars().count();
        let clipped = count > 65536;
        if clipped {
            tail = tail.chars().skip(count - 65536).collect();
        }
        db.execute(
            "UPDATE run_output SET tail=?2,truncated=?3 WHERE run_id=?1",
            params![id, tail, i64::from(clipped || truncated != 0)],
        )
        .map_err(db_error)?;
    }
    db.execute(
        "DELETE FROM run_events WHERE run_id=?1 AND seq<=?2",
        params![id, seq - RETAIN_EVENTS],
    )
    .map_err(db_error)?;
    run["last_seq"] = json!(seq);
    save_run(db, run)?;
    Ok(event)
}
fn transition(db: &Connection, run: &mut Value, status: &str) -> Result<(), ApiError> {
    run["status"] = json!(status);
    if status == "running" {
        run["started_at"] = json!(rove_core::now());
    }
    if terminal(run) {
        run["finished_at"] = json!(rove_core::now());
    }
    event_tx(db, run, "status", json!({"status":status}))?;
    Ok(())
}
impl Agent {
    pub(crate) fn interrupt_old_runs(&self) -> Result<(), ApiError> {
        let mut db = self.store.connection.lock().unwrap();
        let tx = db.transaction().map_err(db_error)?;
        for mut run in all_records(&tx, "unfinished_runs")? {
            if !terminal(&run) {
                transition(&tx, &mut run, "interrupted")?;
            }
        }
        tx.commit().map_err(db_error)
    }
    pub(crate) fn start_scheduler(agent: &Arc<Self>) {
        let weak = Arc::downgrade(agent);
        let wake = agent.runtime.wake.clone();
        let stop = agent.runtime.stopping.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {_=stop.cancelled()=>break,_=wake.notified()=>{}}
                let Some(agent) = weak.upgrade() else { break };
                if stop.is_cancelled() {
                    break;
                }
                if agent.launch_ready().is_err() {
                    eprintln!("rove-agent: scheduler could not persist state");
                }
            }
        });
    }
    fn launch_ready(self: &Arc<Self>) -> Result<(), ApiError> {
        let max = self.store.settings().map_err(storage_error)?["max_active_runs"]
            .as_u64()
            .unwrap() as usize;
        let mut active = self.runtime.active.lock().unwrap();
        let mut db = self.store.connection.lock().unwrap();
        let tx = db.transaction().map_err(db_error)?;
        let runs = all_records(&tx, "unfinished_runs")?;
        let mut busy: HashSet<String> = runs
            .iter()
            .filter(|r| matches!(r["status"].as_str(), Some("running" | "cancelling")))
            .map(|r| r["session_id"].as_str().unwrap().to_string())
            .collect();
        let mut launch = Vec::new();
        for mut run in runs {
            if active.len() + launch.len() >= max {
                break;
            }
            if run["status"] != "queued" {
                continue;
            }
            let session = run["session_id"].as_str().unwrap().to_owned();
            if !busy.insert(session.clone()) {
                continue;
            }
            let id = run["run_id"].as_str().unwrap().to_owned();
            // Snapshot the actual device model at dequeue time, not submission time.
            let model = crate::models::selected(&tx, run["model_id"].as_str())?;
            if model.is_null() {
                run["error"] = json!(ApiError::new(
                    422,
                    "model_not_configured",
                    "Configure a model on the execution device"
                ));
                transition(&tx, &mut run, "failed")?;
                busy.remove(&session);
                continue;
            }
            run["model"] = json!({"provider":model["provider"],"base_url":model["base_url"],"model":model["model"]});
            transition(&tx, &mut run, "running")?;
            let input = run["input_message"].as_str().unwrap();
            save_message_tx(&tx, &run, "user", json!([{"kind":"text","text":input}]))?;
            let message = rig::completion::Message::user(input);
            tx.execute(
                "INSERT INTO rig_messages(session_id,run_id,message) VALUES(?1,?2,?3)",
                params![session, id, serde_json::to_string(&message).unwrap()],
            )
            .map_err(db_error)?;
            launch.push((id, run, model, CancellationToken::new()));
        }
        tx.commit().map_err(db_error)?;
        drop(db);
        for (id, run, model, cancel) in launch {
            active.insert(id.clone(), cancel.clone());
            let agent = self.clone();
            tokio::spawn(async move {
                let worker = tokio::spawn(crate::ai::execute(
                    agent.clone(),
                    run,
                    model,
                    cancel.clone(),
                ));
                let result = worker.await.unwrap_or_else(|_| {
                    Err(ApiError::new(
                        500,
                        "executor_failed",
                        "AI executor stopped unexpectedly",
                    ))
                });
                if agent
                    .finish_run(&id, result, cancel.is_cancelled())
                    .is_err()
                {
                    eprintln!("rove-agent: could not persist run completion");
                }
                agent.runtime.active.lock().unwrap().remove(&id);
                agent.runtime.wake.notify_one();
            });
        }
        Ok(())
    }
    fn finish_run(
        &self,
        id: &str,
        result: Result<(), ApiError>,
        cancelled: bool,
    ) -> Result<(), ApiError> {
        let mut db = self.store.connection.lock().unwrap();
        let tx = db.transaction().map_err(db_error)?;
        let mut run = record(&tx, "runs", id)?;
        if terminal(&run) {
            return Ok(());
        }
        let status = if self.runtime.stopping.is_cancelled() {
            "interrupted"
        } else if cancelled || run["status"] == "cancelling" {
            "cancelled"
        } else if result.is_ok() {
            "succeeded"
        } else {
            "failed"
        };
        if let Err(error) = result {
            run["error"] = json!(error);
            event_tx(&tx, &mut run, "error", json!(error))?;
        }
        transition(&tx, &mut run, status)?;
        tx.commit().map_err(db_error)
    }
    pub(crate) fn append_event(&self, id: &str, kind: &str, data: Value) -> Result<(), ApiError> {
        let mut db = self.store.connection.lock().unwrap();
        let tx = db.transaction().map_err(db_error)?;
        let mut run = record(&tx, "runs", id)?;
        event_tx(&tx, &mut run, kind, data)?;
        tx.commit().map_err(db_error)
    }
    pub(crate) fn save_message(
        &self,
        run: &Value,
        role: &str,
        parts: Value,
        rig_message: Option<&rig::completion::Message>,
    ) -> Result<(), ApiError> {
        let mut db = self.store.connection.lock().unwrap();
        let tx = db.transaction().map_err(db_error)?;
        save_message_tx(&tx, run, role, parts)?;
        if let Some(message) = rig_message {
            tx.execute(
                "INSERT INTO rig_messages(session_id,run_id,message) VALUES(?1,?2,?3)",
                params![
                    run["session_id"].as_str().unwrap(),
                    run["run_id"].as_str().unwrap(),
                    serde_json::to_string(message).unwrap()
                ],
            )
            .map_err(db_error)?;
        }
        tx.commit().map_err(db_error)
    }
    pub(crate) fn rig_history(
        &self,
        session: &str,
    ) -> Result<Vec<rig::completion::Message>, ApiError> {
        let db = self.store.connection.lock().unwrap();
        let bytes:i64=db.query_row("SELECT COALESCE(sum(length(CAST(message AS BLOB))),0) FROM rig_messages WHERE session_id=?1",[session],|r|r.get(0)).map_err(db_error)?;
        if bytes > 8 * 1024 * 1024 {
            return Err(ApiError::new(
                413,
                "history_too_large",
                "Model context exceeds 8 MiB; start a new session (saved history is retained)",
            ));
        }
        let mut stmt = db
            .prepare("SELECT message FROM rig_messages WHERE session_id=?1 ORDER BY seq")
            .map_err(db_error)?;
        let messages = stmt
            .query_map([session], |r| r.get::<_, String>(0))
            .map_err(db_error)?
            .map(|r| {
                serde_json::from_str(&r.map_err(db_error)?).map_err(|e| storage_error(e.into()))
            })
            .collect::<Result<Vec<rig::completion::Message>, ApiError>>()?;
        // Cancellation may leave a model's later tools unexecuted. Preserve the
        // audit messages, but don't replay unmatched calls to the next request.
        let answered: HashSet<_> = messages
            .iter()
            .flat_map(|m| match m {
                rig::completion::Message::User { content } => content
                    .iter()
                    .filter_map(|p| match p {
                        rig::message::UserContent::ToolResult(r) => Some(r.call.clone()),
                        _ => None,
                    })
                    .collect::<Vec<_>>(),
                _ => vec![],
            })
            .collect();
        Ok(messages
            .into_iter()
            .filter_map(|mut m| {
                if let rig::completion::Message::Assistant { content, .. } = &mut m {
                    content.retain(|p| match p {
                        rig::message::AssistantContent::ToolCall(call) => {
                            answered.contains(&call.id)
                        }
                        _ => true,
                    });
                    if content.is_empty() {
                        return None;
                    }
                }
                Some(m)
            })
            .collect())
    }
    pub(crate) fn run_snapshot(&self, id: &str) -> Result<Value, ApiError> {
        let db = self.store.connection.lock().unwrap();
        let run = record(&db, "runs", id)?;
        let (tail, truncated): (String, i64) = db
            .query_row(
                "SELECT tail,truncated FROM run_output WHERE run_id=?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(db_error)?;
        let retained: i64 = db
            .query_row(
                "SELECT COALESCE(min(seq),1) FROM run_events WHERE run_id=?1",
                [id],
                |r| r.get(0),
            )
            .map_err(db_error)?;
        Ok(
            json!({"snapshot_seq":run["last_seq"],"run":run,"output_tail":tail,"output_truncated":truncated!=0,"retained_from_seq":retained}),
        )
    }
    pub(crate) fn events_after(
        &self,
        id: &str,
        after: i64,
    ) -> Result<(Vec<Value>, bool), ApiError> {
        let db = self.store.connection.lock().unwrap();
        let run = record(&db, "runs", id)?;
        if after > run["last_seq"].as_i64().unwrap() {
            return Err(ApiError::new(
                409,
                "event_cursor_ahead",
                "Event cursor is ahead of the run",
            ));
        }
        let retained: i64 = db
            .query_row(
                "SELECT COALESCE(min(seq),1) FROM run_events WHERE run_id=?1",
                [id],
                |r| r.get(0),
            )
            .map_err(db_error)?;
        if after < retained - 1 {
            return Err(ApiError::new(
                410,
                "event_history_expired",
                "Fetch a run snapshot and resume from snapshot_seq",
            ));
        }
        let mut stmt = db
            .prepare(
                "SELECT record FROM run_events WHERE run_id=?1 AND seq>?2 ORDER BY seq LIMIT 100",
            )
            .map_err(db_error)?;
        let events = stmt
            .query_map(params![id, after], |r| r.get::<_, String>(0))
            .map_err(db_error)?
            .map(|r| decode(r.map_err(db_error)?))
            .collect::<Result<Vec<_>, _>>()?;
        let caught_up = events.last().map_or(after, |e| e["seq"].as_i64().unwrap())
            == run["last_seq"].as_i64().unwrap();
        Ok((events, terminal(&run) && caught_up))
    }
    pub(crate) fn session_operation(
        &self,
        request: &Request,
    ) -> Result<(u16, Option<Value>), ApiError> {
        let body = request.body.as_ref().unwrap_or(&Value::Null);
        let id = request
            .path_parameters
            .get("session_id")
            .or_else(|| request.path_parameters.get("run_id"))
            .map(String::as_str)
            .unwrap_or("");
        if request.operation_id == "get_run" {
            return Ok((200, Some(self.run_snapshot(id)?)));
        }
        let mut db = self.store.connection.lock().unwrap();
        let tx = db.transaction().map_err(db_error)?;
        let (status, value) = match request.operation_id.as_str() {
            "create_session" => {
                let id = body["session_id"]
                    .as_str()
                    .map(str::to_owned)
                    .unwrap_or_else(|| Uuid::new_v4().to_string());
                if let Some(execution) = body.get("execution_target")
                    && execution["device_id"] == json!(self.store.device_id)
                {
                    return Err(ApiError::invalid("Use a local session for this device"));
                }
                if let Some(original) = tx
                    .query_row(
                        "SELECT request FROM session_creations WHERE id=?1",
                        [&id],
                        |r| r.get::<_, String>(0),
                    )
                    .optional()
                    .map_err(db_error)?
                {
                    if original == "null" {
                        return Err(ApiError::new(
                            409,
                            "session_deleted",
                            "This session was deleted and cannot be recreated",
                        ));
                    }
                    if decode(original)? != *body {
                        return Err(ApiError::new(
                            409,
                            "session_conflict",
                            "Session ID already used with different creation input",
                        ));
                    }
                    return match record(&tx, "sessions", &id) {
                        Ok(session) => Ok((200, Some(session))),
                        Err(e) if e.status == 404 => Err(ApiError::new(
                            409,
                            "session_deleted",
                            "This session was deleted and cannot be recreated",
                        )),
                        Err(e) => Err(e),
                    };
                }
                if tx
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM sessions WHERE id=?1)",
                        [&id],
                        |r| r.get::<_, bool>(0),
                    )
                    .map_err(db_error)?
                {
                    return Err(ApiError::new(
                        409,
                        "session_conflict",
                        "Session ID is already in use",
                    ));
                }
                let now = rove_core::now();
                let mut session = json!({"session_id":id,"device_id":self.store.device_id,"title":body.get("title").cloned().unwrap_or(json!("New session")),"created_at":now,"updated_at":now,"archived_at":null});
                session["title_source"] = json!(if body["auto_title"]
                    .as_bool()
                    .unwrap_or(body.get("title").is_none())
                {
                    "default"
                } else {
                    "manual"
                });
                if let Some(execution) = body.get("execution_target") {
                    session["execution_target"] = execution.clone();
                }
                tx.execute(
                    "INSERT INTO session_creations(id,request) VALUES(?1,?2)",
                    params![id, body.to_string()],
                )
                .map_err(db_error)?;
                tx.execute(
                    "INSERT INTO sessions(id,record) VALUES(?1,?2)",
                    params![id, session.to_string()],
                )
                .map_err(db_error)?;
                (201, session)
            }
            "get_session" => (200, record(&tx, "sessions", id)?),
            "update_session" => {
                let mut session = record(&tx, "sessions", id)?;
                require_unarchived(&session)?;
                if let Some(title) = body.get("title") {
                    let title = title.as_str().unwrap().trim();
                    if title.is_empty() {
                        return Err(ApiError::invalid("Session title cannot be blank"));
                    }
                    session["title"] = json!(title);
                    session["title_source"] = json!("manual");
                }
                if let Some(model_id) = body.get("model_id") {
                    if session["execution_target"].is_null()
                        && !model_id.is_null()
                        && crate::models::selected(&tx, model_id.as_str())?.is_null()
                    {
                        return Err(ApiError::new(
                            404,
                            "model_not_found",
                            "Model configuration not found",
                        ));
                    }
                    session["model_id"] = model_id.clone();
                }
                session["updated_at"] = json!(rove_core::now());
                tx.execute(
                    "UPDATE sessions SET record=?2 WHERE id=?1",
                    params![id, session.to_string()],
                )
                .map_err(db_error)?;
                (200, session)
            }
            "list_sessions" => {
                let mut sessions = all_records(&tx, "sessions")?;
                if request
                    .query_parameters
                    .get("order")
                    .and_then(Value::as_str)
                    == Some("desc")
                {
                    sessions.reverse();
                }
                if let Some(archived) = request
                    .query_parameters
                    .get("archived")
                    .and_then(Value::as_bool)
                {
                    sessions.retain(|s| s["archived_at"].is_null() != archived);
                }
                (200, crate::page(request, sessions, "session_id")?)
            }
            "archive_session" | "restore_session" => {
                let mut session = record(&tx, "sessions", id)?;
                if request.operation_id == "archive_session" {
                    if session["archived_at"].is_null() {
                        session["archived_at"] = json!(rove_core::now());
                    }
                } else {
                    session["archived_at"] = Value::Null;
                }
                tx.execute(
                    "UPDATE sessions SET record=?2 WHERE id=?1",
                    params![id, session.to_string()],
                )
                .map_err(db_error)?;
                (204, Value::Null)
            }
            "delete_session" => {
                let session = record(&tx, "sessions", id)?;
                if session["archived_at"].is_null() {
                    return Err(ApiError::new(
                        409,
                        "session_not_archived",
                        "Archive the session before deleting it",
                    ));
                }
                let active: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM runs WHERE session_id=?1 AND json_extract(record,'$.status') IN ('queued','running','cancelling'))",[id],|r|r.get(0)).map_err(db_error)?;
                if active {
                    return Err(ApiError::new(
                        409,
                        "session_has_active_runs",
                        "Wait for active jobs or restore the session and cancel them",
                    ));
                }
                let remote_active:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM remote_submissions WHERE session_id=?1 AND run_id IS NULL) OR EXISTS(SELECT 1 FROM remote_runs WHERE session_id=?1 AND json_extract(record,'$.status') NOT IN ('succeeded','failed','cancelled','interrupted'))",[id],|r|r.get(0)).map_err(db_error)?;
                if remote_active {
                    return Err(ApiError::new(
                        409,
                        "session_has_active_runs",
                        "Resolve pending submissions and refresh remote completion before deletion",
                    ));
                }
                tx.execute("INSERT OR IGNORE INTO deleted_requests(request_id) SELECT request_id FROM remote_submissions WHERE session_id=?1",[id]).map_err(db_error)?;
                tx.execute("INSERT OR IGNORE INTO deleted_requests(request_id) SELECT json_extract(record,'$.request_id') FROM remote_runs WHERE session_id=?1",[id]).map_err(db_error)?;
                tx.execute("DELETE FROM remote_submissions WHERE session_id=?1", [id])
                    .map_err(db_error)?;
                tx.execute("DELETE FROM remote_runs WHERE session_id=?1", [id])
                    .map_err(db_error)?;
                // Keep only a deletion marker, not the original title/target.
                tx.execute(
                    "UPDATE session_creations SET request='null' WHERE id=?1",
                    [id],
                )
                .map_err(db_error)?;
                tx.execute("INSERT OR IGNORE INTO deleted_requests(request_id) SELECT request_id FROM runs WHERE session_id=?1",[id]).map_err(db_error)?;
                for sql in [
                    "DELETE FROM run_events WHERE run_id IN (SELECT id FROM runs WHERE session_id=?1)",
                    "DELETE FROM run_output WHERE run_id IN (SELECT id FROM runs WHERE session_id=?1)",
                    "DELETE FROM rig_messages WHERE session_id=?1",
                    "DELETE FROM messages WHERE session_id=?1",
                    "DELETE FROM runs WHERE session_id=?1",
                    "DELETE FROM sessions WHERE id=?1",
                ] {
                    tx.execute(sql, [id]).map_err(db_error)?;
                }
                (204, Value::Null)
            }
            "list_runs" => {
                let mut runs = all_records(&tx, "runs")?;
                let mut stmt = tx
                    .prepare("SELECT record FROM remote_runs")
                    .map_err(db_error)?;
                for value in stmt
                    .query_map([], |r| r.get::<_, String>(0))
                    .map_err(db_error)?
                {
                    runs.push(decode(value.map_err(db_error)?)?);
                }
                runs.retain(|r| {
                    request
                        .query_parameters
                        .iter()
                        .filter(|(k, _)| matches!(k.as_str(), "session_id" | "status"))
                        .all(|(k, v)| r[k] == *v)
                });
                runs.sort_by_key(|r| {
                    (
                        r["created_at"].as_str().unwrap().to_string(),
                        r["run_id"].as_str().unwrap().to_string(),
                    )
                });
                (200, crate::page(request, runs, "run_id")?)
            }
            "list_messages" => {
                record(&tx, "sessions", id)?;
                let mut stmt = tx
                    .prepare("SELECT seq,record FROM messages WHERE session_id=?1 ORDER BY seq")
                    .map_err(db_error)?;
                let items: Result<Vec<_>, _> = stmt
                    .query_map([id], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))
                    .map_err(db_error)?
                    .map(|r| {
                        let (seq, s) = r.map_err(db_error)?;
                        let mut value = decode(s)?;
                        value["_seq"] = json!(format!("{seq:020}"));
                        Ok(value)
                    })
                    .collect();
                let mut page = crate::page(request, items?, "_seq")?;
                for item in page["items"].as_array_mut().unwrap() {
                    item.as_object_mut().unwrap().remove("_seq");
                }
                (200, page)
            }
            "submit_run" => {
                if self.runtime.stopping.is_cancelled() {
                    return Err(ApiError::new(
                        503,
                        "agent_stopping",
                        "Agent is stopping; retry after restart",
                    ));
                }
                let mut canonical = json!({"session_id":id,"network_id":body.get("network_id").cloned().unwrap_or(Value::Null),"message":body["message"]});
                // Preserve pre-catalog idempotency digests for requests without a selector.
                if let Some(model_id) = body.get("model_id") {
                    canonical["model_id"] = model_id.clone();
                }
                let deleted: bool = tx
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM deleted_requests WHERE request_id=?1)",
                        [body["request_id"].as_str().unwrap()],
                        |r| r.get(0),
                    )
                    .map_err(db_error)?;
                if deleted {
                    return Err(ApiError::new(
                        409,
                        "request_deleted",
                        "This request belongs to deleted history and will not execute again",
                    ));
                }
                if tx
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM remote_submissions WHERE request_id=?1) OR EXISTS(SELECT 1 FROM remote_runs WHERE json_extract(record,'$.request_id')=?1)",
                        [body["request_id"].as_str()],
                        |r| r.get::<_, bool>(0),
                    )
                    .map_err(db_error)?
                {
                    return Err(ApiError::new(
                        409,
                        "request_conflict",
                        "request_id belongs to a linked remote submission",
                    ));
                }
                let previous: Option<(String, String)> = tx
                    .query_row(
                        "SELECT record,request FROM runs WHERE request_id=?1",
                        [body["request_id"].as_str().unwrap()],
                        |r| Ok((r.get(0)?, r.get(1)?)),
                    )
                    .optional()
                    .map_err(db_error)?;
                if let Some((run, original)) = previous {
                    if decode(original)? != canonical {
                        return Err(ApiError::new(
                            409,
                            "request_conflict",
                            "request_id was already used with different input",
                        ));
                    }
                    return Ok((200, Some(decode(run)?)));
                }
                let session = record(&tx, "sessions", id)?;
                require_unarchived(&session)?;
                let selected_model = body.get("model_id").unwrap_or(&session["model_id"]);
                if crate::models::selected(&tx, selected_model.as_str())?.is_null() {
                    return Err(ApiError::new(
                        422,
                        "model_not_configured",
                        "Configure a model on the execution device",
                    ));
                }
                let queued: i64 = tx
                    .query_row(
                        "SELECT count(*) FROM runs WHERE json_extract(record,'$.status')='queued'",
                        [],
                        |row| row.get(0),
                    )
                    .map_err(db_error)?;
                if queued >= MAX_QUEUED_RUNS as i64 {
                    return Err(ApiError::new(
                        429,
                        "queue_full",
                        "Pending run queue is full",
                    ));
                }
                let run_id = Uuid::new_v4().to_string();
                let mut run = json!({"run_id":run_id,"request_id":body["request_id"],"session_id":id,"device_id":self.store.device_id,"status":"queued","created_at":rove_core::now(),"started_at":null,"finished_at":null,"model":null,"last_seq":0,"error":null,"network_id":canonical["network_id"],"input_message":body["message"]});
                run["model_id"] = selected_model.clone();
                tx.execute("INSERT INTO runs(id,request_id,session_id,record,request) VALUES(?1,?2,?3,?4,?5)",params![run_id,body["request_id"].as_str().unwrap(),id,run.to_string(),canonical.to_string()]).map_err(db_error)?;
                tx.execute(
                    "INSERT INTO run_output(run_id,tail,truncated) VALUES(?1,'',0)",
                    [run_id],
                )
                .map_err(db_error)?;
                event_tx(&tx, &mut run, "status", json!({"status":"queued"}))?;
                title_from_message(&tx, id, body["message"].as_str().unwrap())?;
                (202, run)
            }
            "cancel_run" => {
                let mut run = record(&tx, "runs", id)?;
                let status = if run["status"] == "queued" {
                    transition(&tx, &mut run, "cancelled")?;
                    200
                } else if !terminal(&run) {
                    if run["status"] != "cancelling" {
                        transition(&tx, &mut run, "cancelling")?;
                    }
                    202
                } else {
                    200
                };
                (status, run)
            }
            _ => {
                return Err(ApiError::new(
                    501,
                    "unsupported",
                    "Session operation not implemented",
                ));
            }
        };
        tx.commit().map_err(db_error)?;
        drop(db);
        if request.operation_id == "cancel_run"
            && let Some(cancel) = self.runtime.active.lock().unwrap().get(id)
        {
            cancel.cancel();
        }
        self.runtime.wake.notify_one();
        Ok((status, if status == 204 { None } else { Some(value) }))
    }
}
pub(crate) fn require_unarchived(session: &Value) -> Result<(), ApiError> {
    if !session["archived_at"].is_null() {
        return Err(ApiError::new(
            409,
            "session_archived",
            "Restore the session before modifying it",
        ));
    }
    Ok(())
}
/// Runs inside the acceptance/cache transaction and re-reads current metadata,
/// so a concurrent explicit rename always wins. Legacy titles stay untouched.
pub(crate) fn title_from_message(db: &Connection, id: &str, message: &str) -> Result<(), ApiError> {
    let mut session = record(db, "sessions", id)?;
    if session["title_source"] != "default"
        || !session["archived_at"].is_null()
        || session["kind"] == "network_onboarding"
    {
        return Ok(());
    }
    let clean = message.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut chars = clean.chars().filter(|c| !c.is_control());
    let mut title: String = chars.by_ref().take(32).collect();
    if title.is_empty() {
        return Ok(());
    }
    if chars.next().is_some() {
        title.push('…');
    }
    session["title"] = json!(title);
    session["title_source"] = json!("message");
    session["updated_at"] = json!(rove_core::now());
    db.execute(
        "UPDATE sessions SET record=?2 WHERE id=?1",
        params![id, session.to_string()],
    )
    .map_err(db_error)?;
    Ok(())
}
fn save_message_tx(db: &Connection, run: &Value, role: &str, parts: Value) -> Result<(), ApiError> {
    let message = json!({"message_id":Uuid::new_v4(),"session_id":run["session_id"],"run_id":run["run_id"],"role":role,"parts":parts,"created_at":rove_core::now()});
    AGENT.validate("Message", &message)?;
    db.execute(
        "INSERT INTO messages(session_id,run_id,record) VALUES(?1,?2,?3)",
        params![
            run["session_id"].as_str().unwrap(),
            run["run_id"].as_str().unwrap(),
            message.to_string()
        ],
    )
    .map_err(db_error)?;
    db.execute(
        "UPDATE sessions SET record=json_set(record,'$.updated_at',?2) WHERE id=?1",
        params![run["session_id"].as_str().unwrap(), rove_core::now()],
    )
    .map_err(db_error)?;
    Ok(())
}
