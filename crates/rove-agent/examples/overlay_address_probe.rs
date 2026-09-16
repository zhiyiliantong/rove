//! Fault injection for scripts/check-overlay.py only, never an application API.
//! Replaces the sole instance in a disposable container with a different address
//! so Rove must observe upstream state and rebuild its own listeners/discovery.
#[cfg(all(feature = "easytier", target_os = "linux"))]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use easytier::{
        proto::{
            api::manage::{
                GetNetworkInstanceConfigRequest, ListNetworkInstanceRequest,
                RunNetworkInstanceRequest, WebClientServiceClientFactory,
            },
            rpc_impl::standalone::StandAloneClient,
            rpc_types::controller::BaseController,
        },
        tunnel::tcp::TcpTunnelConnector,
    };
    anyhow::ensure!(
        std::env::var("ROVE_DISPOSABLE_OVERLAY_TEST").as_deref() == Ok("1"),
        "Only enabled inside the disposable overlay test"
    );
    let address: std::net::Ipv4Addr = std::env::args()
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("Missing test address"))?
        .parse()?;
    anyhow::ensure!(address.is_private(), "Expected a private test address");
    let mut transport =
        StandAloneClient::new(TcpTunnelConnector::new("tcp://127.0.0.1:15888".parse()?));
    let client = transport
        .scoped_client::<WebClientServiceClientFactory<BaseController>>(String::new())
        .await?;
    let instances = client
        .list_network_instance(BaseController::default(), ListNetworkInstanceRequest {})
        .await?;
    anyhow::ensure!(
        instances.inst_ids.len() == 1,
        "Expected exactly one test instance"
    );
    let id = instances.inst_ids[0];
    let response = client
        .get_network_instance_config(
            BaseController::default(),
            GetNetworkInstanceConfigRequest { inst_id: Some(id) },
        )
        .await?;
    let mut config = response
        .config
        .ok_or_else(|| anyhow::anyhow!("Missing test config"))?;
    anyhow::ensure!(
        config
            .dev_name
            .as_deref()
            .is_some_and(|name| name.starts_with("rove")),
        "Expected a Rove-owned test interface"
    );
    config.dhcp = Some(false);
    config.virtual_ipv4 = Some(address.to_string());
    config.network_length = Some(24);
    client
        .run_network_instance(
            BaseController::default(),
            RunNetworkInstanceRequest {
                inst_id: Some(id),
                config: Some(config),
                overwrite: true,
                source: 1,
            },
        )
        .await?;
    println!("Test-only upstream address replacement accepted");
    Ok(())
}

#[cfg(not(all(feature = "easytier", target_os = "linux")))]
fn main() {
    panic!("Requires Linux and --features easytier");
}
