//! Windows local transport: current OS account only, never remote SMB clients.
use crate::{Agent, socket};
use std::{ffi::c_void, mem::size_of, ptr, sync::Arc};
use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};
use tokio_util::sync::CancellationToken;
use windows_sys::Win32::{
    Foundation::{CloseHandle, HANDLE, LocalFree},
    Security::{
        Authorization::{
            ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW,
        },
        GetTokenInformation, SECURITY_ATTRIBUTES, TOKEN_QUERY, TOKEN_USER, TokenUser,
    },
    System::Threading::{GetCurrentProcess, OpenProcessToken},
};

struct Token(HANDLE);
impl Drop for Token {
    fn drop(&mut self) {
        // SAFETY: owns the successful OpenProcessToken result exactly once.
        unsafe {
            CloseHandle(self.0);
        }
    }
}
struct LocalAllocation(*mut c_void);
impl Drop for LocalAllocation {
    fn drop(&mut self) {
        // SAFETY: owns memory returned by a Windows conversion API using LocalAlloc.
        unsafe {
            LocalFree(self.0);
        }
    }
}

fn current_user_sddl() -> std::io::Result<Vec<u16>> {
    // SAFETY: all out pointers refer to live, correctly sized storage. Token
    // buffer uses usize elements to satisfy TOKEN_USER alignment. API-owned
    // strings remain alive until copied, then RAII releases their allocations.
    unsafe {
        let mut token = ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return Err(std::io::Error::last_os_error());
        }
        let token = Token(token);
        let mut length = 0;
        GetTokenInformation(token.0, TokenUser, ptr::null_mut(), 0, &mut length);
        if length < size_of::<TOKEN_USER>() as u32 {
            return Err(std::io::Error::last_os_error());
        }
        let mut buffer = vec![0usize; (length as usize).div_ceil(size_of::<usize>())];
        if GetTokenInformation(
            token.0,
            TokenUser,
            buffer.as_mut_ptr().cast(),
            length,
            &mut length,
        ) == 0
        {
            return Err(std::io::Error::last_os_error());
        }
        let user = &*buffer.as_ptr().cast::<TOKEN_USER>();
        let mut sid = ptr::null_mut();
        if ConvertSidToStringSidW(user.User.Sid, &mut sid) == 0 {
            return Err(std::io::Error::last_os_error());
        }
        let allocation = LocalAllocation(sid.cast());
        let mut len = 0;
        while *sid.add(len) != 0 {
            len += 1;
        }
        let sid = String::from_utf16_lossy(std::slice::from_raw_parts(sid, len));
        drop(allocation);
        // Protected DACL: no inherited Everyone/Anonymous/other-user access.
        Ok(format!("D:P(A;;GA;;;{sid})")
            .encode_utf16()
            .chain([0])
            .collect())
    }
}

fn create(path: &std::path::Path, sddl: &[u16], first: bool) -> std::io::Result<NamedPipeServer> {
    // SAFETY: sddl is nul-terminated; the descriptor stays alive during the
    // synchronous CreateNamedPipe call, which copies its security information.
    unsafe {
        let mut descriptor = ptr::null_mut();
        if ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl.as_ptr(),
            1,
            &mut descriptor,
            ptr::null_mut(),
        ) == 0
        {
            return Err(std::io::Error::last_os_error());
        }
        let allocation = LocalAllocation(descriptor);
        let mut attributes = SECURITY_ATTRIBUTES {
            nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: descriptor,
            bInheritHandle: 0,
        };
        let result = ServerOptions::new()
            .first_pipe_instance(first)
            .reject_remote_clients(true)
            .create_with_security_attributes_raw(
                path,
                (&mut attributes as *mut SECURITY_ATTRIBUTES).cast(),
            );
        drop(allocation);
        result
    }
}

pub async fn serve(agent: Arc<Agent>, shutdown: CancellationToken) -> anyhow::Result<()> {
    let path = rove_sdk::socket_path(&agent.store.data_dir);
    let sddl = current_user_sddl()?;
    let mut listener = create(&path, &sddl, true)?;
    let mut clients = tokio::task::JoinSet::new();
    let outcome = loop {
        tokio::select! {
            _ = shutdown.cancelled() => break Ok(()),
            connected = listener.connect() => {
                if let Err(error) = connected { break Err(error); }
                // Maintain an available instance before handing off this one.
                let next = match create(&path, &sddl, false) { Ok(next) => next, Err(error) => break Err(error) };
                let stream = std::mem::replace(&mut listener, next);
                let agent = agent.clone();
                clients.spawn(async move { let _ = socket::serve_connection(agent, stream).await; });
            }
            _ = clients.join_next(), if !clients.is_empty() => {},
        }
    };
    drop(listener);
    clients.abort_all();
    while clients.join_next().await.is_some() {}
    agent.shutdown().await;
    outcome.map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rove_protocol::Request;
    use serde_json::json;

    #[tokio::test]
    async fn current_account_clients_share_state_and_pipe_name_cannot_be_taken_over() {
        let temp = tempfile::tempdir().unwrap();
        let agent = Agent::open(&temp.path().join("agent")).unwrap();
        let path = rove_sdk::socket_path(&agent.store.data_dir);
        let stop = CancellationToken::new();
        let server = tokio::spawn(serve(agent.clone(), stop.clone()));
        let client = rove_sdk::LocalClient::new(&path);
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                if client.call(Request::new("get_device")).await.is_ok() {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        assert!(create(&path, &current_user_sddl().unwrap(), true).is_err());
        let second = rove_sdk::LocalClient::new(&path);
        let (a, b) = tokio::join!(
            client.call(Request::new("get_device")),
            second.call(Request::new("get_device"))
        );
        assert_eq!(
            a.unwrap().body.unwrap()["device_id"],
            b.unwrap().body.unwrap()["device_id"]
        );
        client
            .call(Request::new("update_settings").with_body(json!({"max_active_runs":3})))
            .await
            .unwrap();
        assert_eq!(
            second
                .call(Request::new("get_settings"))
                .await
                .unwrap()
                .body
                .unwrap()["max_active_runs"],
            3
        );
        stop.cancel();
        server.await.unwrap().unwrap();
        assert!(second.call(Request::new("get_device")).await.is_err());
        // No server handle leaks: a fresh first instance can claim the name.
        let fresh = create(&path, &current_user_sddl().unwrap(), true).unwrap();
        drop(fresh);
    }
}
