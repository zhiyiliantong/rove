//! Use only against a dedicated, empty EasyTier process. Never a user's service.
#[cfg(feature = "easytier")]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use rove_agent::overlay::EasyTier;
    use serde_json::json;
    use uuid::Uuid;
    let portal: std::net::SocketAddr = std::env::args()
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("Usage: easytier_probe 127.0.0.1:<isolated-portal>"))?
        .parse()?;
    let client = EasyTier::connect(portal)?;
    anyhow::ensure!(
        client.list_instances().await?.is_empty(),
        "Probe requires an empty dedicated EasyTier process"
    );
    let mut created = Vec::new();
    let result=async {
        for _ in 0..2 {
            let instance=Uuid::new_v4();let network=Uuid::new_v4();
            let join=json!({"schema_version":1,"network_id":network,"display_name":"isolated probe","easytier":{"network_name":format!("rove-probe-{network}"),"network_secret":Uuid::new_v4().to_string(),"bootstrap_peers":[],"dhcp":true}});
            client.probe_start(instance,&join).await?;created.push(instance);
        }
        let listed=client.list_instances().await?;anyhow::ensure!(created.iter().all(|id|listed.contains(id)),"Both instances must exist in the same process");
        for id in &created {
            let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(10);
            loop {
                let info = client.info(*id).await?;
                let state = &info["map"][id.to_string()];
                if state["running"] == true {
                    let peers = state["peers"].as_array().ok_or_else(|| anyhow::anyhow!("Missing peer list"))?;
                    println!("instance {id}: running=true, peers={}, virtual_ipv4={}", peers.len(), state["my_node_info"]["virtual_ipv4"]);
                    break;
                }
                anyhow::ensure!(tokio::time::Instant::now() < deadline, "Instance did not become running before deadline");
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        }
        Ok::<_,anyhow::Error>(())
    }.await;
    for id in created {
        client.stop(id).await?;
    }
    anyhow::ensure!(
        client.list_instances().await?.is_empty(),
        "Probe instances must be removed"
    );
    result?;
    println!(
        "PASS: two instances running, peer lists queried, then stopped through one EasyTier RPC portal; no cross-device connectivity claimed"
    );
    Ok(())
}
#[cfg(not(feature = "easytier"))]
fn main() {
    eprintln!("Build with --features easytier to run this isolated integration probe");
    std::process::exit(1);
}
