//! Windows-only process boundary. The helper exposes bounded EasyTier instance
//! operations, not the Rove business API, shell commands, files or arbitrary
//! upstream configuration. One verified local agent owns its whole lifetime.
use super::EasyTier;
use anyhow::Context;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{os::windows::io::AsRawHandle, path::Path, time::Duration};
use tokio::net::windows::named_pipe::{ClientOptions, NamedPipeClient};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    sync::Mutex,
};
use uuid::Uuid;
use windows_sys::Win32::System::Pipes::{GetNamedPipeClientProcessId, GetNamedPipeServerProcessId};

const MAX_FRAME: usize = 1024 * 1024;

#[derive(Serialize, Deserialize)]
#[serde(tag = "operation", deny_unknown_fields)]
pub(super) enum Request {
    List,
    Start {
        instance_id: Uuid,
        join: Value,
        listener: String,
        local_ipv4: Option<Value>,
    },
    Info {
        instance_id: Uuid,
    },
    Stop {
        instance_id: Uuid,
    },
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "status", deny_unknown_fields)]
enum Response {
    Success {
        value: Value,
    },
    Failure {
        phase: String,
        os_error: Option<i32>,
    },
}

fn diagnostic_phase(value: &str) -> Option<&'static str> {
    match value {
        "native_interface" => Some("native_interface"),
        "native_routes" => Some("native_routes"),
        "native_isolation" => Some("native_isolation"),
        "native_firewall" => Some("native_firewall"),
        "native_helper_start" => Some("native_helper_start"),
        "native_helper_state" => Some("native_helper_state"),
        _ => None,
    }
}

fn validate_path(path: &Path) -> anyhow::Result<()> {
    let raw = path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid helper pipe path"))?;
    let id = raw
        .strip_prefix(r"\\.\pipe\rove-network-")
        .ok_or_else(|| anyhow::anyhow!("Invalid helper pipe namespace"))?;
    let parsed = Uuid::parse_str(id)?;
    anyhow::ensure!(
        !parsed.is_nil() && id == parsed.to_string(),
        "Invalid helper pipe identity"
    );
    Ok(())
}

async fn read_frame(stream: &mut (impl AsyncRead + Unpin)) -> anyhow::Result<Vec<u8>> {
    let size = stream.read_u32().await? as usize;
    anyhow::ensure!(size > 0 && size <= MAX_FRAME, "Invalid helper frame length");
    let mut data = vec![0; size];
    stream.read_exact(&mut data).await?;
    Ok(data)
}

async fn write_frame(
    stream: &mut (impl AsyncWrite + Unpin),
    value: &impl Serialize,
) -> anyhow::Result<()> {
    let data = serde_json::to_vec(value)?;
    anyhow::ensure!(
        !data.is_empty() && data.len() <= MAX_FRAME,
        "Helper response exceeds limit"
    );
    stream.write_u32(data.len() as u32).await?;
    stream.write_all(&data).await?;
    stream.flush().await?;
    Ok(())
}

pub(super) struct Client {
    stream: Mutex<Option<NamedPipeClient>>,
    stopping: tokio_util::sync::CancellationToken,
    session: Uuid,
    server_pid: std::sync::atomic::AtomicU32,
}
impl Drop for Client {
    fn drop(&mut self) {
        self.shutdown();
    }
}
impl Client {
    pub(super) fn unstarted() -> Self {
        Self {
            stream: Mutex::new(None),
            stopping: tokio_util::sync::CancellationToken::new(),
            session: Uuid::new_v4(),
            server_pid: std::sync::atomic::AtomicU32::new(0),
        }
    }

