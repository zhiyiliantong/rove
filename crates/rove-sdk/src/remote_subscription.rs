use crate::{RemoteClient, limited_body};
use rove_protocol::{Request, contract::AGENT, frame::MAX_FRAME_BYTES};
use serde_json::{Value, json};
use uuid::Uuid;

/// Remote observer bound to one network/device/run. Closing this value only
/// disconnects HTTP. Resume is explicit and never submits or cancels a run.
pub struct RemoteSubscription {
    response: Option<reqwest::Response>,
    decoder: Decoder,
    pending: Vec<u8>,
    cursor: usize,
    network_id: Uuid,
    device_id: Uuid,
    pub run_id: Uuid,
    pub last_seq: i64,
    pub terminal: bool,
    failed: bool,
}
impl RemoteClient {
    pub async fn subscribe(
        &self,
        run_id: Uuid,
        after_seq: i64,
    ) -> anyhow::Result<RemoteSubscription> {
        let mut request = Request::new("subscribe_run_events").with_path("run_id", run_id);
        request
            .query_parameters
            .insert("after_seq".into(), json!(after_seq));
        AGENT.request(&request)?;
        self.handshake().await?;
        let response = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            self.client
                .get(self.base_url.join(&format!("/v1/runs/{run_id}/events"))?)
                .header("X-Rove-Network-Id", self.network_id.to_string())
                .header("X-Rove-Target-Device-Id", self.device_id.to_string())
                .header("Accept", "text/event-stream")
                .query(&[("after_seq", after_seq)])
                .send(),
        )
        .await
        .map_err(|_| anyhow::anyhow!("Remote event handshake timed out"))??;
        let status = response.status().as_u16();
        if !matches!(status, 200 | 204) {
            let bytes = tokio::time::timeout(
                std::time::Duration::from_secs(5),
                limited_body(response, 64 * 1024),
            )
            .await
            .map_err(|_| anyhow::anyhow!("Remote event error response timed out"))??;
            let value: Value = serde_json::from_slice(&bytes)
                .map_err(|_| anyhow::anyhow!("Invalid remote event error response"))?;
            AGENT.validate("ErrorResponse", &value)?;
            let mut error: rove_protocol::ApiError =
                serde_json::from_value(value["error"].clone())?;
            error.status = status;
            return Err(error.into());
        }
        if status == 200 {
            anyhow::ensure!(
                response
                    .headers()
                    .get(reqwest::header::CONTENT_TYPE)
                    .and_then(|v| v.to_str().ok())
                    .is_some_and(|v| v
                        .split(';')
                        .next()
                        .unwrap_or("")
                        .trim()
                        .eq_ignore_ascii_case("text/event-stream")),
                "Expected text/event-stream"
            );
        }
        Ok(RemoteSubscription {
            response: if status == 204 { None } else { Some(response) },
            decoder: Decoder::default(),
            pending: Vec::new(),
            cursor: 0,
            network_id: self.network_id,
            device_id: self.device_id,
            run_id,
            last_seq: after_seq,
            terminal: status == 204,
            failed: false,
        })
    }
    pub async fn resume(
        &self,
        previous: &RemoteSubscription,
    ) -> anyhow::Result<RemoteSubscription> {
        anyhow::ensure!(
            self.network_id == previous.network_id && self.device_id == previous.device_id,
            "target_mismatch"
        );
        self.subscribe(previous.run_id, previous.last_seq).await
    }
}
impl RemoteSubscription {
    pub async fn next(&mut self) -> anyhow::Result<Option<Value>> {
        anyhow::ensure!(!self.failed, "Event stream failed; resume from last_seq");
        let result = self.read_next().await;
        if result.is_err() {
            self.failed = true;
            self.response = None;
        }
        result
    }
    async fn read_next(&mut self) -> anyhow::Result<Option<Value>> {
        if self.terminal {
            return Ok(None);
        }
        loop {
            while self.cursor < self.pending.len() {
                let byte = self.pending[self.cursor];
                self.cursor += 1;
                if let Some((id, event, data)) = self.decoder.byte(byte)? {
                    anyhow::ensure!(event == "run_event", "Unexpected SSE event type");
                    let value: Value = serde_json::from_str(&data)
                        .map_err(|_| anyhow::anyhow!("Invalid event JSON"))?;
                    AGENT.validate("RunEvent", &value)?;
                    anyhow::ensure!(
                        value["run_id"] == self.run_id.to_string(),
                        "Unexpected run_id"
                    );
                    let seq = value["seq"].as_i64().unwrap();
                    anyhow::ensure!(
                        !id.is_empty()
                            && id.bytes().all(|b| b.is_ascii_digit())
                            && id.parse::<i64>().ok() == Some(seq),
                        "SSE id does not match event seq"
                    );
                    if seq <= self.last_seq {
                        continue;
                    }
                    anyhow::ensure!(
                        seq == self.last_seq + 1,
                        "Event sequence gap; restore from snapshot"
                    );
                    self.last_seq = seq;
                    self.terminal = value["kind"] == "status"
                        && matches!(
                            value["data"]["status"].as_str(),
                            Some("succeeded" | "failed" | "cancelled" | "interrupted")
                        );
                    if self.terminal {
                        self.response = None;
                    }
                    return Ok(Some(value));
                }
            }
            let chunk = self
                .response
                .as_mut()
                .ok_or_else(|| anyhow::anyhow!("Subscription closed"))?
                .chunk()
                .await?
                .ok_or_else(|| {
                    anyhow::anyhow!("Event transport disconnected; resume from last_seq")
                })?;
            anyhow::ensure!(
                chunk.len() <= MAX_FRAME_BYTES,
                "SSE chunk exceeds size limit"
            );
            self.pending = chunk.to_vec();
            self.cursor = 0;
        }
    }
}

