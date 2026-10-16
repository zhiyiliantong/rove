//! Windows application firewall authorization, confined to the verified agent
//! executable and current overlay addresses. Existing block rules remain final.
use ::windows::{
    Win32::{
        Foundation::VARIANT_TRUE,
        NetworkManagement::WindowsFirewall::*,
        System::Com::{
            CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx,
            CoUninitialize,
        },
    },
    core::BSTR,
};
use std::{
    io,
    net::Ipv4Addr,
    path::{Path, PathBuf},
};

struct Com;
impl Com {
    fn initialize() -> anyhow::Result<Self> {
        unsafe {
            CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
        }
        Ok(Self)
    }
}
impl Drop for Com {
    fn drop(&mut self) {
        unsafe {
            CoUninitialize();
        }
    }
}

/// Process image is obtained from the authenticated pipe peer, never a request
/// field. An administrative helper cannot be used to authorize arbitrary apps.
pub(crate) fn verified_agent_image(pid: u32) -> anyhow::Result<PathBuf> {
    use windows_sys::Win32::{
        Foundation::CloseHandle,
        System::Threading::{
            OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
        },
    };
    let mut buffer = vec![0u16; 32768];
    let mut length = buffer.len() as u32;
    unsafe {
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if process.is_null() {
            return Err(io::Error::last_os_error().into());
        }
        let ok = QueryFullProcessImageNameW(process, 0, buffer.as_mut_ptr(), &mut length);
        let error = io::Error::last_os_error();
        CloseHandle(process);
        if ok == 0 {
            return Err(error.into());
        }
    }
    let actual = PathBuf::from(String::from_utf16(&buffer[..length as usize])?);
    let expected = std::env::current_exe()?.with_file_name("rove-agent.exe");
    anyhow::ensure!(
        actual
            .as_os_str()
            .to_string_lossy()
            .eq_ignore_ascii_case(&expected.as_os_str().to_string_lossy()),
        "Unexpected network helper client image"
    );
    Ok(actual)
}

pub(crate) struct FirewallGuard {
    name: BSTR,
}
impl FirewallGuard {
    pub(crate) fn install(
        agent: &Path,
        address: Ipv4Addr,
        subnet: ipnet::Ipv4Net,
    ) -> anyhow::Result<Self> {
        anyhow::ensure!(
            (1..=30).contains(&subnet.prefix_len()) && subnet.contains(&address),
            "Invalid firewall overlay scope"
        );
        let _com = Com::initialize()?;
        let name = BSTR::from(format!("Rove overlay {}", uuid::Uuid::new_v4()));
        unsafe {
            let policy: INetFwPolicy2 =
                CoCreateInstance(&NetFwPolicy2, None, CLSCTX_INPROC_SERVER)?;
            let rule: INetFwRule = CoCreateInstance(&NetFwRule, None, CLSCTX_INPROC_SERVER)?;
            rule.SetName(&name)?;
            rule.SetGrouping(&BSTR::from("Rove overlay runtime"))?;
            rule.SetDescription(&BSTR::from("Rove agent on its verified overlay address only; existing block policies are not overridden"))?;
            rule.SetApplicationName(&BSTR::from(agent.to_string_lossy().as_ref()))?;
            rule.SetProtocol(6)?;
            rule.SetLocalAddresses(&BSTR::from(address.to_string()))?;
            rule.SetRemoteAddresses(&BSTR::from(subnet.to_string()))?;
            rule.SetDirection(NET_FW_RULE_DIR_IN)?;
            rule.SetAction(NET_FW_ACTION_ALLOW)?;
            rule.SetProfiles(NET_FW_PROFILE2_ALL.0)?;
            rule.SetEnabled(VARIANT_TRUE)?;
            policy.Rules()?.Add(&rule)?;
        }
        Ok(Self { name })
    }
}
impl Drop for FirewallGuard {
    fn drop(&mut self) {
        let cleanup = || -> anyhow::Result<()> {
            let _com = Com::initialize()?;
            unsafe {
                let policy: INetFwPolicy2 =
                    CoCreateInstance(&NetFwPolicy2, None, CLSCTX_INPROC_SERVER)?;
                policy.Rules()?.Remove(&self.name)?;
            }
            Ok(())
        };
        if cleanup().is_err() {
            eprintln!("network helper: firewall rule cleanup failed");
        }
    }
}
