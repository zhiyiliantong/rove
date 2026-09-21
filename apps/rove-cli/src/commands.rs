use clap::{Args, Subcommand};
use rove_protocol::Request;
use serde_json::{Value, json};
use std::io::{IsTerminal, Read};
use uuid::Uuid;

#[derive(Args, Default)]
pub struct Page {
    #[arg(long, value_parser=clap::value_parser!(u16).range(1..=100))]
    pub limit: Option<u16>,
    #[arg(long)]
    pub cursor: Option<String>,
}
impl Page {
    pub fn apply(self, mut request: Request) -> Request {
        if let Some(limit) = self.limit {
            request
                .query_parameters
                .insert("limit".into(), json!(limit));
        }
        if let Some(cursor) = self.cursor {
            request
                .query_parameters
                .insert("cursor".into(), json!(cursor));
        }
        request
    }
}

#[derive(Subcommand)]
pub enum Network {
    List {
        #[command(flatten)]
        page: Page,
    },
    Show {
        network_id: Uuid,
    },
    Create {
        name: String,
        #[arg(long)]
        bootstrap_peer: Vec<String>,
        /// Use manual addressing in this subnet instead of EasyTier DHCP.
        #[arg(long)]
        subnet: Option<String>,
        /// This device's static host IP, never included in shared configuration.
        #[arg(long, requires = "subnet")]
        local_ipv4: Option<String>,
    },
    /// Test TCP reachability only; no network join or credential exchange.
    TestPeer {
        endpoint: String,
    },
    /// Import a share URL; use '-' to read it from stdin without shell history.
    Join {
        url: String,
    },
    /// Import a full JoinConfig JSON. Use --body - for private stdin input.
    Import {
        #[arg(long, default_value = "-")]
        body: String,
    },
    /// Explicitly replace the network display name and EasyTier join settings.
    Update {
        network_id: Uuid,
        #[arg(long, default_value = "-")]
        body: String,
    },
    /// Join this network if its actual subnet does not overlap another network.
    Start {
        network_id: Uuid,
    },
    /// Leave this device's current network, retaining its saved configuration.
    Stop {
        network_id: Uuid,
    },
    /// Delete this device's stopped network configuration; not member revocation.
    Delete {
        network_id: Uuid,
    },
    /// Explicitly export secret join configuration.
    Export {
        network_id: Uuid,
    },
    /// Sharing grants trusted network access; QR and URL contain the same key.
    Share {
        network_id: Uuid,
        #[arg(long)]
        config_server: Option<String>,
        #[arg(long)]
        qr: bool,
    },
}
impl Network {
    pub fn request(self) -> anyhow::Result<(Request, bool)> {
        let mut qr = false;
        let request = match self {
            Self::List { page } => page.apply(Request::new("list_networks")),
            Self::Show { network_id } => {
                Request::new("get_network").with_path("network_id", network_id)
            }
            Self::Create {
                name,
                bootstrap_peer,
                subnet,
                local_ipv4,
            } => {
                let mut body = json!({"display_name":name});
                if let Some(subnet) = subnet {
                    body["config"] = json!({"network_name":format!("rove-{}",Uuid::new_v4()),"network_secret":format!("{}{}",Uuid::new_v4().simple(),Uuid::new_v4().simple()),"bootstrap_peers":bootstrap_peer,"dhcp":false,"ipv4_cidr":subnet});
                    body["local_ipv4"] = json!(local_ipv4);
                } else if !bootstrap_peer.is_empty() {
                    body["bootstrap_peers"] = json!(bootstrap_peer);
                }
                Request::new("create_network").with_body(body)
            }
            Self::TestPeer { endpoint } => {
                Request::new("test_network_peer").with_body(json!({"endpoint":endpoint}))
            }
            Self::Join { url } => Request::new("import_network")
                .with_body(json!({"source":"url","url":input(&url)?.trim()})),
            Self::Import { body } => Request::new("import_network")
                .with_body(json!({"source":"manual","config":json_input(&body)?})),
            Self::Update { network_id, body } => Request::new("update_network")
                .with_path("network_id", network_id)
                .with_body(json_input(&body)?),
            Self::Start { network_id } => {
                Request::new("start_network").with_path("network_id", network_id)
            }
            Self::Stop { network_id } => {
                Request::new("stop_network").with_path("network_id", network_id)
            }
            Self::Delete { network_id } => {
                Request::new("delete_network").with_path("network_id", network_id)
            }
            Self::Export { network_id } => {
                Request::new("get_network_join_config").with_path("network_id", network_id)
            }
            Self::Share {
                network_id,
                config_server,
                qr: render,
            } => {
                qr = render;
                let body = config_server.map_or(json!({}), |url| json!({"config_server_url":url}));
                Request::new("create_network_share")
                    .with_path("network_id", network_id)
                    .with_body(body)
            }
        };
        Ok((request, qr))
    }
}

