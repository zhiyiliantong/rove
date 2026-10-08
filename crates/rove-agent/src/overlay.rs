//! Upstream-specific management client. No EasyTier types escape this module.
#[cfg(unix)]
pub mod mobile_vpn;
#[cfg(unix)]
pub mod vpn_io;
pub mod vpn_plan;
#[cfg(all(windows, feature = "easytier-native"))]
pub mod windows_helper;
use easytier::{
    common::config::ConfigFileControl,
    instance_manager::NetworkInstanceManager,
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
    backend: Backend,
    startup_deadlines: std::sync::Mutex<std::collections::BTreeMap<Uuid, std::time::Instant>>,
    // Upstream mobile NIC deliberately does not close supplied descriptors.
    // Retain ownership until the corresponding instance has fully stopped.
    #[cfg(target_os = "android")]
    tun_descriptors: std::sync::Mutex<std::collections::BTreeMap<Uuid, std::os::fd::OwnedFd>>,
}
enum Backend {
    Rpc(Mutex<StandAloneClient<TcpTunnelConnector>>),
    Embedded(NetworkInstanceManager),
    #[cfg(all(windows, feature = "easytier-native"))]
    WindowsHelper(windows_helper::Client),
}

impl Drop for EasyTier {
    fn drop(&mut self) {
        // Upstream watcher tasks hold references to the instance map. Explicit
        // deletion joins the instance threads before owned mobile fds drop.
        self.shutdown_owned();
    }
}

impl EasyTier {
    pub(crate) fn owns_runtime(&self) -> bool {
        !matches!(self.backend, Backend::Rpc(_))
    }

    #[cfg(all(windows, feature = "easytier-native"))]
    pub(crate) async fn prepare_native_runtime(&self) -> anyhow::Result<()> {
        #[cfg(all(windows, feature = "easytier-native"))]
        if let Backend::WindowsHelper(client) = &self.backend {
            client.prepare().await?;
        }
        Ok(())
    }
    pub(crate) fn shutdown_owned(&self) {
        self.startup_deadlines.lock().unwrap().clear();
        #[cfg(all(windows, feature = "easytier-native"))]
        if let Backend::WindowsHelper(client) = &self.backend {
            client.shutdown();
        }
        if let Backend::Embedded(manager) = &self.backend {
            let _ = manager.retain_network_instance(Vec::new());
            #[cfg(target_os = "android")]
            self.tun_descriptors.lock().unwrap().clear();
        }
        // Never terminate an externally managed daemon or another app's work.
    }
    /// Application-owned upstream instances. This does not grant permission to
    /// create a TUN: the native platform must supply that capability separately.
    pub fn embedded() -> Self {
        Self {
            backend: Backend::Embedded(NetworkInstanceManager::new()),
            startup_deadlines: Default::default(),
            #[cfg(target_os = "android")]
            tun_descriptors: Default::default(),
        }
    }

