use crate::LocalClient;
use rove_protocol::{
    Request, Response, check_protocol,
    contract::AGENT,
    frame::{FrameReader, read_frame, write_frame},
};
use serde_json::{Value, json};
use uuid::Uuid;
#[cfg(unix)]
type LocalStream = tokio::net::UnixStream;
#[cfg(windows)]
type LocalStream = tokio::net::windows::named_pipe::NamedPipeClient;
pub struct Subscription {
    stream: Option<LocalStream>,
    reader: FrameReader,
    pub subscription_id: Option<Uuid>,
    pub last_seq: i64,
    pub terminal: bool,
    pub run_id: Uuid,
    target: Option<rove_protocol::SocketTarget>,
}
impl LocalClient {
    /// Bounded long-poll bridge for UI hosts. Every call observes existing work;
    /// dropping it only closes a socket. No detached tasks or run cancellation.
    /// A failed/partial batch is replayed from the caller's last applied cursor.
    pub async fn poll_events(
        &self,
        run_id: Uuid,
        after_seq: i64,
        target: Option<rove_protocol::SocketTarget>,
    ) -> anyhow::Result<Value> {
        let mut subscription = self.subscribe_target(run_id, after_seq, target).await?;
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(750);
        let mut events = Vec::new();
        let mut bytes = 0;
        while events.len() < 64 && bytes < 512 * 1024 {
            match tokio::time::timeout_at(deadline, subscription.next()).await {
                Ok(Ok(Some(event))) => {
                    bytes += event.to_string().len();
                    events.push(event);
                }
                Ok(Ok(None)) => break,
                Ok(Err(error)) => return Err(error),
                Err(_) => break,
            }
        }
        Ok(
            json!({"events":events,"last_seq":subscription.last_seq,"terminal":subscription.terminal}),
        )
    }
    pub async fn subscribe(&self, run_id: Uuid, after_seq: i64) -> anyhow::Result<Subscription> {
        self.subscribe_target(run_id, after_seq, None).await
    }
    pub async fn subscribe_target(
        &self,
        run_id: Uuid,
        after_seq: i64,
        target: Option<rove_protocol::SocketTarget>,
    ) -> anyhow::Result<Subscription> {
        self.subscribe_with_deadline(run_id, after_seq, target, std::time::Duration::from_secs(5))
            .await
    }
    pub(crate) async fn subscribe_with_deadline(
        &self,
        run_id: Uuid,
        after_seq: i64,
        target: Option<rove_protocol::SocketTarget>,
        deadline: std::time::Duration,
    ) -> anyhow::Result<Subscription> {
        let mut request = Request::new("subscribe_run_events").with_path("run_id", run_id);
        request.target = target.clone();
        request
            .query_parameters
            .insert("after_seq".into(), json!(after_seq));
        AGENT.request(&request)?;
        tokio::time::timeout(
            deadline,
            self.open_subscription(request, run_id, after_seq, target),
        )
        .await
        .map_err(|_| anyhow::anyhow!("Event handshake timed out; run was not cancelled"))?
    }
    async fn open_subscription(
        &self,
        request: Request,
        run_id: Uuid,
        after_seq: i64,
        target: Option<rove_protocol::SocketTarget>,
    ) -> anyhow::Result<Subscription> {
        #[cfg(unix)]
        let mut stream = LocalStream::connect(self.endpoint()).await?;
        #[cfg(windows)]
        let mut stream = crate::open_pipe(self.endpoint()).await?;
        let hello = Request::new("get_hello");
        write_frame(&mut stream, &json!(hello)).await?;
        let response: Response = serde_json::from_value(
            read_frame(&mut stream)
                .await?
                .ok_or_else(|| anyhow::anyhow!("Disconnected during handshake"))?,
        )?;
        AGENT.response("get_hello", &response)?;
        anyhow::ensure!(
            response.status_code == 200 && response.correlation_id == hello.correlation_id,
            "Invalid handshake"
        );
        let hello = response.body.unwrap();
        check_protocol(
            hello["protocol"]["min"].as_u64().unwrap(),
            hello["protocol"]["max"].as_u64().unwrap(),
        )?;
        write_frame(&mut stream, &json!(request)).await?;
        let response: Response = serde_json::from_value(
            read_frame(&mut stream)
                .await?
                .ok_or_else(|| anyhow::anyhow!("Disconnected during subscription"))?,
        )?;
        AGENT.response("subscribe_run_events", &response)?;
        anyhow::ensure!(
            response.correlation_id == request.correlation_id,
            "Unexpected subscription response"
        );
        if response.status_code >= 400 {
            anyhow::bail!(
                "{}",
                response.body.unwrap()["error"]["code"]
                    .as_str()
                    .unwrap_or("subscription_failed")
            );
        }
        if response.status_code == 204 {
            return Ok(Subscription {
                stream: None,
                reader: FrameReader::default(),
                subscription_id: None,
                last_seq: after_seq,
                terminal: true,
                run_id,
                target,
            });
        }
        let body = response.body.unwrap();
        anyhow::ensure!(
            body["run_id"] == run_id.to_string(),
            "Unexpected run_id in subscription response"
        );
        let id = body["subscription_id"].as_str().unwrap().parse()?;
        Ok(Subscription {
            stream: Some(stream),
            reader: FrameReader::default(),
            subscription_id: Some(id),
            last_seq: after_seq,
            terminal: false,
            run_id,
            target,
        })
    }
    /// Reconnect without resubmitting the run. A 410 remains explicit: obtain
    /// get_run's snapshot and choose its snapshot_seq before subscribing again.
    pub async fn resume(&self, previous: &Subscription) -> anyhow::Result<Subscription> {
        self.subscribe_target(previous.run_id, previous.last_seq, previous.target.clone())
            .await
    }
}
impl Subscription {
    pub async fn next(&mut self) -> anyhow::Result<Option<Value>> {
        if self.terminal {
            return Ok(None);
        }
        let stream = self
            .stream
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Subscription is closed"))?;
        loop {
            let frame = self.reader.next(stream).await?.ok_or_else(|| {
                anyhow::anyhow!("Event transport disconnected; resume from last_seq")
            })?;
            match frame["kind"].as_str() {
                Some("event") => {
                    AGENT.validate("SocketEvent", &frame)?;
                    anyhow::ensure!(
                        frame["subscription_id"] == self.subscription_id.unwrap().to_string(),
                        "Unexpected subscription_id"
                    );
                    let event = &frame["event"];
                    anyhow::ensure!(
                        event["run_id"] == self.run_id.to_string(),
                        "Unexpected run_id"
                    );
                    let seq = event["seq"].as_i64().unwrap();
                    if seq <= self.last_seq {
                        continue;
                    }
                    anyhow::ensure!(
                        seq == self.last_seq + 1,
                        "Event sequence gap; restore from snapshot"
                    );
                    self.last_seq = seq;
                    return Ok(Some(event.clone()));
                }
                Some("stream_end") => {
                    AGENT.validate("SocketStreamEnd", &frame)?;
                    anyhow::ensure!(
                        frame["subscription_id"] == self.subscription_id.unwrap().to_string(),
                        "Unexpected subscription_id"
                    );
                    if frame["reason"] == "transport_error" {
                        anyhow::bail!("Event transport failed; resume from last_seq");
                    }
                    self.terminal = frame["reason"] == "terminal";
                    self.stream = None;
                    return Ok(None);
                }
                _ => anyhow::bail!("Unexpected event frame"),
            }
        }
    }
    pub async fn unsubscribe(&mut self) -> anyhow::Result<()> {
        if let Some(mut stream) = self.stream.take() {
            let correlation = Uuid::new_v4();
            write_frame(&mut stream,&json!({"kind":"unsubscribe","correlation_id":correlation,"subscription_id":self.subscription_id})).await?;
            while let Some(frame) = self.reader.next(&mut stream).await? {
                if frame["kind"] == "response" && frame["correlation_id"] == correlation.to_string()
                {
                    AGENT.validate("SocketResponse", &frame)?;
                    anyhow::ensure!(frame["status_code"] == 204, "Unsubscribe failed");
                    anyhow::ensure!(
                        frame.get("body").is_none(),
                        "Unsubscribe acknowledgement must omit body"
                    );
                    return Ok(());
                }
            }
            anyhow::bail!(
                "Disconnected before unsubscribe acknowledgement; the run is not cancelled"
            );
        }
        Ok(())
    }
}
