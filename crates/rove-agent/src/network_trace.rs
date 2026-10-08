//! Opt-in, bounded native-network diagnostics. Never serialize upstream config,
//! event strings, URLs with credentials, network names, keys or tool payloads.
use serde_json::{Value, json};
use std::{
    fs::File,
    io::Write,
    path::Path,
    sync::Mutex,
    time::{Duration, Instant},
};

pub(super) struct Trace(Mutex<(File, usize, Instant)>);

impl Trace {
    pub(super) fn open(directory: &Path) -> Option<Self> {
        if std::env::var("ROVE_NETWORK_TRACE").as_deref() != Ok("1") {
            return None;
        }
        let path = directory.join(format!("network-trace-{}.jsonl", uuid::Uuid::new_v4()));
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let file = options.open(path).ok()?;
        Some(Self(Mutex::new((
            file,
            4 * 1024 * 1024,
            Instant::now() - Duration::from_secs(2),
        ))))
    }

    pub(super) fn record(&self, network: &str, info: &Value) {
        let Ok(mut state) = self.0.lock() else {
            return;
        };
        if state.1 == 0 || state.2.elapsed() < Duration::from_secs(2) {
            return;
        }
        state.2 = Instant::now();
        let mut snapshot = sanitized(info);
        snapshot["network_id"] = json!(network);
        snapshot["at"] = json!(rove_core::now());
        let Ok(mut bytes) = serde_json::to_vec(&snapshot) else {
            return;
        };
        bytes.push(b'\n');
        if bytes.len() > state.1 {
            state.1 = 0;
            return;
        }
        state.1 -= bytes.len();
        if state.0.write_all(&bytes).is_err() {
            state.1 = 0;
        }
    }
}

fn number(value: &Value) -> Value {
    value.as_i64().map_or(Value::Null, |n| json!(n))
}
fn endpoint(value: &Value) -> Value {
    let Some(url) = value.as_str().and_then(|v| url::Url::parse(v).ok()) else {
        return Value::Null;
    };
    json!({"scheme":url.scheme(),"host":url.host_str(),"port":url.port()})
}
fn sanitized(info: &Value) -> Value {
    let routes: Vec<_> = info["routes"].as_array().into_iter().flatten().take(128).map(|r|json!({
        "peer_id":number(&r["peer_id"]),"ipv4":number(&r["ipv4_addr"]["address"]["addr"]),
        "next_hop":number(&r["next_hop_peer_id"]),"cost":number(&r["cost"]),"latency":number(&r["path_latency"])
    })).collect();
    let peers: Vec<_> = info["peers"].as_array().into_iter().flatten().take(128).map(|p|{
        let conns:Vec<_>=p["conns"].as_array().into_iter().flatten().take(8).map(|c|json!({
            "peer_id":number(&c["peer_id"]),"closed":c["is_closed"].as_bool(),
            "local":endpoint(&c["tunnel"]["local_addr"]["url"]),"remote":endpoint(&c["tunnel"]["remote_addr"]["url"]),
            "rx_packets":number(&c["stats"]["rx_packets"]),"tx_packets":number(&c["stats"]["tx_packets"]),"latency_us":number(&c["stats"]["latency_us"])
        })).collect();
        json!({"peer_id":number(&p["peer_id"]),"conns":conns})
    }).collect();
    json!({"running":info["running"].as_bool(),"peer_id":number(&info["my_node_info"]["peer_id"]),
        "route_count":info["routes"].as_array().map(Vec::len),"peer_count":info["peers"].as_array().map(Vec::len),"routes":routes,"peers":peers})
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn trace_is_allowlisted_and_bounded() {
        let secret = "do-not-record";
        let row = json!({"peer_id":5,"ipv4_addr":{"address":{"addr":123}},"hostname":secret,"network_secret":secret});
        let input = json!({"running":true,"my_node_info":{"peer_id":9,"config":secret},"events":[secret],"routes":vec![row;200],"peers":[{"peer_id":5,"conns":[{"network_name":secret,"stats":{"rx_packets":3},"tunnel":{"remote_addr":{"url":"tcp://user:do-not-record@127.0.0.1:11010/path?secret=do-not-record"}}}]}]});
        let result = sanitized(&input);
        assert!(!result.to_string().contains(secret));
        assert_eq!(result["route_count"], 200);
        assert_eq!(result["routes"].as_array().unwrap().len(), 128);
        assert_eq!(
            result["peers"][0]["conns"][0]["remote"],
            json!({"scheme":"tcp","host":"127.0.0.1","port":11010})
        );
    }
}
