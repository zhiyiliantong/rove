//! Native VPN host contract. No Android/JNI types enter the shared packet code.
use super::{
    vpn_io::{PacketFd, VpnBridge},
    vpn_plan::{VpnNetwork, VpnPlan},
};
use std::{collections::BTreeMap, os::fd::OwnedFd, sync::Arc};
use tokio::sync::Mutex;
use uuid::Uuid;

pub struct NativeTun {
    pub descriptor: OwnedFd,
    pub interface: String,
}
impl NativeTun {
    #[cfg(target_os = "android")]
    pub fn from_descriptor(descriptor: OwnedFd) -> anyhow::Result<Self> {
        use std::os::fd::AsRawFd;
        // SAFETY: initialized ifreq storage; TUNGETIFF writes the actual name
        // and flags of this owned descriptor, not a caller-supplied alias.
        let mut request: libc::ifreq = unsafe { std::mem::zeroed() };
        let result =
            unsafe { libc::ioctl(descriptor.as_raw_fd(), 0x800454d2_u32 as _, &mut request) };
        anyhow::ensure!(result == 0, "Native VPN descriptor is not a TUN");
        // SAFETY: successful TUNGETIFF initialized the flags union member.
        let flags = unsafe { request.ifr_ifru.ifru_flags } as u16;
        anyhow::ensure!(
            flags & 0x1003 == 0x1001,
            "Native VPN requires a raw-IP TUN without a packet-info header"
        );
        let bytes: Vec<u8> = request
            .ifr_name
            .iter()
            .map(|b| *b as u8)
            .take_while(|b| *b != 0)
            .collect();
        anyhow::ensure!(
            !bytes.is_empty() && bytes.len() < libc::IFNAMSIZ,
            "Invalid native TUN name"
        );
        Ok(Self {
            descriptor,
            interface: String::from_utf8(bytes)?,
        })
    }
}

pub trait NativeVpnHost: Send + Sync {
    /// Permission and foreground service must already have been established by
    /// an explicit user action. This method must never launch permission UI.
    fn ready(&self) -> bool;
    /// Only validated overlay prefixes; no default route or DNS replacement.
    fn establish(&self, plan: &VpnPlan) -> anyhow::Result<NativeTun>;
    fn close(&self) -> anyhow::Result<()>;
}

#[derive(Default)]
struct State {
    plan: VpnPlan,
    channels: BTreeMap<Uuid, Arc<PacketFd>>,
    bridge: Option<VpnBridge>,
    interface: String,
}

pub struct MobileVpn {
    host: Arc<dyn NativeVpnHost>,
    state: Mutex<State>,
}
impl MobileVpn {
    pub fn new(host: Arc<dyn NativeVpnHost>) -> Self {
        Self {
            host,
            state: Mutex::new(State::default()),
        }
    }
    pub fn ready(&self) -> bool {
        self.host.ready()
    }

    pub async fn admit(
        &self,
        network: VpnNetwork,
        attach: impl FnOnce(OwnedFd) -> anyhow::Result<()>,
    ) -> anyhow::Result<String> {
        anyhow::ensure!(
            self.ready(),
            "Native VPN permission or foreground service is unavailable"
        );
        let mut state = self.state.lock().await;
        if state.plan.networks().contains(&network) {
            anyhow::ensure!(
                state.bridge.as_ref().is_some_and(VpnBridge::is_running),
                "Native VPN packet channel failed"
            );
            return Ok(state.interface.clone());
        }
        let mut networks = state.plan.networks().to_vec();
        networks.retain(|n| n.network_id != network.network_id);
        networks.push(network.clone());
        // Validate before attaching a new channel or changing the existing VPN.
        let plan = VpnPlan::new(networks)?;
        if !state.channels.contains_key(&network.instance_id) {
            let (channel, fd) = PacketFd::pair()?;
            attach(fd)?;
            state.channels.insert(network.instance_id, channel);
        }
        if let Err(error) = self.replace(&mut state, plan).await {
            let retained: Vec<_> = state
                .plan
                .networks()
                .iter()
                .map(|n| n.instance_id)
                .collect();
            state.channels.retain(|id, _| retained.contains(id));
            return Err(error);
        }
        Ok(state.interface.clone())
    }

    pub async fn remove(&self, network_id: Uuid) -> anyhow::Result<()> {
        let mut state = self.state.lock().await;
        if !state
            .plan
            .networks()
            .iter()
            .any(|n| n.network_id == network_id)
        {
            return Ok(());
        }
        let plan = VpnPlan::new(
            state
                .plan
                .networks()
                .iter()
                .filter(|n| n.network_id != network_id)
                .cloned()
                .collect(),
        )?;
        self.replace(&mut state, plan).await
    }

