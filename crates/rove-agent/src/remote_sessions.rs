//! Origin-owned conversation links. Only explicit submit/retry performs writes
//! on the execution peer. Reads/restarts never replay an uncertain submission.
use crate::{Agent, sessions, storage_error};
use rove_protocol::{ApiError, Request, SocketTarget, contract::AGENT};
use rusqlite::{OptionalExtension, params};
use serde_json::{Value, json};
use std::{sync::Arc, time::Duration};

fn db_error(error: rusqlite::Error) -> ApiError {
    storage_error(error.into())
}
fn decode(value: String) -> Result<Value, ApiError> {
    serde_json::from_str(&value).map_err(|e| storage_error(e.into()))
}
fn target(session: &Value) -> Result<Option<SocketTarget>, ApiError> {
    session
        .get("execution_target")
        .filter(|v| !v.is_null())
        .cloned()
        .map(serde_json::from_value)
        .transpose()
        .map_err(|e| storage_error(e.into()))
}
fn unavailable() -> ApiError {
    ApiError::new(
        503,
        "target_unreachable",
        "Remote outcome is unknown; no local execution or automatic resubmission occurred",
    )
}
impl Agent {
    pub(crate) async fn linked_client(
        &self,
        target: &SocketTarget,
    ) -> Result<rove_sdk::RemoteClient, ApiError> {
        #[cfg(test)]
        if let Some(url) = self
            .remote_test_routes
            .lock()
            .unwrap()
            .get(&target.device_id)
            .cloned()
        {
            return rove_sdk::RemoteClient::new(url, target.network_id, target.device_id)
                .map_err(|_| unavailable());
        }
        #[cfg(all(feature = "easytier", target_os = "linux"))]
        {
            self.target_client(target).await
        }
        #[cfg(not(all(feature = "easytier", target_os = "linux")))]
        {
            let _ = target;
            Err(unavailable())
        }
    }
    async fn linked_call(
        &self,
        target: &SocketTarget,
        mut request: Request,
    ) -> Result<(u16, Value), ApiError> {
        request.target = None;
        #[cfg(test)]
        let is_submit = request.operation_id == "submit_run";
        let response = self
            .linked_client(target)
            .await?
            .call(request)
            .await
            .map_err(|_| unavailable())?;
        if response.status_code >= 400 {
            let mut error: ApiError = response
                .body
                .and_then(|v| serde_json::from_value(v["error"].clone()).ok())
                .unwrap_or_else(unavailable);
            error.status = response.status_code;
            return Err(error);
        }
        #[cfg(test)]
        if is_submit
            && self
                .remote_drop_submit_response
                .swap(false, std::sync::atomic::Ordering::SeqCst)
        {
            return Err(unavailable());
        }
        Ok((response.status_code, response.body.unwrap_or(Value::Null)))
    }
    fn linked_session(&self, id: &str) -> Result<Option<Value>, ApiError> {
        if id.is_empty() {
            return Ok(None);
        }
        let db = self.store.connection.lock().unwrap();
        let session = match sessions::record(&db, "sessions", id) {
            Ok(v) => v,
            Err(e) if e.status == 404 => return Ok(None),
            Err(e) => return Err(e),
        };
        Ok(target(&session)?.map(|_| session))
    }
    fn run_session(&self, id: &str) -> Result<Option<Value>, ApiError> {
        let session: Option<String> = self
            .store
            .connection
            .lock()
            .unwrap()
            .query_row(
                "SELECT session_id FROM remote_runs WHERE id=?1",
                [id],
                |r| r.get(0),
            )
            .optional()
            .map_err(db_error)?;
        session
            .map(|id| self.linked_session(&id))
            .transpose()
            .map(Option::flatten)
    }
    fn cache_run(
        &self,
        session: &Value,
        run: &Value,
        snapshot: Option<&Value>,
    ) -> Result<(), ApiError> {
        AGENT.validate("Run", run)?;
        let execution = target(session)?.unwrap();
        if run["session_id"] != session["session_id"]
            || run["device_id"] != json!(execution.device_id)
        {
            return Err(ApiError::new(
                409,
                "target_mismatch",
                "Run does not belong to the linked execution device/session",
            ));
        }
        if let Some(snapshot) = snapshot {
            AGENT.validate("RunSnapshot", snapshot)?;
            if snapshot["snapshot_seq"] != run["last_seq"] {
                return Err(ApiError::new(
                    409,
                    "event_sequence_gap",
                    "Snapshot watermark does not match its run",
                ));
            }
        }
        let mut db = self.store.connection.lock().unwrap();
        let tx = db.transaction().map_err(db_error)?;
        sessions::record(&tx, "sessions", session["session_id"].as_str().unwrap())?;
        let previous: Option<String> = tx
            .query_row(
                "SELECT record FROM remote_runs WHERE id=?1",
                [run["run_id"].as_str()],
                |r| r.get(0),
            )
            .optional()
            .map_err(db_error)?;
        if let Some(previous) = previous {
            let previous = decode(previous)?;
            if [
                "request_id",
                "session_id",
                "device_id",
                "created_at",
                "input_message",
            ]
            .iter()
            .any(|key| previous[*key] != run[*key])
            {
                return Err(ApiError::new(
                    409,
                    "target_mismatch",
                    "Remote run identity changed",
                ));
            }
        }
        let pending: Option<(String, String)> = tx
            .query_row(
                "SELECT session_id,wire FROM remote_submissions WHERE request_id=?1",
                [run["request_id"].as_str()],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .map_err(db_error)?;
        if let Some((owner, wire)) = pending {
            let wire = decode(wire)?;
            if owner != session["session_id"].as_str().unwrap()
                || wire["message"] != run["input_message"]
                || wire.get("model_id").is_some_and(|m| *m != run["model_id"])
            {
                return Err(ApiError::new(
                    409,
                    "request_conflict",
                    "Remote run differs from the persisted submission",
                ));
            }
        }
        let collision: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM runs WHERE id=?1 OR request_id=?2)",
                params![run["run_id"].as_str(), run["request_id"].as_str()],
                |r| r.get(0),
            )
            .map_err(db_error)?;
        if collision {
            return Err(ApiError::new(
                409,
                "request_conflict",
                "Remote run collides with local execution history",
            ));
        }
        tx.execute("INSERT INTO remote_runs(id,session_id,record,snapshot,observed_seq) VALUES(?1,?2,?3,?4,?5)
            ON CONFLICT(id) DO UPDATE SET record=CASE WHEN json_extract(excluded.record,'$.last_seq')>=json_extract(remote_runs.record,'$.last_seq') THEN excluded.record ELSE remote_runs.record END,
            snapshot=CASE WHEN excluded.snapshot IS NOT NULL AND (remote_runs.snapshot IS NULL OR json_extract(excluded.snapshot,'$.snapshot_seq')>=json_extract(remote_runs.snapshot,'$.snapshot_seq')) THEN excluded.snapshot ELSE remote_runs.snapshot END,
            observed_seq=max(remote_runs.observed_seq,excluded.observed_seq)",
            params![run["run_id"].as_str(), session["session_id"].as_str(), run.to_string(), snapshot.map(Value::to_string), run["last_seq"].as_i64()]).map_err(db_error)?;
        // A read can reconcile acceptance even when the original response was lost.
        tx.execute("UPDATE remote_submissions SET run_id=?1,error=NULL WHERE request_id=?2 AND session_id=?3",
            params![run["run_id"].as_str(),run["request_id"].as_str(),session["session_id"].as_str()]).map_err(db_error)?;
        sessions::title_from_message(
            &tx,
            session["session_id"].as_str().unwrap(),
            run["input_message"].as_str().unwrap(),
        )?;
        tx.commit().map_err(db_error)
    }
    fn pending_submissions(&self, request: &Request) -> Result<Value, ApiError> {
        let id = &request.path_parameters["session_id"];
        let db = self.store.connection.lock().unwrap();
        sessions::record(&db, "sessions", id)?;
        let mut stmt = db.prepare("SELECT wire,created_at,error FROM remote_submissions WHERE session_id=?1 AND run_id IS NULL ORDER BY request_id").map_err(db_error)?;
        let values = stmt.query_map([id], |r| Ok((r.get::<_,String>(0)?, r.get::<_,String>(1)?, r.get::<_,Option<String>>(2)?)))
            .map_err(db_error)?.map(|r| {
                let (wire,created,error)=r.map_err(db_error)?;
                Ok(json!({"session_id":id,"request":decode(wire)?,"created_at":created,"error":error.map(decode).transpose()?}))
            }).collect::<Result<Vec<Value>,ApiError>>()?;
        let mut values = values;
        for value in &mut values {
            value["_id"] = value["request"]["request_id"].clone();
        }
        let mut page = crate::page(request, values, "_id")?;
        for value in page["items"].as_array_mut().unwrap() {
            value.as_object_mut().unwrap().remove("_id");
        }
        Ok(page)
    }
    async fn submit_linked(
        &self,
        session: &Value,
        body: &Value,
    ) -> Result<(u16, Option<Value>), ApiError> {
        let id = session["session_id"].as_str().unwrap();
        let execution = target(session)?.unwrap();
        let wire = {
            let mut db = self.store.connection.lock().unwrap();
            let tx = db.transaction().map_err(db_error)?;
            let current = sessions::record(&tx, "sessions", id)?;
            let request_id = body["request_id"].as_str().unwrap();
            let deleted: bool = tx
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM deleted_requests WHERE request_id=?1)",
                    [request_id],
                    |r| r.get(0),
                )
                .map_err(db_error)?;
            if deleted {
                return Err(ApiError::new(
                    409,
                    "request_deleted",
                    "This request belongs to deleted history",
                ));
            }
            let previous:Option<(String,String,Option<String>,String)>=tx.query_row("SELECT request,wire,run_id,session_id FROM remote_submissions WHERE request_id=?1",[request_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional().map_err(db_error)?;
            if let Some((original, wire, run_id, owner)) = previous {
                let wire = decode(wire)?;
                if owner != id || (decode(original)? != *body && wire != *body) {
                    return Err(ApiError::new(
                        409,
                        "request_conflict",
                        "request_id already used with different input",
                    ));
                }
                if let Some(run_id) = run_id {
                    let record: String = tx
                        .query_row(
                            "SELECT record FROM remote_runs WHERE id=?1",
                            [run_id],
                            |r| r.get(0),
                        )
                        .map_err(db_error)?;
                    return Ok((200, Some(decode(record)?)));
                }
                // Archive may not turn an uncertain request into a new side effect.
                sessions::require_unarchived(&current)?;
                wire
            } else {
                sessions::require_unarchived(&current)?;
                let used: bool = tx
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM runs WHERE request_id=?1) OR EXISTS(SELECT 1 FROM remote_runs WHERE json_extract(record,'$.request_id')=?1)",
                        [request_id],
                        |r| r.get(0),
                    )
                    .map_err(db_error)?;
                if used {
                    return Err(ApiError::new(
                        409,
                        "request_conflict",
                        "request_id belongs to a local run",
                    ));
                }
                let pending: i64 = tx
                    .query_row(
                        "SELECT count(*) FROM remote_submissions WHERE run_id IS NULL",
                        [],
                        |r| r.get(0),
                    )
                    .map_err(db_error)?;
                if pending >= 1024 {
                    return Err(ApiError::new(
                        429,
                        "queue_full",
                        "Too many unconfirmed remote submissions",
                    ));
                }
                let mut wire = body.clone();
                if wire.get("network_id").is_none() {
                    wire["network_id"] = json!(execution.network_id);
                }
                if wire.get("model_id").is_none() && current["model_id"].is_string() {
                    wire["model_id"] = current["model_id"].clone();
                }
                tx.execute("INSERT INTO remote_submissions(request_id,session_id,request,wire,created_at) VALUES(?1,?2,?3,?4,?5)",params![request_id,id,body.to_string(),wire.to_string(),rove_core::now()]).map_err(db_error)?;
                tx.commit().map_err(db_error)?;
                wire
            }
        };
        let outcome=async {
            // Stable creation input is saved separately so later local renames do
            // not conflict with retries after a lost create response.
            let original:String=self.store.connection.lock().unwrap().query_row("SELECT request FROM session_creations WHERE id=?1",[id],|r|r.get(0)).map_err(db_error)?;
            let original=decode(original)?;
            let created=self.linked_call(&execution,Request::new("create_session").with_body(json!({"session_id":id,"title":original.get("title").cloned().unwrap_or(json!("New session"))}))).await?.1;
            if created["device_id"]!=json!(execution.device_id) || created["session_id"]!=id || !created["execution_target"].is_null(){return Err(ApiError::new(409,"target_mismatch","Execution session must be local to the selected peer"));}
            let (status,run)=self.linked_call(&execution,Request::new("submit_run").with_path("session_id",id).with_body(wire.clone())).await?;
            if run["request_id"]!=wire["request_id"] || run["input_message"]!=wire["message"] {return Err(ApiError::new(409,"request_conflict","Execution peer returned a different request"));}
            self.cache_run(session,&run,None)?;
            Ok((status,Some(run)))
        }.await;
        if let Err(error) = &outcome {
            self.store
                .connection
                .lock()
                .unwrap()
                .execute(
                    "UPDATE remote_submissions SET error=?2 WHERE request_id=?1 AND run_id IS NULL",
                    params![body["request_id"].as_str(), json!(error).to_string()],
                )
                .map_err(db_error)?;
        }
        outcome
    }
    pub(crate) async fn remote_session_operation(
        self: &Arc<Self>,
        request: &Request,
    ) -> Result<Option<(u16, Option<Value>)>, ApiError> {
        let op = request.operation_id.as_str();
        if op == "list_session_submissions" {
            return Ok(Some((200, Some(self.pending_submissions(request)?))));
        }
        let session_id = request
            .path_parameters
            .get("session_id")
            .map(String::as_str)
            .or_else(|| {
                request
                    .query_parameters
                    .get("session_id")
                    .and_then(Value::as_str)
            })
            .unwrap_or("");
        let run_id = request
            .path_parameters
            .get("run_id")
            .map(String::as_str)
            .unwrap_or("");
        let session = if !run_id.is_empty() {
            self.run_session(run_id)?
        } else {
            self.linked_session(session_id)?
        };
        let Some(session) = session else {
            return Ok(None);
        };
        let execution = target(&session)?.unwrap();
        let value = match op {
            "submit_run" => {
                return self
                    .submit_linked(&session, request.body.as_ref().unwrap())
                    .await
                    .map(Some);
            }
            "get_run" => match self.linked_call(&execution, request.clone()).await {
                Ok((_, snapshot)) => {
                    if snapshot["run"]["run_id"] != run_id || !snapshot["sync_error"].is_null() {
                        return Err(ApiError::new(
                            409,
                            "target_mismatch",
                            "Expected an authoritative snapshot for the linked run",
                        ));
                    }
                    self.cache_run(&session, &snapshot["run"], Some(&snapshot))?;
                    snapshot
                }
                Err(error) => {
                    let cached: Option<String> = self
                        .store
                        .connection
                        .lock()
                        .unwrap()
                        .query_row(
                            "SELECT snapshot FROM remote_runs WHERE id=?1",
                            [run_id],
                            |r| r.get(0),
                        )
                        .map_err(db_error)?;
                    let Some(cached) = cached else {
                        return Err(error);
                    };
                    let mut snapshot = decode(cached)?;
                    snapshot["sync_error"] = json!(error);
                    snapshot
                }
            },
            "cancel_run" => {
                let (status, run) = self.linked_call(&execution, request.clone()).await?;
                if run["run_id"] != run_id {
                    return Err(ApiError::new(
                        409,
                        "target_mismatch",
                        "Cancel response belongs to a different run",
                    ));
                }
                self.cache_run(&session, &run, None)?;
                return Ok(Some((status, Some(run))));
            }
            "list_runs" => {
                let page = match self.linked_call(&execution, request.clone()).await {
                    Ok((_, page)) => page,
                    Err(error) => {
                        let (_, page) = self.session_operation(request)?;
                        let mut page = page.unwrap();
                        page["sync_error"] = json!(error);
                        return Ok(Some((200, Some(page))));
                    }
                };
                for run in page["items"].as_array().unwrap() {
                    self.cache_run(&session, run, None)?;
                }
                page
            }
            "list_messages" => self.linked_call(&execution, request.clone()).await?.1,
            // Selection belongs to the origin, but references the execution
            // device's catalog. No credentials are copied or silently borrowed.
            "update_session"
                if request
                    .body
                    .as_ref()
                    .is_some_and(|b| b.get("model_id").is_some_and(|id| !id.is_null())) =>
            {
                let catalog = self
                    .linked_call(&execution, Request::new("get_model_catalog"))
                    .await?
                    .1;
                if !catalog["models"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|m| m["model_id"] == request.body.as_ref().unwrap()["model_id"])
                {
                    return Err(ApiError::new(
                        404,
                        "model_not_found",
                        "Model configuration not found on execution device",
                    ));
                }
                return Ok(None);
            }
            _ => return Ok(None),
        };
        Ok(Some((200, Some(value))))
    }
    /// Shared by socket and HTTP event transports, always through rove-sdk.
    pub(crate) async fn read_events(
        &self,
        id: &str,
        after: i64,
    ) -> Result<(Vec<Value>, bool), ApiError> {
        let Some(session) = self.run_session(id)? else {
            return self.events_after(id, after);
        };
        let execution = target(&session)?.unwrap();
        let client = self.linked_client(&execution).await?;
        let mut stream = client
            .subscribe(
                id.parse()
                    .map_err(|_| ApiError::invalid("Invalid run ID"))?,
                after,
            )
            .await
            .map_err(|e| e.downcast::<ApiError>().unwrap_or_else(|_| unavailable()))?;
        let mut events = Vec::new();
        let mut terminal = stream.terminal;
        while !terminal && events.len() < 100 {
            match tokio::time::timeout(
                Duration::from_millis(if events.is_empty() { 500 } else { 5 }),
                stream.next(),
            )
            .await
            {
                Ok(Ok(Some(event))) => events.push(event),
                Ok(Ok(None)) => terminal = true,
                Ok(Err(_)) => return Err(unavailable()),
                Err(_) => break,
            }
        }
        if let Some(last) = events.last() {
            self.store
                .connection
                .lock()
                .unwrap()
                .execute(
                    "UPDATE remote_runs SET observed_seq=max(observed_seq,?2) WHERE id=?1",
                    params![id, last["seq"].as_i64()],
                )
                .map_err(db_error)?;
        }
        Ok((events, terminal))
    }
}

#[cfg(all(test, target_os = "linux"))]
#[path = "remote_session_tests.rs"]
mod tests;
