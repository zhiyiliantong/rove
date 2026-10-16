//! Packet-preserving native VPN boundary. Routing policy lives in VpnPlan.
//! Owned descriptors are never exported as borrowed integers to async tasks.
use super::vpn_plan::VpnPlan;
use std::{
    collections::BTreeMap,
    io,
    os::fd::{AsRawFd, OwnedFd},
    sync::Arc,
};
use tokio::{io::unix::AsyncFd, task::JoinHandle};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

pub struct PacketFd(AsyncFd<OwnedFd>);
impl PacketFd {
    pub fn new(fd: OwnedFd) -> io::Result<Self> {
        // SAFETY: fd remains owned here; preserve other descriptor flags.
        unsafe {
            let flags = libc::fcntl(fd.as_raw_fd(), libc::F_GETFL);
            if flags < 0 || libc::fcntl(fd.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK) < 0
            {
                return Err(io::Error::last_os_error());
            }
        }
        Ok(Self(AsyncFd::new(fd)?))
    }

    pub fn pair() -> io::Result<(Arc<Self>, OwnedFd)> {
        let (bridge, instance) = std::os::unix::net::UnixDatagram::pair()?;
        instance.set_nonblocking(true)?;
        Ok((Arc::new(Self::new(bridge.into())?), instance.into()))
    }

    async fn read(&self, buffer: &mut [u8]) -> io::Result<usize> {
        loop {
            let mut ready = self.0.readable().await?;
            match ready.try_io(|fd| {
                // SAFETY: writable buffer is live for the synchronous syscall.
                let size = unsafe {
                    libc::read(
                        fd.get_ref().as_raw_fd(),
                        buffer.as_mut_ptr().cast(),
                        buffer.len(),
                    )
                };
                if size < 0 {
                    Err(io::Error::last_os_error())
                } else {
                    Ok(size as usize)
                }
            }) {
                Ok(Ok(0)) => {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "Native VPN channel closed",
                    ));
                }
                Ok(result) => return result,
                Err(_) => continue,
            }
        }
    }

    async fn write(&self, packet: &[u8]) -> io::Result<()> {
        loop {
            let mut ready = self.0.writable().await?;
            match ready.try_io(|fd| {
                // SAFETY: readable packet remains live for this syscall.
                let size = unsafe {
                    libc::write(
                        fd.get_ref().as_raw_fd(),
                        packet.as_ptr().cast(),
                        packet.len(),
                    )
                };
                if size < 0 {
                    Err(io::Error::last_os_error())
                } else if size as usize != packet.len() {
                    Err(io::Error::new(
                        io::ErrorKind::WriteZero,
                        "Partial native VPN packet",
                    ))
                } else {
                    Ok(())
                }
            }) {
                Ok(result) => return result,
                Err(_) => continue,
            }
        }
    }
}