    pub(super) async fn prepare(&self) -> anyhow::Result<()> {
        let mut slot = self.stream.lock().await;
        anyhow::ensure!(
            !self.stopping.is_cancelled(),
            "Network helper is shutting down"
        );
        if slot.is_some() {
            if !crate::overlay_platform::helper_process_exited(
                self.server_pid.load(std::sync::atomic::Ordering::Relaxed),
            )? {
                return Ok(());
            }
            // An idle or terminated helper must be reauthorized before intent
            // is accepted. Never reuse a stale pipe or replay an uncertain call.
            slot.take();
            crate::overlay_platform::clear_helper_isolation(self.session);
        }
        let old_pid = self.server_pid.load(std::sync::atomic::Ordering::Relaxed);
        tokio::time::timeout(Duration::from_secs(6), async {
            while !crate::overlay_platform::helper_process_exited(old_pid)? {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            Ok::<(), anyhow::Error>(())
        })
        .await??;
        let path = std::path::PathBuf::from(format!(r"\\.\pipe\rove-network-{}", Uuid::new_v4()));
        let launch_path = path.clone();
        let pid = tokio::task::spawn_blocking(move || launch(&launch_path)).await??;
        let stream = tokio::time::timeout(Duration::from_secs(15), async {
            loop {
                match ClientOptions::new().open(&path) {
                    Ok(stream) => {
                        let mut actual = 0;
                        let ok = unsafe {
                            GetNamedPipeServerProcessId(stream.as_raw_handle(), &mut actual)
                        };
                        anyhow::ensure!(ok != 0 && actual == pid, "Unexpected helper process");
                        break Ok::<_, anyhow::Error>(stream);
                    }
                    Err(error) if matches!(error.raw_os_error(), Some(2 | 231)) => {
                        tokio::time::sleep(Duration::from_millis(100)).await
                    }
                    Err(error) => break Err(error.into()),
                }
            }
        })
        .await??;
        self.server_pid
            .store(pid, std::sync::atomic::Ordering::Relaxed);
        *slot = Some(stream);
        Ok(())
    }
    pub(super) async fn connect(path: &Path, server_pid: u32) -> anyhow::Result<Self> {
        validate_path(path)?;
        anyhow::ensure!(server_pid != 0, "Missing expected helper process");
        let stream = ClientOptions::new().open(path)?;
        let mut actual = 0;
        // SAFETY: the live pipe handle and writable PID are valid for this call.
        let ok = unsafe { GetNamedPipeServerProcessId(stream.as_raw_handle(), &mut actual) };
        anyhow::ensure!(
            ok != 0 && actual == server_pid,
            "Unexpected helper pipe server"
        );
        Ok(Self {
            stream: Mutex::new(Some(stream)),
            stopping: tokio_util::sync::CancellationToken::new(),
            session: Uuid::new_v4(),
            server_pid: std::sync::atomic::AtomicU32::new(server_pid),
        })
    }

    pub(super) fn shutdown(&self) {
        crate::overlay_platform::clear_helper_isolation(self.session);
        self.stopping.cancel();
        if let Ok(mut stream) = self.stream.try_lock() {
            stream.take();
        }
    }

    pub(super) async fn call(&self, request: Request) -> anyhow::Result<Value> {
        let mut slot = self.stream.lock().await;
        if slot.is_none() {
            anyhow::ensure!(
                crate::overlay_platform::helper_process_exited(
                    self.server_pid.load(std::sync::atomic::Ordering::Relaxed)
                )?,
                "Disconnected helper exit has not been confirmed"
            );
            return match request {
                Request::List => Ok(json!([])),
                Request::Stop { .. } => Ok(Value::Null),
                _ => anyhow::bail!("Network helper has not been authorized"),
            };
        }
        let stream = slot
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Network helper is disconnected"))?;
        let result = tokio::time::timeout(Duration::from_secs(6), async {
            tokio::select! {
                biased;
                _ = self.stopping.cancelled() => anyhow::bail!("Network helper stopped"),
                result = async {
                write_frame(stream, &request).await?;
                let reply: Response = serde_json::from_slice(&read_frame(stream).await?)?;
            Ok::<Response, anyhow::Error>(reply)
                } => result,
            }
        })
        .await;
        match result {
            Ok(Ok(Response::Success { value })) => {
                match &request {
                    Request::Info { instance_id } => {
                        let proof = &value["rove_isolation"];
                        let verified = proof["index"]
                            .as_u64()
                            .and_then(|v| u32::try_from(v).ok())
                            .zip(proof["address"].as_str().and_then(|s| s.parse().ok()))
                            .map(|(index, address)| {
                                (
                                    index,
                                    address,
                                    self.server_pid.load(std::sync::atomic::Ordering::Relaxed),
                                )
                            });
                        crate::overlay_platform::set_helper_isolation(
                            self.session,
                            *instance_id,
                            verified,
                        );
                    }
                    Request::Stop { instance_id } => crate::overlay_platform::set_helper_isolation(
                        self.session,
                        *instance_id,
                        None,
                    ),
                    _ => {}
                }
                Ok(value)
            }
            Ok(Ok(Response::Failure { phase, os_error })) => {
                let error = os_error
                    .map(|code| anyhow::Error::from(std::io::Error::from_raw_os_error(code)))
                    .unwrap_or_else(|| anyhow::anyhow!("Network helper operation failed"));
                Err(error.context(diagnostic_phase(&phase).unwrap_or("native_helper_state")))
            }
            _ => {
                // Never replay an operation with uncertain completion. Closing
                // the session also tells the helper to destroy owned instances.
                slot.take();
                crate::overlay_platform::clear_helper_isolation(self.session);
                anyhow::bail!("Network helper disconnected or operation failed")
            }
        }
    }
}

fn launch(pipe: &Path) -> anyhow::Result<u32> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::{
        Foundation::CloseHandle,
        System::Threading::GetProcessId,
        UI::{
            Shell::{
                SEE_MASK_NOASYNC, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW, ShellExecuteExW,
            },
            WindowsAndMessaging::SW_HIDE,
        },
    };
    validate_path(pipe)?;
    let program = std::env::current_exe()?.with_file_name("rove-network-helper.exe");
    anyhow::ensure!(program.is_file(), "Bundled network helper is missing");
    let file: Vec<u16> = program.as_os_str().encode_wide().chain(Some(0)).collect();
    let verb: Vec<u16> = "runas".encode_utf16().chain(Some(0)).collect();
    // Both parameters are internally generated UUID/PID values, never user
    // paths, shell commands or network secrets. ShellExecute performs normal
    // system elevation; it does not alter token privileges of the agent.
    let parameters: Vec<u16> = format!(
        "--pipe {} --agent-pid {}",
        pipe.display(),
        std::process::id()
    )
    .encode_utf16()
    .chain(Some(0))
    .collect();
    let mut info = SHELLEXECUTEINFOW::default();
    info.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
    info.fMask = SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC;
    info.lpVerb = verb.as_ptr();
    info.lpFile = file.as_ptr();
    info.lpParameters = parameters.as_ptr();
    info.nShow = SW_HIDE;
    unsafe {
        if ShellExecuteExW(&mut info) == 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        anyhow::ensure!(!info.hProcess.is_null(), "Missing helper process handle");
        let pid = GetProcessId(info.hProcess);
        CloseHandle(info.hProcess);
        anyhow::ensure!(pid != 0, "Invalid helper process handle");
        Ok(pid)
    }
}

