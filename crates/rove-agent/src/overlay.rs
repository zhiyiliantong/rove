//! Upstream-specific management client. No EasyTier types escape this module.
use easytier::{
    proto::{
        api::manage::{
            CollectNetworkInfoRequest, DeleteNetworkInstanceRequest, ListNetworkInstanceRequest,
            NetworkConfig, RunNetworkInstanceRequest, WebClientServiceClientFactory,
        },
        rpc_impl::standalone::StandAloneClient,
        rpc_types::controller::BaseController,
    },
    tunnel::tcp::TcpTunnelConnector,
};
use serde_json::{Value, json};
use std::net::SocketAddr;
use tokio::sync::Mutex;
use uuid::Uuid;
pub const EASYTIER_REVISION: &str = "8428a89d2dabc94c97d370ec607c6ca142473626";
pub const EASYTIER_VERSION: &str = "2.6.4-8428a89d";
pub struct EasyTier {
    client: Mutex<StandAloneClient<TcpTunnelConnector>>,
}
impl EasyTier {
    /// Start a single adapter-owned network. The caller serializes lifecycle
    /// operations and must confirm old instances have exited before calling.
    pub async fn start(
        &self,
        instance_id: Uuid,
        join: &Value,
        listener: &str,
    ) -> anyhow::Result<()> {
        self.start_with_address(instance_id, join, listener, None)
            .await
    }
    pub async fn start_with_address(
        &self,
        instance_id: Uuid,
        join: &Value,
        listener: &str,
        local_ipv4: Option<&Value>,
    ) -> anyhow::Result<()> {
        crate::sharing::validate_join(join)?;
        let address = crate::networks::validate_local_address(join, local_ipv4, true)?;
        let url: url::Url = listener.parse()?;
        anyhow::ensure!(
            url.scheme() == "tcp"
                && url.host().is_some()
                && url.port().is_some()
                && url.username().is_empty()
                && url.password().is_none()
                && url.query().is_none()
                && url.fragment().is_none(),
            "Expected an explicit TCP overlay listener"
        );
        let config = NetworkConfig {
            instance_id: Some(instance_id.to_string()),
            dhcp: Some(join["easytier"]["dhcp"] == true),
            virtual_ipv4: address.map(|v| v.to_string()),
            network_length: if address.is_some() {
                Some(
                    join["easytier"]["ipv4_cidr"]
                        .as_str()
                        .unwrap()
                        .parse::<ipnet::Ipv4Net>()?
                        .prefix_len() as i32,
                )
            } else {
                None
            },
            hostname: Some(format!("rove-{}", &instance_id.simple().to_string()[..8])),
            network_name: Some(join["easytier"]["network_name"].as_str().unwrap().into()),
            network_secret: Some(join["easytier"]["network_secret"].as_str().unwrap().into()),
            networking_method: Some(1),
            peer_urls: join["easytier"]["bootstrap_peers"]
                .as_array()
                .unwrap()
                .iter()
                .map(|value| value.as_str().unwrap().into())
                .collect(),
            listener_urls: vec![listener.into()],
            advanced_settings: Some(true),
            no_tun: Some(false),
            dev_name: Some(format!("rove{}", &instance_id.simple().to_string()[..8])),
            disable_upnp: Some(true),
            disable_ipv6: Some(true),
            ..Default::default()
        };
        let client = self
            .client
            .lock()
            .await
            .scoped_client::<WebClientServiceClientFactory<BaseController>>(String::new())
            .await?;
        client
            .run_network_instance(
                BaseController::default(),
                RunNetworkInstanceRequest {
                    inst_id: Some(instance_id.into()),
                    config: Some(config),
                    overwrite: false,
                    source: 1,
                },
            )
            .await?;
        Ok(())
    }
    pub fn connect(portal: SocketAddr) -> anyhow::Result<Self> {
        anyhow::ensure!(
            portal.ip().is_loopback(),
            "EasyTier management portal must be loopback"
        );
        Ok(Self {
            client: Mutex::new(StandAloneClient::new(TcpTunnelConnector::new(
                format!("tcp://{portal}").parse()?,
            ))),
        })
    }
    pub async fn list_instances(&self) -> anyhow::Result<Vec<Uuid>> {
        let client = self
            .client
            .lock()
            .await
            .scoped_client::<WebClientServiceClientFactory<BaseController>>(String::new())
            .await?;
        let response = client
            .list_network_instance(BaseController::default(), ListNetworkInstanceRequest {})
            .await?;
        Ok(response.inst_ids.into_iter().map(Into::into).collect())
    }
    /// Management-only probe, not a production network start.
    /// No TUN, public listener or bootstrap connection is created here.
    pub async fn probe_start(&self, instance_id: Uuid, join: &Value) -> anyhow::Result<()> {
        crate::sharing::validate_join(join)?;
        anyhow::ensure!(
            join["easytier"]["bootstrap_peers"]
                .as_array()
                .unwrap()
                .is_empty(),
            "Management probe must not connect to external peers"
        );
        let config = NetworkConfig {
            instance_id: Some(instance_id.to_string()),
            dhcp: Some(true),
            hostname: Some(format!("rove-{}", &instance_id.simple().to_string()[..8])),
            network_name: Some(join["easytier"]["network_name"].as_str().unwrap().into()),
            network_secret: Some(join["easytier"]["network_secret"].as_str().unwrap().into()),
            networking_method: Some(1),
            peer_urls: join["easytier"]["bootstrap_peers"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap().into())
                .collect(),
            listener_urls: vec!["tcp://127.0.0.1:0".into()],
            advanced_settings: Some(true),
            no_tun: Some(true),
            disable_upnp: Some(true),
            disable_ipv6: Some(true),
            ..Default::default()
        };
        let client = self
            .client
            .lock()
            .await
            .scoped_client::<WebClientServiceClientFactory<BaseController>>(String::new())
            .await?;
        client
            .run_network_instance(
                BaseController::default(),
                RunNetworkInstanceRequest {
                    inst_id: Some(instance_id.into()),
                    config: Some(config),
                    overwrite: false,
                    source: 1,
                },
            )
            .await?;
        Ok(())
    }
    pub async fn stop(&self, instance_id: Uuid) -> anyhow::Result<()> {
        let client = self
            .client
            .lock()
            .await
            .scoped_client::<WebClientServiceClientFactory<BaseController>>(String::new())
            .await?;
        client
            .delete_network_instance(
                BaseController::default(),
                DeleteNetworkInstanceRequest {
                    inst_ids: vec![instance_id.into()],
                },
            )
            .await?;
        Ok(())
    }
    pub async fn info(&self, instance_id: Uuid) -> anyhow::Result<Value> {
        let client = self
            .client
            .lock()
            .await
            .scoped_client::<WebClientServiceClientFactory<BaseController>>(String::new())
            .await?;
        let response = client
            .collect_network_info(
                BaseController::default(),
                CollectNetworkInfoRequest {
                    inst_ids: vec![instance_id.into()],
                },
            )
            .await?;
        // Retain only runtime information. Configuration/secret export is a
        // separate explicit Rove operation.
        Ok(json!(response.info))
    }
}