    /// Attach one application-owned channel to a mobile instance. The caller
    /// must first obtain VPN consent and establish the native VPN. Passing an
    /// owned fd prevents JNI integers outliving their Java descriptor owner.
    #[cfg(target_os = "android")]
    pub fn attach_tun(
        &self,
        instance_id: Uuid,
        descriptor: std::os::fd::OwnedFd,
    ) -> anyhow::Result<()> {
        use std::os::fd::AsRawFd;
        let Backend::Embedded(manager) = &self.backend else {
            anyhow::bail!("A mobile TUN requires an application-owned runtime");
        };
        let mut descriptors = self.tun_descriptors.lock().unwrap();
        anyhow::ensure!(
            !descriptors.contains_key(&instance_id),
            "TUN is already attached; stop the instance before replacing its channel"
        );
        anyhow::ensure!(descriptor.as_raw_fd() > 0, "Invalid mobile TUN descriptor");
        manager.set_tun_fd(&instance_id, descriptor.as_raw_fd())?;
        descriptors.insert(instance_id, descriptor);
        Ok(())
    }

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
        #[cfg(all(windows, feature = "easytier-native"))]
        if let Backend::WindowsHelper(client) = &self.backend {
            // Validate with the same converter before crossing the privilege
            // boundary; the helper independently validates it again.
            network_config(instance_id, join, listener, local_ipv4)?;
            client
                .call(windows_helper::Request::Start {
                    instance_id,
                    join: join.clone(),
                    listener: listener.into(),
                    local_ipv4: local_ipv4.cloned(),
                })
                .await?;
            return Ok(());
        }
        self.run_config(
            instance_id,
            network_config(instance_id, join, listener, local_ipv4)?,
        )
        .await
    }

    async fn run_config(&self, instance_id: Uuid, config: NetworkConfig) -> anyhow::Result<()> {
        let Backend::Rpc(client) = &self.backend else {
            let Backend::Embedded(manager) = &self.backend else {
                unreachable!()
            };
            // Use the same upstream conversion as the RPC service, including
            // validation, rather than constructing a platform-specific TOML.
            let id = manager.run_network_instance(
                config.gen_config()?,
                true,
                ConfigFileControl::STATIC_CONFIG,
            )?;
            anyhow::ensure!(id == instance_id, "EasyTier instance identity mismatch");
            self.startup_deadlines.lock().unwrap().insert(
                instance_id,
                std::time::Instant::now() + std::time::Duration::from_secs(30),
            );
            return Ok(());
        };
        let client = client
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
            backend: Backend::Rpc(Mutex::new(StandAloneClient::new(TcpTunnelConnector::new(
                format!("tcp://{portal}").parse()?,
            )))),
            startup_deadlines: Default::default(),
            #[cfg(target_os = "android")]
            tun_descriptors: Default::default(),
        })
    }

    pub async fn list_instances(&self) -> anyhow::Result<Vec<Uuid>> {
        #[cfg(all(windows, feature = "easytier-native"))]
        if let Backend::WindowsHelper(client) = &self.backend {
            return Ok(serde_json::from_value(
                client.call(windows_helper::Request::List).await?,
            )?);
        }
        let Backend::Rpc(client) = &self.backend else {
            let Backend::Embedded(manager) = &self.backend else {
                unreachable!()
            };
            return Ok(manager.list_network_instance_ids());
        };
        let client = client
            .lock()
            .await
            .scoped_client::<WebClientServiceClientFactory<BaseController>>(String::new())
            .await?;
        let response = client
            .list_network_instance(BaseController::default(), ListNetworkInstanceRequest {})
            .await?;
        Ok(response.inst_ids.into_iter().map(Into::into).collect())
    }

    /// Management test only: no TUN and no connection to an external peer.
    pub async fn probe_start(&self, instance_id: Uuid, join: &Value) -> anyhow::Result<()> {
        let mut config = network_config(instance_id, join, "tcp://127.0.0.1:0", None)?;
        anyhow::ensure!(
            config.peer_urls.is_empty(),
            "Management probe must not connect to external peers"
        );
        config.no_tun = Some(true);
        self.run_config(instance_id, config).await
    }

    pub async fn stop(&self, instance_id: Uuid) -> anyhow::Result<()> {
        self.startup_deadlines.lock().unwrap().remove(&instance_id);
        #[cfg(all(windows, feature = "easytier-native"))]
        if let Backend::WindowsHelper(client) = &self.backend {
            client
                .call(windows_helper::Request::Stop { instance_id })
                .await?;
            return Ok(());
        }
        let Backend::Rpc(client) = &self.backend else {
            let Backend::Embedded(manager) = &self.backend else {
                unreachable!()
            };
            manager.delete_network_instance(vec![instance_id])?;
            #[cfg(target_os = "android")]
            self.tun_descriptors.lock().unwrap().remove(&instance_id);
            return Ok(());
        };
        let client = client
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
        #[cfg(all(windows, feature = "easytier-native"))]
        if let Backend::WindowsHelper(client) = &self.backend {
            return client
                .call(windows_helper::Request::Info { instance_id })
                .await;
        }
        let Backend::Rpc(client) = &self.backend else {
            let Backend::Embedded(manager) = &self.backend else {
                unreachable!()
            };
            let Some(info) = manager.get_network_info(&instance_id).await else {
                // The upstream management API appears only after native NIC
                // initialization. An owned pending instance is not missing,
                // and must never be mistaken for running either.
                let deadlines = self.startup_deadlines.lock().unwrap();
                anyhow::ensure!(
                    deadlines
                        .get(&instance_id)
                        .is_some_and(|deadline| std::time::Instant::now() < *deadline),
                    "EasyTier startup deadline exceeded or instance not found"
                );
                return Ok(json!({"map":{instance_id.to_string():{"running":false}}}));
            };
            self.startup_deadlines.lock().unwrap().remove(&instance_id);
            return Ok(json!({"map": {instance_id.to_string(): info}}));
        };
        let client = client
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
        Ok(json!(response.info))
    }
}