/// Elevated helper entry point. Only the current account and expected local
/// agent PID may connect; SMB clients and additional sessions are rejected.
/// Caller launches this separate executable, never elevates the business agent.
pub async fn serve(path: &Path, agent_pid: u32) -> anyhow::Result<()> {
    validate_path(path)?;
    anyhow::ensure!(agent_pid != std::process::id(), "Invalid owner process");
    let mut sddl = crate::windows_pipe::current_user_sddl()?;
    // UAC elevates the helper, not the agent. Explicit medium integrity permits
    // that same account's ordinary agent to write; the protected DACL and PID
    // checks still exclude other accounts, remote clients and other processes.
    sddl.pop();
    sddl.extend("S:(ML;;NW;;;ME)".encode_utf16().chain([0]));
    let _driver = DriverLibrary::load()?;
    let mut pipe = crate::windows_pipe::create(path, &sddl, true)?;
    tokio::time::timeout(Duration::from_secs(60), pipe.connect()).await??;
    let mut actual = 0;
    // SAFETY: connected local pipe and writable process ID are live.
    let ok = unsafe { GetNamedPipeClientProcessId(pipe.as_raw_handle(), &mut actual) };
    anyhow::ensure!(
        ok != 0 && actual != 0 && (agent_pid == 0 || actual == agent_pid),
        "Unexpected helper client process"
    );
    let runtime = EasyTier::embedded();
    let agent_image = crate::overlay_platform::verified_agent_image(actual).ok();
    let mut isolation = std::collections::BTreeMap::new();
    let result = async {
        loop {
            let raw =
                tokio::time::timeout(Duration::from_secs(30), read_frame(&mut pipe)).await??;
            let request: Request = serde_json::from_slice(&raw)?;
            let result = execute(&runtime, &mut isolation, agent_image.as_deref(), request).await;
            // Never return raw upstream errors, config, paths or credentials.
            let response = match result {
                Ok(value) => Response::Success { value },
                Err(error) => {
                    let phase = error
                        .chain()
                        .find_map(|cause| diagnostic_phase(&cause.to_string()))
                        .unwrap_or("native_helper_state");
                    let os_error = error
                        .downcast_ref::<std::io::Error>()
                        .and_then(std::io::Error::raw_os_error);
                    eprintln!("network helper: stage={phase}, os_error={os_error:?}");
                    Response::Failure {
                        phase: phase.into(),
                        os_error,
                    }
                }
            };
            tokio::time::timeout(Duration::from_secs(6), write_frame(&mut pipe, &response))
                .await??;
        }
        #[allow(unreachable_code)]
        Ok::<(), anyhow::Error>(())
    }
    .await;
    runtime.shutdown_owned();
    drop(isolation);
    result
}