pub struct VpnBridge {
    stop: CancellationToken,
    task: JoinHandle<io::Result<()>>,
}
impl VpnBridge {
    pub fn start(
        tun: OwnedFd,
        plan: VpnPlan,
        channels: BTreeMap<Uuid, Arc<PacketFd>>,
    ) -> anyhow::Result<Self> {
        anyhow::ensure!(!plan.networks().is_empty(), "Empty native VPN plan");
        anyhow::ensure!(
            channels.len() == plan.networks().len()
                && plan
                    .networks()
                    .iter()
                    .all(|n| channels.contains_key(&n.instance_id)),
            "Native VPN channels do not match plan"
        );
        let tun = Arc::new(PacketFd::new(tun)?);
        let plan = Arc::new(plan);
        let stop = CancellationToken::new();
        let stopping = stop.clone();
        let task = tokio::spawn(async move {
            let mut tasks = tokio::task::JoinSet::new();
            for (instance, channel) in &channels {
                let (instance, channel, plan, tun) =
                    (*instance, channel.clone(), plan.clone(), tun.clone());
                tasks.spawn(async move {
                    let mut packet = vec![0; 65536];
                    loop {
                        let size = channel.read(&mut packet).await?;
                        if plan.accepts_inbound(instance, &packet[..size]) {
                            bounded_write(&tun, &packet[..size]).await?;
                        }
                    }
                    #[allow(unreachable_code)]
                    Ok::<(), io::Error>(())
                });
            }
            tasks.spawn(async move {
                let mut packet = vec![0; 65536];
                loop {
                    let size = tun.read(&mut packet).await?;
                    if let Some(instance) = plan.outbound_instance(&packet[..size]) {
                        bounded_write(&channels[&instance], &packet[..size]).await?;
                    }
                }
                #[allow(unreachable_code)]
                Ok::<(), io::Error>(())
            });
            let result = tokio::select! {
                biased;
                _ = stopping.cancelled() => Ok(()),
                result = tasks.join_next() => match result {
                    Some(Ok(result)) => result,
                    Some(Err(error)) => Err(io::Error::other(error)),
                    None => Err(io::Error::other("Native VPN has no packet tasks")),
                },
            };
            tasks.abort_all();
            while tasks.join_next().await.is_some() {}
            result
        });
        Ok(Self { stop, task })
    }
    pub fn is_running(&self) -> bool {
        !self.task.is_finished() && !self.stop.is_cancelled()
    }
    pub async fn stop(mut self) -> io::Result<()> {
        self.stop.cancel();
        (&mut self.task).await.map_err(io::Error::other)?
    }
}
impl Drop for VpnBridge {
    fn drop(&mut self) {
        self.stop.cancel();
        self.task.abort();
    }
}
async fn bounded_write(channel: &PacketFd, packet: &[u8]) -> io::Result<()> {
    tokio::time::timeout(std::time::Duration::from_secs(1), channel.write(packet))
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "Native VPN channel stalled"))?
}

#[cfg(test)]
mod tests {
    use super::super::vpn_plan::VpnNetwork;
    use super::*;
    fn network(octet: u8) -> VpnNetwork {
        VpnNetwork {
            network_id: Uuid::new_v4(),
            instance_id: Uuid::new_v4(),
            address: format!("10.203.{octet}.2").parse().unwrap(),
            subnet: format!("10.203.{octet}.0/24").parse().unwrap(),
        }
    }
    fn packet(source: std::net::Ipv4Addr, destination: std::net::Ipv4Addr) -> Vec<u8> {
        let mut p = vec![0; 20];
        p[0] = 0x45;
        p[2..4].copy_from_slice(&20u16.to_be_bytes());
        p[12..16].copy_from_slice(&source.octets());
        p[16..20].copy_from_slice(&destination.octets());
        p
    }
    #[tokio::test]
    async fn packet_channels_preserve_network_isolation_and_stop() {
        let a = network(1);
        let b = network(2);
        let (os, tun) = PacketFd::pair().unwrap();
        let (a_channel, a_fd) = PacketFd::pair().unwrap();
        let (b_channel, b_fd) = PacketFd::pair().unwrap();
        let a_peer = PacketFd::new(a_fd).unwrap();
        let b_peer = PacketFd::new(b_fd).unwrap();
        let bridge = VpnBridge::start(
            tun,
            VpnPlan::new(vec![a.clone(), b.clone()]).unwrap(),
            BTreeMap::from([(a.instance_id, a_channel), (b.instance_id, b_channel)]),
        )
        .unwrap();
        let outgoing = packet(a.address, "10.203.1.3".parse().unwrap());
        os.write(&outgoing).await.unwrap();
        let mut buffer = [0; 256];
        let size =
            tokio::time::timeout(std::time::Duration::from_secs(1), a_peer.read(&mut buffer))
                .await
                .unwrap()
                .unwrap();
        assert_eq!(&buffer[..size], outgoing);
        assert!(
            tokio::time::timeout(
                std::time::Duration::from_millis(30),
                b_peer.read(&mut buffer)
            )
            .await
            .is_err()
        );
        let incoming = packet("10.203.1.3".parse().unwrap(), a.address);
        b_peer.write(&incoming).await.unwrap();
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(30), os.read(&mut buffer))
                .await
                .is_err()
        );
        a_peer.write(&incoming).await.unwrap();
        let size = tokio::time::timeout(std::time::Duration::from_secs(1), os.read(&mut buffer))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(&buffer[..size], incoming);
        bridge.stop().await.unwrap();
        assert!(os.write(&outgoing).await.is_err());
    }
}
