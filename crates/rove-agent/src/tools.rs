use crate::Agent;
#[cfg(all(test, unix))]
use rove_protocol::contract::AGENT;
use rove_protocol::{
    ApiError,
    dto::{WireDto, agent::SystemExecInput},
};
use serde_json::{Value, json};
use std::{process::Stdio, sync::Arc, time::Duration};
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    process::Command,
    sync::mpsc,
};
use tokio_util::sync::CancellationToken;

#[cfg(all(test, unix))]
#[path = "tool_tests.rs"]
mod tests;

async fn read_pipe(
    mut pipe: impl AsyncRead + Unpin,
    stream: &'static str,
    tx: mpsc::Sender<(&'static str, String)>,
) -> std::io::Result<()> {
    let mut bytes = [0u8; 4096];
    let mut pending = Vec::new();
    loop {
        let size = pipe.read(&mut bytes).await?;
        pending.extend_from_slice(&bytes[..size]);
        let valid = match std::str::from_utf8(&pending) {
            Ok(_) => pending.len(),
            Err(e) if e.error_len().is_none() && size != 0 => e.valid_up_to(),
            Err(_) => pending.len(),
        };
        if valid > 0 {
            let text = String::from_utf8_lossy(&pending[..valid]).into_owned();
            pending.drain(..valid);
            if tx.send((stream, text)).await.is_err() {
                return Ok(());
            }
        }
        if size == 0 {
            return Ok(());
        }
    }
}
fn result(status: &str, error: Option<ApiError>) -> Value {
    json!({"status":status,"exit_code":null,"stdout":"","stderr":"","output_truncated":false,"residual_processes":[],"error":error})
}
pub(crate) fn supports_system_exec(os: &str) -> bool {
    matches!(os, "linux" | "macos" | "windows")
}

pub async fn system_exec(
    agent: Arc<Agent>,
    run_id: &str,
    call_id: &str,
    input: &Value,
    cancel: CancellationToken,
) -> Value {
    system_exec_on_platform(agent, run_id, call_id, input, cancel, std::env::consts::OS).await
}

// Platform selection is internal, never supplied by an API request. Keeping
// this check on the execution path lets host tests prove denial has no effects.
async fn system_exec_on_platform(
    agent: Arc<Agent>,
    run_id: &str,
    call_id: &str,
    input: &Value,
    cancel: CancellationToken,
    os: &str,
) -> Value {
    let input = match SystemExecInput::from_value(input.clone()) {
        Ok(input) => input,
        Err(error) => return result("failed", Some(error)),
    };
    if !supports_system_exec(os) {
        return result(
            "unsupported",
            Some(ApiError::new(
                501,
                "unsupported",
                "Native system execution is unavailable on this platform",
            )),
        );
    }
    if cancel.is_cancelled() {
        return result("cancelled", None);
    }
    #[cfg(unix)]
    let mut command = {
        let mut c = Command::new("/bin/sh");
        c.arg("-c").arg(&input.command);
        c.process_group(0);
        c
    };
    #[cfg(windows)]
    let mut command = {
        let mut c = Command::new("cmd.exe");
        c.args(["/D", "/S", "/C"]).arg(&input.command);
        c
    };
    let default_dir = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" });
    if let Some(dir) = &input.working_directory {
        command.current_dir(dir);
    } else if let Some(dir) = default_dir {
        command.current_dir(dir);
    }
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(_) => {
            return result(
                "failed",
                Some(ApiError::new(
                    500,
                    "command_spawn_failed",
                    "Could not start command; check OS permissions and working directory",
                )),
            );
        }
    };
    let pid = child.id();
    let (tx, mut rx) = mpsc::channel(32);
    let mut readers = tokio::task::JoinSet::new();
    readers.spawn(read_pipe(
        child.stdout.take().unwrap(),
        "stdout",
        tx.clone(),
    ));
    readers.spawn(read_pipe(child.stderr.take().unwrap(), "stderr", tx));
    let timeout = tokio::time::sleep(Duration::from_secs(
        input
            .timeout_seconds
            .as_ref()
            .and_then(|n| n.as_u64())
            .unwrap_or(300),
    ));
    tokio::pin!(timeout);
    let mut output = result("failed", None);
    let mut completed = false;
    let mut closed = false;
    let mut cancelled = false;
    let mut drain_deadline: Option<tokio::time::Instant> = None;
    while !completed || !closed {
        tokio::select! {
            event=rx.recv(),if !closed=>match event {
                Some((stream,text))=>{
                    if agent.append_event(run_id,"tool_output",json!({"tool_call_id":call_id,"stream":stream,"text":text})).is_err(){output["output_truncated"]=json!(true);}
                    let mut accumulated=output[stream].as_str().unwrap().to_owned();accumulated.push_str(&text);
                    let count=accumulated.chars().count();if count>65536{accumulated=accumulated.chars().skip(count-65536).collect();output["output_truncated"]=json!(true);}
                    output[stream]=json!(accumulated);
                },None=>closed=true,
            },
            status=child.wait(),if !completed=>{
                completed=true;drain_deadline=Some(tokio::time::Instant::now()+Duration::from_secs(2));
                match status {
                    Ok(status)=>{output["exit_code"]=json!(status.code());if !cancelled{output["status"]=json!(if status.success(){"succeeded"}else{"failed"});}},
                    Err(_)=>output["error"]=json!(ApiError::new(500,"command_wait_failed","Could not collect command exit status")),
                }
            },
            _=cancel.cancelled(),if !cancelled=>{
                cancelled=true;output["status"]=json!("cancelled");stop_child(&mut child,pid,&mut output).await;
                drain_deadline=Some(tokio::time::Instant::now()+Duration::from_secs(2));
            },
            _=&mut timeout,if !cancelled=>{
                cancelled=true;output["status"]=json!("timed_out");stop_child(&mut child,pid,&mut output).await;
                drain_deadline=Some(tokio::time::Instant::now()+Duration::from_secs(2));
            },
            _=async {if let Some(deadline)=drain_deadline{tokio::time::sleep_until(deadline).await}else{std::future::pending::<()>().await}}=>{
                output["output_truncated"]=json!(true);break;
            },
        }
    }
    // Dropping pipe read futures must not leave unbounded tasks when a daemon
    // inherited stdout. Abort the actual pipe tasks, not only their joiners.
    readers.abort_all();
    while let Some(joined) = readers.join_next().await {
        if let Ok(Err(_)) = joined {
            output["error"] = json!(ApiError::new(
                500,
                "command_output_failed",
                "Could not read command output"
            ));
            if output["status"] == "succeeded" {
                output["status"] = json!("failed");
            }
        }
    }
    output
}
async fn stop_child(child: &mut tokio::process::Child, pid: Option<u32>, output: &mut Value) {
    #[cfg(unix)]
    if let Some(pid) = pid {
        // SAFETY: the child was started as leader of its own process group.
        let stopped = unsafe { libc::kill(-(pid as i32), libc::SIGKILL) };
        if stopped != 0 && std::io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH) {
            output["residual_processes"] = json!(["Could not signal the command process group"]);
        }
    }
    #[cfg(windows)]
    {
        let _ = pid;
        output["residual_processes"] =
            json!(["Descendant process termination requires Windows Job Object integration"]);
    }
    if child.start_kill().is_err() && !matches!(child.try_wait(), Ok(Some(_))) {
        // A failed termination request must not be hidden behind "cancelled".
        // Ignore the failure only when the child's exit has been confirmed.
        output["residual_processes"]
            .as_array_mut()
            .unwrap()
            .push(json!("Could not terminate the command process"));
    }
}