#[cfg(all(windows, feature = "easytier-native"))]
impl EasyTier {
    pub fn windows_native() -> Self {
        Self {
            backend: Backend::WindowsHelper(windows_helper::Client::unstarted()),
            startup_deadlines: Default::default(),
        }
    }
    pub async fn windows_helper(path: &std::path::Path, server_pid: u32) -> anyhow::Result<Self> {
        Ok(Self {
            backend: Backend::WindowsHelper(
                windows_helper::Client::connect(path, server_pid).await?,
            ),
            startup_deadlines: Default::default(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn join() -> Value {
        json!({"schema_version":1,"network_id":Uuid::new_v4(),"display_name":"runtime-test",
            "easytier":{"network_name":"runtime-test","network_secret":"test-only-secret",
                "dhcp":true,"bootstrap_peers":[]}})
    }

    #[test]
    fn both_backends_use_portable_identity_and_original_discovery_uri() {
        let mut join = join();
        join["easytier"]["bootstrap_peers"] = json!(["txt://bootstrap.example.com"]);
        let instance = Uuid::new_v4();
        let config = network_config(instance, &join, "tcp://0.0.0.0:0", None).unwrap();
        assert_eq!(config.instance_id, Some(instance.to_string()));
        assert_ne!(config.instance_id.as_deref(), join["network_id"].as_str());
        assert_eq!(config.peer_urls, ["txt://bootstrap.example.com"]);
        assert_eq!(config.dhcp, Some(true));
        assert_eq!(config.virtual_ipv4, None);
        assert_eq!(config.no_tun, Some(false));
        let config = config.gen_config().unwrap();
        use easytier::common::config::ConfigLoader;
        assert_eq!(config.get_id(), instance);
    }

    #[test]
    fn manual_address_must_be_local_and_inside_shared_subnet() {
        let mut join = join();
        join["easytier"]["dhcp"] = json!(false);
        join["easytier"]["ipv4_cidr"] = json!("10.203.8.0/24");
        let instance = Uuid::new_v4();
        assert!(network_config(instance, &join, "tcp://0.0.0.0:0", None).is_err());
        assert!(
            network_config(
                instance,
                &join,
                "tcp://0.0.0.0:0",
                Some(&json!("10.203.9.2"))
            )
            .is_err()
        );
        let config = network_config(
            instance,
            &join,
            "tcp://0.0.0.0:0",
            Some(&json!("10.203.8.2")),
        )
        .unwrap();
        assert_eq!(config.virtual_ipv4.as_deref(), Some("10.203.8.2"));
        assert_eq!(config.network_length, Some(24));
    }

    // Real upstream instances, but deliberately NO TUN or external peers.
    // This verifies backend ownership, not successful overlay membership.
    #[tokio::test]
    async fn embedded_instances_are_isolated_and_individually_stopped() {
        let runtime = EasyTier::embedded();
        let other = EasyTier::embedded();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        runtime.probe_start(a, &join()).await.unwrap();
        runtime.probe_start(b, &join()).await.unwrap();
        assert!(runtime.probe_start(a, &join()).await.is_err());
        assert_eq!(runtime.list_instances().await.unwrap().len(), 2);
        assert_eq!(runtime.startup_deadlines.lock().unwrap().len(), 2);
        assert!(other.list_instances().await.unwrap().is_empty());
        let info = tokio::time::timeout(std::time::Duration::from_secs(10), async {
            loop {
                if let Ok(info) = runtime.info(b).await
                    && info["map"][b.to_string()]["running"] == true
                {
                    break info;
                }
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            }
        })
        .await
        .unwrap();
        assert_eq!(
            info["map"][b.to_string()]["my_node_info"]["version"],
            EASYTIER_VERSION
        );
        assert!(info["map"][b.to_string()].get("config").is_none());
        runtime.stop(a).await.unwrap();
        assert_eq!(runtime.list_instances().await.unwrap(), vec![b]);
        runtime.stop(b).await.unwrap();
        assert!(runtime.list_instances().await.unwrap().is_empty());
        assert!(runtime.info(a).await.is_err());
        runtime.probe_start(a, &join()).await.unwrap();
        runtime.shutdown_owned();
        assert!(runtime.list_instances().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn pending_native_initialization_is_bounded_and_never_running() {
        let runtime = EasyTier::embedded();
        let id = Uuid::new_v4();
        assert!(runtime.info(id).await.is_err());
        runtime.startup_deadlines.lock().unwrap().insert(
            id,
            std::time::Instant::now() + std::time::Duration::from_secs(30),
        );
        let pending = runtime.info(id).await.unwrap();
        assert_eq!(pending["map"][id.to_string()]["running"], false);
        assert!(pending["map"][id.to_string()]["my_node_info"].is_null());
        runtime
            .startup_deadlines
            .lock()
            .unwrap()
            .insert(id, std::time::Instant::now());
        assert!(runtime.info(id).await.is_err());
        runtime.shutdown_owned();
        assert!(runtime.startup_deadlines.lock().unwrap().is_empty());
    }
}
fn network_config(
    instance_id: Uuid,
    join: &Value,
    listener: &str,
    local_ipv4: Option<&Value>,
) -> anyhow::Result<NetworkConfig> {
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
    Ok(NetworkConfig {
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
        // Compiled for upstream mobile NIC compatibility, not an opt-in to
        // alter system DNS or run a magic-DNS service for the user's network.
        enable_magic_dns: Some(false),
        ..Default::default()
    })
}
