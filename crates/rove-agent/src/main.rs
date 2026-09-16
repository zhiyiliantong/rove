use clap::Parser;
use std::path::PathBuf;
use tokio_util::sync::CancellationToken;
#[derive(Parser)]
#[command(version, about = "Rove per-device agent")]
struct Args {
    #[arg(long, env = "ROVE_DATA_DIR")]
    data_dir: Option<PathBuf>,
    /// Dedicated local EasyTier management portal (requires Linux + easytier feature).
    #[arg(long, env = "ROVE_EASYTIER_PORTAL")]
    easytier_portal: Option<std::net::SocketAddr>,
    #[arg(long, default_value = "easytier-core")]
    easytier_core: PathBuf,
    #[arg(long, default_value = "tcp://0.0.0.0:11010")]
    easytier_listen: String,
    #[arg(long, default_value_t = 43190)]
    overlay_port: u16,
}
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let agent = rove_agent::Agent::open(&args.data_dir.unwrap_or_else(rove_sdk::default_data_dir))?;
    if let Some(portal) = args.easytier_portal {
        #[cfg(all(feature = "easytier", target_os = "linux"))]
        agent
            .configure_easytier(
                portal,
                &args.easytier_core,
                args.easytier_listen,
                args.overlay_port,
            )
            .await?;
        #[cfg(not(all(feature = "easytier", target_os = "linux")))]
        {
            let _ = portal;
            anyhow::bail!(
                "EasyTier network runtime requires a Linux build with --features easytier"
            );
        }
    }
    let shutdown = CancellationToken::new();
    let signal = shutdown.clone();
    tokio::spawn(async move {
        #[cfg(unix)]
        {
            if let Ok(mut terminate) =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            {
                tokio::select! {_=tokio::signal::ctrl_c()=>{},_=terminate.recv()=>{}}
            } else {
                let _ = tokio::signal::ctrl_c().await;
            }
        }
        #[cfg(not(unix))]
        let _ = tokio::signal::ctrl_c().await;
        signal.cancel();
    });
    eprintln!(
        "rove-agent {} device_id={}",
        env!("CARGO_PKG_VERSION"),
        agent.store.device_id
    );
    rove_agent::serve(agent, shutdown).await
}