#[derive(Subcommand)]
pub enum Device {
    Show,
    /// Inspect Rove-owned storage metadata; does not read secrets or clean files.
    Storage,
    List {
        network_id: Uuid,
        #[command(flatten)]
        page: Page,
    },
}
impl Device {
    pub fn request(self) -> Request {
        match self {
            Self::Show => Request::new("get_device"),
            Self::Storage => Request::new("get_storage_usage"),
            Self::List { network_id, page } => {
                page.apply(Request::new("list_network_devices").with_path("network_id", network_id))
            }
        }
    }
}

#[derive(Subcommand)]
pub enum Model {
    Show,
    List,
    /// Copy an API connection and all its models to one explicitly selected peer.
    Sync {
        connection_id: Uuid,
        #[arg(long)]
        to_network: Uuid,
        #[arg(long)]
        to_device: Uuid,
        /// Replace this existing target connection; requires --yes.
        #[arg(long)]
        replace_connection: Option<Uuid>,
        /// Confirm overwriting changed target configuration (never account login).
        #[arg(long)]
        yes: bool,
    },
    /// Receive an API sync snapshot from private stdin (agent-to-agent parity).
    ReceiveSync {
        #[arg(long, default_value = "-")]
        body: String,
    },
    /// Query provider model IDs without storing credentials.
    Discover {
        #[arg(long, default_value = "-")]
        body: String,
    },
    /// Test inference without tools; credentials or model_id are read from stdin.
    Test {
        #[arg(long, default_value = "-")]
        body: String,
    },
    /// Add one API connection and multiple models; credentials are read from stdin.
    Import {
        #[arg(long, default_value = "-")]
        body: String,
    },
    Rename {
        model_id: Uuid,
        name: String,
    },
    Default {
        model_id: Option<Uuid>,
    },
    UpdateConnection {
        connection_id: Uuid,
        #[arg(long, default_value = "-")]
        body: String,
    },
    DeleteConnection {
        connection_id: Uuid,
        #[arg(long)]
        yes: bool,
    },
    /// Replace model configuration with ModelConfigWrite JSON; stdin is private.
    Set {
        #[arg(long, default_value = "-")]
        body: String,
    },
    Clear,
}
impl Model {
    pub fn request(self) -> anyhow::Result<Request> {
        Ok(match self {
            Self::Show => Request::new("get_model_config"),
            Self::List => Request::new("get_model_catalog"),
            Self::Sync {
                connection_id,
                to_network,
                to_device,
                replace_connection,
                yes,
            } => {
                anyhow::ensure!(
                    replace_connection.is_none() || yes,
                    "Use --yes to confirm replacing the selected target connection"
                );
                let mut body = json!({"target":{"network_id":to_network,"device_id":to_device},"overwrite":yes});
                if let Some(id) = replace_connection {
                    body["replace_connection_id"] = json!(id);
                }
                Request::new("sync_model_connection")
                    .with_path("connection_id", connection_id)
                    .with_body(body)
            }
            Self::ReceiveSync { body } => {
                Request::new("receive_model_sync").with_body(json_input(&body)?)
            }
            Self::Discover { body } => {
                Request::new("discover_models").with_body(json_input(&body)?)
            }
            Self::Test { body } => Request::new("test_model").with_body(json_input(&body)?),
            Self::Import { body } => Request::new("import_models").with_body(json_input(&body)?),
            Self::Rename { model_id, name } => Request::new("rename_model")
                .with_path("model_id", model_id)
                .with_body(json!({"name":name})),
            Self::Default { model_id } => {
                Request::new("set_default_model").with_body(json!({"model_id":model_id}))
            }
            Self::UpdateConnection {
                connection_id,
                body,
            } => Request::new("update_model_connection")
                .with_path("connection_id", connection_id)
                .with_body(json_input(&body)?),
            Self::DeleteConnection { connection_id, yes } => {
                anyhow::ensure!(
                    yes,
                    "Use --yes to remove this connection and all its model entries; history is retained"
                );
                Request::new("delete_model_connection").with_path("connection_id", connection_id)
            }
            Self::Set { body } => Request::new("set_model_config").with_body(json_input(&body)?),
            Self::Clear => Request::new("clear_model_config"),
        })
    }
}

