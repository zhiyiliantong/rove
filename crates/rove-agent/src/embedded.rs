//! App-owned runtime using the exact same agent and local socket protocol.
use std::{path::Path, time::Duration};
use tokio_util::sync::CancellationToken;

pub struct EmbeddedAgent {
    stopping: CancellationToken,
    task: tokio::task::JoinHandle<anyhow::Result<()>>,
}

impl EmbeddedAgent {
    pub async fn start(directory: &Path) -> anyhow::Result<Self> {
        let agent = crate::Agent::open(directory)?;
        let socket = rove_sdk::socket_path(&agent.store.data_dir);
        let stopping = CancellationToken::new();
        let task = tokio::spawn(crate::serve(agent, stopping.clone()));
        let runtime = Self { stopping, task };
        let ready = tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if runtime.task.is_finished() {
                    anyhow::bail!("Embedded agent stopped before its socket became ready");
                }
                if tokio::net::UnixStream::connect(&socket).await.is_ok() {
                    return Ok(());
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await;
        match ready {
            Ok(Ok(())) => Ok(runtime),
            Ok(Err(error)) => {
                runtime.stop().await?;
                Err(error)
            }
            Err(error) => {
                runtime.stop().await?;
                Err(error.into())
            }
        }
    }

    pub async fn stop(mut self) -> anyhow::Result<()> {
        self.stopping.cancel();
        (&mut self.task).await??;
        Ok(())
    }
}

impl Drop for EmbeddedAgent {
    fn drop(&mut self) {
        // A window disappearing is not a per-run cancellation API. This guard
        // belongs to the entire mobile app, whose OS lifetime remains limited.
        self.stopping.cancel();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn app_runtime_uses_sdk_socket_and_retains_identity_after_restart() {
        let temp = tempfile::tempdir().unwrap();
        let directory = temp.path().join("app");
        let first = EmbeddedAgent::start(&directory).await.unwrap();
        let client = rove_sdk::LocalClient::new(rove_sdk::socket_path(&directory));
        let request = rove_protocol::Request::new("get_device");
        let before = client.call(request.clone()).await.unwrap().body.unwrap();
        assert!(EmbeddedAgent::start(&directory).await.is_err());
        first.stop().await.unwrap();
        let second = EmbeddedAgent::start(&directory).await.unwrap();
        let after = client.call(request).await.unwrap().body.unwrap();
        assert_eq!(before["device_id"], after["device_id"]);
        second.stop().await.unwrap();
    }
}