/// Incremental SSE framing with an encoded 8 MiB event limit. CR, LF, CRLF,
/// UTF-8 and BOM can span network chunks. State survives cancellation of next.
#[derive(Default)]
struct Decoder {
    line: Vec<u8>,
    data: String,
    id: String,
    event: String,
    skip_lf: bool,
    started: bool,
    bytes: usize,
}
impl Decoder {
    fn byte(&mut self, byte: u8) -> anyhow::Result<Option<(String, String, String)>> {
        self.bytes += 1;
        anyhow::ensure!(
            self.bytes <= MAX_FRAME_BYTES,
            "SSE event exceeds size limit"
        );
        if self.skip_lf {
            self.skip_lf = false;
            if byte == b'\n' {
                return Ok(None);
            }
        }
        if !matches!(byte, b'\r' | b'\n') {
            self.line.push(byte);
            return Ok(None);
        }
        self.skip_lf = byte == b'\r';
        let raw = std::mem::take(&mut self.line);
        let mut line =
            std::str::from_utf8(&raw).map_err(|_| anyhow::anyhow!("Invalid SSE UTF-8"))?;
        if !self.started {
            self.started = true;
            line = line.strip_prefix('\u{feff}').unwrap_or(line);
        }
        if line.is_empty() {
            self.bytes = 0;
            let data = std::mem::take(&mut self.data);
            let event = std::mem::take(&mut self.event);
            let id = std::mem::take(&mut self.id);
            if data.is_empty() {
                return Ok(None);
            }
            return Ok(Some((
                id,
                event,
                data.strip_suffix('\n').unwrap_or(&data).to_owned(),
            )));
        }
        if line.starts_with(':') {
            return Ok(None);
        }
        let (field, value) = line.split_once(':').unwrap_or((line, ""));
        let value = value.strip_prefix(' ').unwrap_or(value);
        match field {
            "data" => {
                self.data.push_str(value);
                self.data.push('\n');
            }
            "event" => self.event = value.to_owned(),
            "id" => {
                anyhow::ensure!(!value.contains('\0'), "Invalid SSE id");
                self.id = value.to_owned();
            }
            _ => {}
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn framing_handles_unicode_bom_multiline_and_all_line_endings() {
        for newline in ["\n", "\r", "\r\n"] {
            let input = [
                "\u{feff}: heartbeat",
                "",
                "id: 3",
                "event: run_event",
                "data: 漫游",
                "data: 者",
                "",
                "",
            ]
            .join(newline);
            let mut decoder = Decoder::default();
            let mut events = Vec::new();
            for byte in input.bytes() {
                if let Some(event) = decoder.byte(byte).unwrap() {
                    events.push(event);
                }
            }
            assert_eq!(
                events,
                vec![("3".into(), "run_event".into(), "漫游\n者".into())]
            );
        }
    }
    #[test]
    fn unterminated_event_and_invalid_utf8_are_bounded() {
        let mut decoder = Decoder {
            bytes: MAX_FRAME_BYTES,
            ..Decoder::default()
        };
        assert!(decoder.byte(b'x').is_err());
        let mut decoder = Decoder::default();
        decoder.byte(0xff).unwrap();
        assert!(decoder.byte(b'\n').is_err());
    }
}