#[derive(Subcommand)]
pub enum Config {
    Show,
    Set {
        #[arg(long,value_parser=clap::value_parser!(u64).range(1..))]
        max_active_runs: Option<u64>,
        #[arg(long, conflicts_with = "clear_config_server")]
        config_server: Option<String>,
        #[arg(long)]
        clear_config_server: bool,
    },
}
impl Config {
    pub fn request(self) -> anyhow::Result<Request> {
        Ok(match self {
            Self::Show => Request::new("get_settings"),
            Self::Set {
                max_active_runs,
                config_server,
                clear_config_server,
            } => {
                let mut body = json!({});
                if let Some(n) = max_active_runs {
                    body["max_active_runs"] = json!(n);
                }
                if let Some(url) = config_server {
                    body["config_server_url"] = json!(url);
                }
                if clear_config_server {
                    body["config_server_url"] = Value::Null;
                }
                anyhow::ensure!(
                    !body.as_object().unwrap().is_empty(),
                    "Provide a setting to update"
                );
                Request::new("update_settings").with_body(body)
            }
        })
    }
}

#[derive(Subcommand)]
pub enum Service {
    /// Request the local default browser to open a published HTTP/HTTPS endpoint.
    Open {
        service_id: Uuid,
        #[arg(long, default_value_t = 0)]
        endpoint: usize,
    },
    List {
        #[arg(long)]
        network_id: Uuid,
        #[command(flatten)]
        page: Page,
    },
    Show {
        service_id: Uuid,
    },
    /// Publish using ServiceWrite JSON; access_info may contain application secrets.
    Publish {
        #[arg(long, default_value = "-")]
        body: String,
    },
    Update {
        service_id: Uuid,
        #[arg(long, default_value = "-")]
        body: String,
    },
    /// Close the Rove proxy, preserving the target application and its data.
    Unpublish {
        service_id: Uuid,
    },
}
impl Service {
    pub fn request(self) -> anyhow::Result<Request> {
        Ok(match self {
            Self::List { network_id, page } => {
                let mut request = page.apply(Request::new("list_services"));
                request
                    .query_parameters
                    .insert("network_id".into(), json!(network_id));
                request
            }
            Self::Show { service_id } | Self::Open { service_id, .. } => {
                Request::new("get_service").with_path("service_id", service_id)
            }
            Self::Publish { body } => Request::new("publish_service").with_body(json_input(&body)?),
            Self::Update { service_id, body } => Request::new("update_service")
                .with_path("service_id", service_id)
                .with_body(json_input(&body)?),
            Self::Unpublish { service_id } => {
                Request::new("unpublish_service").with_path("service_id", service_id)
            }
        })
    }
}

pub fn browser_endpoint(service: &Value, index: usize) -> anyhow::Result<String> {
    anyhow::ensure!(
        service["state"] == "published",
        "Service is not currently published"
    );
    let address = service["endpoints"]
        .as_array()
        .and_then(|items| items.get(index))
        .and_then(Value::as_str)
        .ok_or_else(|| {
            anyhow::anyhow!("Service endpoint is unavailable; refresh the service directory")
        })?;
    let url = url::Url::parse(address).map_err(|_| anyhow::anyhow!("Invalid service endpoint"))?;
    anyhow::ensure!(
        matches!(url.scheme(), "http" | "https")
            && url.host().is_some()
            && url.username().is_empty()
            && url.password().is_none(),
        "Only HTTP/HTTPS service URLs without embedded credentials can open in a browser; use the appropriate client for TCP"
    );
    Ok(url.to_string())
}

pub fn input(value: &str) -> anyhow::Result<String> {
    if value != "-" {
        return Ok(value.to_owned());
    }
    anyhow::ensure!(
        !std::io::stdin().is_terminal(),
        "Pipe input on stdin when using '-'; use interactive mode for prompts"
    );
    let mut input = String::new();
    std::io::stdin()
        .take(8 * 1024 * 1024 + 1)
        .read_to_string(&mut input)?;
    anyhow::ensure!(input.len() <= 8 * 1024 * 1024, "Input exceeds frame limit");
    Ok(input)
}
pub fn json_input(value: &str) -> anyhow::Result<Value> {
    serde_json::from_str(&input(value)?).map_err(|_| anyhow::anyhow!("Invalid JSON input"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn browser_open_requires_a_published_web_endpoint() {
        let service = json!({"state":"published","endpoints":["http://10.1.2.3:8000","https://[fd00::1]:8443/music"]});
        assert_eq!(
            browser_endpoint(&service, 0).unwrap(),
            "http://10.1.2.3:8000/"
        );
        assert_eq!(
            browser_endpoint(&service, 1).unwrap(),
            "https://[fd00::1]:8443/music"
        );
        assert!(browser_endpoint(&service, 2).is_err());
        for state in ["unavailable", "failed"] {
            assert!(
                browser_endpoint(&json!({"state":state,"endpoints":service["endpoints"]}), 0)
                    .is_err()
            );
        }
        for address in [
            "tcp://10.1.2.3:8000",
            "file:///tmp/a",
            "javascript:alert(1)",
            "https://user:secret@example.com",
            "not a url",
        ] {
            assert!(
                browser_endpoint(&json!({"state":"published","endpoints":[address]}), 0).is_err()
            );
        }
    }
}
