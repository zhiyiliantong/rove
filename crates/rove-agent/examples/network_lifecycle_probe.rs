//! Real management-plane acceptance against a dedicated empty loopback portal.
//! No external bootstrap peers, TUN traffic or cross-device claim.
#[cfg(all(feature = "easytier", target_os = "linux"))]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use rove_agent::{Agent, overlay::EasyTier};
    use rove_protocol::Request;
    use serde_json::{Value, json};
    use std::{sync::Arc, time::Duration};
    async fn until(agent: &Arc<Agent>, id: &str, state: &str) -> anyhow::Result<Value> {
        tokio::time::timeout(Duration::from_secs(15), async {
            loop {
                let record = agent
                    .handle(&Request::new("get_network").with_path("network_id", id))
                    .await
                    .body
                    .unwrap();
                if record["state"] == state {
                    return record;
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        .map_err(Into::into)
    }
    let portal = std::env::args()
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("Expected dedicated loopback portal"))?
        .parse()?;
    let client = EasyTier::connect(portal)?;
    anyhow::ensure!(
        client.list_instances().await?.is_empty(),
        "Dedicated portal must be empty"
    );
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("agent");
    let agent = Agent::open(&path)?;
    agent
        .configure_easytier(
            portal,
            std::path::Path::new("easytier-core"),
            "tcp://127.0.0.1:0".into(),
            43190,
        )
        .await?;
    let mut ids = Vec::new();
    for name in ["probe-home", "probe-friends"] {
        let created = agent
            .handle(&Request::new("create_network").with_body(json!({"display_name":name})))
            .await;
        anyhow::ensure!(created.status_code == 201, "Create failed");
        ids.push(
            created.body.unwrap()["network_id"]
                .as_str()
                .unwrap()
                .to_owned(),
        );
    }
    let first = Request::new("start_network").with_path("network_id", &ids[0]);
    let second = Request::new("start_network").with_path("network_id", &ids[1]);
    let (a, b) = tokio::join!(agent.handle(&first), agent.handle(&second));
    anyhow::ensure!(
        [a.status_code, b.status_code].contains(&202)
            && [a.status_code, b.status_code].contains(&409),
        "Concurrent starts must accept exactly one network"
    );
    let winner = if a.status_code == 202 { 0 } else { 1 };
    let loser = 1 - winner;
    let instance: uuid::Uuid = until(&agent, &ids[winner], "starting").await?["instance_id"]
        .as_str()
        .unwrap()
        .parse()?;
    tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            if client.list_instances().await.unwrap() == vec![instance] {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await?;
    let stopped = agent
        .handle(&Request::new("stop_network").with_path("network_id", &ids[winner]))
        .await;
    anyhow::ensure!(
        stopped.status_code == 202,
        "Stop must be accepted asynchronously"
    );
    until(&agent, &ids[winner], "stopped").await?;
    anyhow::ensure!(
        client.list_instances().await?.is_empty(),
        "Exit must be confirmed in EasyTier"
    );
    let next = agent
        .handle(&Request::new("start_network").with_path("network_id", &ids[loser]))
        .await;
    anyhow::ensure!(
        next.status_code == 202,
        "Second network must be startable after confirmed exit"
    );
    agent
        .handle(&Request::new("stop_network").with_path("network_id", &ids[loser]))
        .await;
    until(&agent, &ids[loser], "stopped").await?;
    anyhow::ensure!(
        client.list_instances().await?.is_empty(),
        "No probe instance may remain"
    );
    agent.shutdown().await;
    println!(
        "PASS: real agent intent -> EasyTier RPC create/delete; concurrent starts isolated; saved configurations retained. No overlay connectivity claimed."
    );
    Ok(())
}
#[cfg(not(all(feature = "easytier", target_os = "linux")))]
fn main() {
    eprintln!("Requires Linux with --features easytier");
    std::process::exit(1);
}
