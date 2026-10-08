//! Narrow network runtime process, never a privileged Rove business agent.
#[cfg(all(windows, feature = "easytier-native"))]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use clap::Parser;
    #[derive(Parser)]
    struct Args {
        #[arg(long)]
        pipe: std::path::PathBuf,
        /// Expected local agent PID; zero allows one current-account client
        /// when launched by an authorized headless supervisor. The owner-only
        /// DACL and client-side helper PID verification still apply.
        #[arg(long, default_value_t = 0)]
        agent_pid: u32,
    }
    let args = Args::parse();
    rove_agent::overlay::windows_helper::serve(&args.pipe, args.agent_pid).await
}

#[cfg(not(all(windows, feature = "easytier-native")))]
fn main() -> anyhow::Result<()> {
    anyhow::bail!("The network helper requires Windows and easytier-native")
}
