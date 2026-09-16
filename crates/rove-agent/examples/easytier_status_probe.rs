//! Read-only diagnostic; deliberately omit configuration, event text and secrets.
#[cfg(feature = "easytier")]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let portal = std::env::args()
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("Expected loopback portal"))?
        .parse()?;
    let client = rove_agent::overlay::EasyTier::connect(portal)?;
    for id in client.list_instances().await? {
        let info = client.info(id).await?;
        let record = &info["map"][id.to_string()];
        println!(
            "{}",
            serde_json::json!({"instance_id":id,"running":record["running"],"dev_name":record["dev_name"],"version":record["my_node_info"]["version"],"ipv4":record["my_node_info"]["virtual_ipv4"],"has_error":record["error_msg"].as_str().is_some_and(|s|!s.is_empty())})
        );
    }
    Ok(())
}
#[cfg(not(feature = "easytier"))]
fn main() {
    std::process::exit(1);
}