type Isolation = std::collections::BTreeMap<
    Uuid,
    (
        u32,
        std::net::Ipv4Addr,
        ipnet::Ipv4Net,
        crate::overlay_platform::IsolationGuard,
        crate::overlay_platform::FirewallGuard,
    ),
>;
async fn execute(
    runtime: &EasyTier,
    isolation: &mut Isolation,
    agent_image: Option<&Path>,
    request: Request,
) -> anyhow::Result<Value> {
    match request {
        Request::List => Ok(json!(runtime.list_instances().await?)),
        Request::Start {
            instance_id,
            join,
            listener,
            local_ipv4,
        } => {
            anyhow::ensure!(agent_image.is_some(), "native_firewall");
            anyhow::ensure!(
                !instance_id.is_nil() && runtime.list_instances().await?.len() < 64,
                "Instance limit"
            );
            let url: url::Url = listener.parse()?;
            anyhow::ensure!(
                url.host_str() == Some("0.0.0.0"),
                "Helper listener must be local wildcard"
            );
            if join["easytier"]["dhcp"] == false {
                crate::sharing::validate_join(&join)?;
                crate::overlay_platform::check_requested_subnet(
                    join["easytier"]["ipv4_cidr"].as_str().unwrap().parse()?,
                )
                .context("native_routes")?;
            }
            runtime
                .start_with_address(instance_id, &join, &listener, local_ipv4.as_ref())
                .await
                .context("native_helper_start")?;
            Ok(Value::Null)
        }
        Request::Info { instance_id } => {
            let mut info = runtime
                .info(instance_id)
                .await
                .context("native_helper_state")?;
            let node = &info["map"][instance_id.to_string()];
            if node["running"] != true || node["my_node_info"]["virtual_ipv4"].is_null() {
                return Ok(info);
            }
            let virtual_ip = &node["my_node_info"]["virtual_ipv4"];
            let raw = virtual_ip["address"]["addr"]
                .as_u64()
                .and_then(|v| u32::try_from(v).ok())
                .ok_or_else(|| anyhow::anyhow!("Missing native address"))?;
            let address = std::net::Ipv4Addr::from(raw);
            let prefix = virtual_ip["network_length"]
                .as_u64()
                .and_then(|v| u8::try_from(v).ok())
                .ok_or_else(|| anyhow::anyhow!("Missing native prefix"))?;
            let subnet = ipnet::Ipv4Net::new(address, prefix)?.trunc();
            let expected = format!("rove{}", &instance_id.simple().to_string()[..8]);
            anyhow::ensure!(
                node["dev_name"].as_str() == Some(expected.as_str()),
                "native_interface"
            );
            let index = tokio::time::timeout(Duration::from_secs(3), async {
                loop {
                    if let Ok(index) =
                        crate::overlay_platform::validate_overlay_facts(&expected, address)
                    {
                        break index;
                    }
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            })
            .await
            .context("native_interface")?;
            if !isolation
                .get(&instance_id)
                .is_some_and(|(i, a, s, _, _)| *i == index && *a == address && *s == subnet)
            {
                crate::overlay_platform::check_overlay_subnet(&expected, subnet)
                    .context("native_routes")?;
                let guard =
                    crate::overlay_platform::IsolationGuard::install(index, address, subnet)
                        .context("native_isolation")?;
                let firewall = crate::overlay_platform::FirewallGuard::install(
                    agent_image.ok_or_else(|| anyhow::anyhow!("native_firewall"))?,
                    address,
                    subnet,
                )
                .context("native_firewall")?;
                isolation.insert(instance_id, (index, address, subnet, guard, firewall));
            }
            info["rove_isolation"] = json!({"index":index,"address":address});
            Ok(info)
        }
        Request::Stop { instance_id } => {
            runtime.stop(instance_id).await?;
            isolation.remove(&instance_id);
            Ok(Value::Null)
        }
    }
}

struct DriverLibrary {
    // Keep the verified file open without write/delete sharing for the entire
    // session. Upstream itself verifies the Authenticode signer as well.
    _file: std::fs::File,
    _packet_driver: std::fs::File,
    module: windows_sys::Win32::Foundation::HMODULE,
}
impl DriverLibrary {
    fn load() -> anyhow::Result<Self> {
        use sha2::{Digest, Sha256};
        use std::{
            io::Read,
            os::windows::{ffi::OsStrExt, fs::OpenOptionsExt},
        };
        use windows_sys::Win32::System::LibraryLoader::{
            LOAD_LIBRARY_SEARCH_APPLICATION_DIR, LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR,
            LOAD_LIBRARY_SEARCH_SYSTEM32, LoadLibraryExW, SetDefaultDllDirectories,
        };
        let path = std::env::current_exe()?.with_file_name("wintun.dll");
        let mut file = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(1)
            .open(&path)?;
        anyhow::ensure!(
            file.metadata()?.len() == 427552,
            "Unexpected bundled driver size"
        );
        let mut bytes = Vec::with_capacity(427552);
        (&mut file).take(427553).read_to_end(&mut bytes)?;
        anyhow::ensure!(
            format!("{:x}", Sha256::digest(&bytes))
                == "e5da8447dc2c320edc0fc52fa01885c103de8c118481f683643cacc3220dafce",
            "Bundled driver checksum mismatch"
        );
        let mut packet_driver = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(1)
            .open(path.with_file_name("WinDivert64.sys"))?;
        anyhow::ensure!(
            packet_driver.metadata()?.len() == 94144,
            "Unexpected packet driver size"
        );
        let mut packet_bytes = Vec::with_capacity(94144);
        (&mut packet_driver)
            .take(94145)
            .read_to_end(&mut packet_bytes)?;
        anyhow::ensure!(
            format!("{:x}", Sha256::digest(&packet_bytes))
                == "8da085332782708d8767bcace5327a6ec7283c17cfb85e40b03cd2323a90ddc2",
            "Bundled packet driver checksum mismatch"
        );
        let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        unsafe {
            anyhow::ensure!(
                SetDefaultDllDirectories(
                    LOAD_LIBRARY_SEARCH_APPLICATION_DIR | LOAD_LIBRARY_SEARCH_SYSTEM32
                ) != 0,
                "Cannot restrict helper DLL search path"
            );
            let module = LoadLibraryExW(
                wide.as_ptr(),
                std::ptr::null_mut(),
                LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_SYSTEM32,
            );
            if module.is_null() {
                return Err(std::io::Error::last_os_error().into());
            }
            Ok(Self {
                _file: file,
                _packet_driver: packet_driver,
                module,
            })
        }
    }
}
impl Drop for DriverLibrary {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::Foundation::FreeLibrary(self.module);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn disconnected_live_helper_is_not_reported_as_empty_or_stopped() {
        let client = Client::unstarted();
        assert_eq!(client.call(Request::List).await.unwrap(), json!([]));
        client
            .server_pid
            .store(std::process::id(), std::sync::atomic::Ordering::Relaxed);
        assert!(client.call(Request::List).await.is_err());
        assert!(
            client
                .call(Request::Stop {
                    instance_id: Uuid::new_v4()
                })
                .await
                .is_err()
        );
    }
    #[test]
    fn helper_protocol_rejects_general_execution_and_foreign_pipe_names() {
        let success = serde_json::to_vec(&Response::Success { value: Value::Null }).unwrap();
        assert!(matches!(
            serde_json::from_slice::<Response>(&success).unwrap(),
            Response::Success { value: Value::Null }
        ));
        assert!(
            serde_json::from_value::<Request>(json!({"operation":"Execute","command":"whoami"}))
                .is_err()
        );
        assert!(
            serde_json::from_value::<Request>(json!({"operation":"List","command":"whoami"}))
                .is_err()
        );
        assert!(validate_path(Path::new(r"\\other-host\pipe\rove-network-test")).is_err());
        assert!(
            validate_path(Path::new(
                r"\\.\pipe\rove-network-00000000-0000-0000-0000-000000000000"
            ))
            .is_err()
        );
        assert!(
            validate_path(Path::new(&format!(
                r"\\.\pipe\rove-network-{}",
                Uuid::new_v4()
            )))
            .is_ok()
        );
    }
}