    async fn replace(&self, state: &mut State, plan: VpnPlan) -> anyhow::Result<()> {
        // A refused addition must not close the still-working previous bridge.
        // Android establishes the replacement before releasing its old fd.
        let tun = if plan.networks().is_empty() {
            None
        } else {
            Some(self.host.establish(&plan)?)
        };
        // Join old packet tasks before publishing the new plan. A removed
        // instance cannot keep injecting packets using a stale plan snapshot.
        if let Some(bridge) = state.bridge.take() {
            let _ = bridge.stop().await;
        }
        state
            .channels
            .retain(|id, _| plan.networks().iter().any(|n| n.instance_id == *id));
        state.interface.clear();
        state.plan = plan.clone();
        if plan.networks().is_empty() {
            state.plan = plan;
            return self.host.close();
        }
        let tun = tun.unwrap();
        let bridge = match VpnBridge::start(tun.descriptor, plan.clone(), state.channels.clone()) {
            Ok(bridge) => bridge,
            Err(error) => {
                let _ = self.host.close();
                return Err(error);
            }
        };
        state.interface = tun.interface;
        state.plan = plan;
        state.bridge = Some(bridge);
        Ok(())
    }

    pub async fn close(&self) {
        let mut state = self.state.lock().await;
        if let Some(bridge) = state.bridge.take() {
            let _ = bridge.stop().await;
        }
        state.channels.clear();
        state.plan = VpnPlan::default();
        state.interface.clear();
        let _ = self.host.close();
    }
}
impl Drop for MobileVpn {
    fn drop(&mut self) {
        // State drop cancels packet tasks and owns their descriptor references.
        let _ = self.host.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        Mutex as StdMutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    };
    struct Host {
        ready: AtomicBool,
        fail: AtomicBool,
        closes: AtomicUsize,
        plans: StdMutex<Vec<VpnPlan>>,
        peers: StdMutex<Vec<Arc<PacketFd>>>,
    }
    impl Host {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                ready: AtomicBool::new(true),
                fail: AtomicBool::new(false),
                closes: AtomicUsize::new(0),
                plans: StdMutex::new(Vec::new()),
                peers: StdMutex::new(Vec::new()),
            })
        }
    }
    impl NativeVpnHost for Host {
        fn ready(&self) -> bool {
            self.ready.load(Ordering::SeqCst)
        }
        fn establish(&self, plan: &VpnPlan) -> anyhow::Result<NativeTun> {
            anyhow::ensure!(
                !self.fail.load(Ordering::SeqCst),
                "fixture: establish failed"
            );
            let (peer, descriptor) = PacketFd::pair()?;
            self.peers.lock().unwrap().push(peer);
            self.plans.lock().unwrap().push(plan.clone());
            Ok(NativeTun {
                descriptor,
                interface: "fixture-tun".into(),
            })
        }
        fn close(&self) -> anyhow::Result<()> {
            self.closes.fetch_add(1, Ordering::SeqCst);
            self.ready.store(false, Ordering::SeqCst);
            self.peers.lock().unwrap().clear();
            Ok(())
        }
    }
    fn network(octet: u8) -> VpnNetwork {
        VpnNetwork {
            network_id: Uuid::new_v4(),
            instance_id: Uuid::new_v4(),
            address: format!("10.203.{octet}.2").parse().unwrap(),
            subnet: format!("10.203.{octet}.0/24").parse().unwrap(),
        }
    }
    #[tokio::test]
    async fn nonoverlapping_networks_share_vpn_and_stop_independently() {
        let host = Host::new();
        let vpn = MobileVpn::new(host.clone());
        let a = network(1);
        let b = network(2);
        let mut descriptors = Vec::new();
        vpn.admit(a.clone(), |fd| {
            descriptors.push(fd);
            Ok(())
        })
        .await
        .unwrap();
        vpn.admit(b.clone(), |fd| {
            descriptors.push(fd);
            Ok(())
        })
        .await
        .unwrap();
        assert_eq!(
            host.plans.lock().unwrap().last().unwrap().networks().len(),
            2
        );
        vpn.admit(a.clone(), |_| panic!("must not attach twice"))
            .await
            .unwrap();
        assert!(
            vpn.admit(network(1), |_| panic!(
                "overlap must fail before native changes"
            ))
            .await
            .is_err()
        );
        assert_eq!(host.plans.lock().unwrap().len(), 2);
        vpn.remove(b.network_id).await.unwrap();
        assert_eq!(
            host.plans.lock().unwrap().last().unwrap().networks(),
            &[a.clone()]
        );
        assert_eq!(host.closes.load(Ordering::SeqCst), 0);
        vpn.remove(a.network_id).await.unwrap();
        assert_eq!(host.closes.load(Ordering::SeqCst), 1);
        assert!(!vpn.ready());
    }
    #[tokio::test]
    async fn refused_addition_retains_existing_plan_and_revoke_is_not_running() {
        let host = Host::new();
        let vpn = MobileVpn::new(host.clone());
        let a = network(1);
        let mut descriptors = Vec::new();
        vpn.admit(a.clone(), |fd| {
            descriptors.push(fd);
            Ok(())
        })
        .await
        .unwrap();
        host.fail.store(true, Ordering::SeqCst);
        assert!(
            vpn.admit(network(2), |fd| {
                descriptors.push(fd);
                Ok(())
            })
            .await
            .is_err()
        );
        vpn.admit(a.clone(), |_| panic!("existing network was discarded"))
            .await
            .unwrap();
        assert_eq!(host.closes.load(Ordering::SeqCst), 0);
        host.ready.store(false, Ordering::SeqCst);
        assert!(
            vpn.admit(a, |_| panic!("revoked VPN must not attach"))
                .await
                .is_err()
        );
        vpn.close().await;
        assert!(vpn.state.lock().await.channels.is_empty());
    }
}
