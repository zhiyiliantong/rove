use clap::Parser;
use std::{net::SocketAddr, path::PathBuf};
#[derive(Parser)]
#[command(version, about = "Anonymous encrypted Rove join-configuration hosting")]
struct Args {
    #[arg(long, default_value = "127.0.0.1:43191")]
    listen: SocketAddr,
    #[arg(long, default_value = "http://127.0.0.1:43191")]
    public_url: url::Url,
    #[arg(long, default_value = "rove-blobs.db")]
    database: PathBuf,
    #[arg(long, default_value_t = 1073741824)]
    max_storage_bytes: u64,
    #[arg(long, default_value_t = 10000)]
    max_blobs: u64,
}
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let store = rove_config_server::BlobStore::open(
        &args.database,
        args.public_url,
        args.max_storage_bytes,
        args.max_blobs,
    )?;
    let cleanup = store.clone();
    let cleaner = tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600));
        loop {
            interval.tick().await;
            if cleanup.clean_expired().is_err() {
                eprintln!("Ciphertext cleanup failed");
            }
        }
    });
    let listener = tokio::net::TcpListener::bind(args.listen).await?;
    eprintln!("rove-config-server listening on {}", listener.local_addr()?);
    let result = axum::serve(listener, rove_config_server::router(store))
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await;
    cleaner.abort();
    result?;
    Ok(())
}
