//! Rove Windows adaptation: enumerate actual OS interfaces without Npcap.
//! EasyTier uses WinDivert for FakeTCP on Windows x86/x64. The optional pnet
//! packet-capture fallback is explicitly unavailable, never a successful stub.
use crate::{Channel, NetworkInterface};
use ipnetwork::IpNetwork;
use std::io;

pub struct Config;

impl From<&crate::Config> for Config {
    fn from(_: &crate::Config) -> Self {
        Self
    }
}

pub fn channel(_interface: &NetworkInterface, _config: Config) -> io::Result<Channel> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "Npcap capture is not bundled; Windows FakeTCP requires WinDivert",
    ))
}

pub fn interfaces() -> Vec<NetworkInterface> {
    netdev::get_interfaces()
        .into_iter()
        .map(|interface| {
            let ips = interface
                .ipv4
                .iter()
                .filter_map(|ip| IpNetwork::new(ip.addr().into(), ip.prefix_len()).ok())
                .chain(
                    interface
                        .ipv6
                        .iter()
                        .filter_map(|ip| IpNetwork::new(ip.addr().into(), ip.prefix_len()).ok()),
                )
                .collect();
            NetworkInterface {
                name: interface.name,
                description: interface.description.unwrap_or_default(),
                index: interface.index,
                mac: interface
                    .mac_addr
                    .map(|mac| crate::MacAddr::from(mac.octets())),
                ips,
                flags: interface.flags,
            }
        })
        .collect()
}
