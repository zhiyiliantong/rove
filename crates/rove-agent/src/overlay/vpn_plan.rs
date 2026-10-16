//! Platform-independent routing plan for a single native VPN carrying several
//! EasyTier instances. Contains runtime facts, never join secrets or defaults.
use std::net::Ipv4Addr;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VpnNetwork {
    pub network_id: Uuid,
    pub instance_id: Uuid,
    pub address: Ipv4Addr,
    pub subnet: ipnet::Ipv4Net,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct VpnPlan {
    networks: Vec<VpnNetwork>,
}

impl VpnPlan {
    /// Build the complete replacement plan before touching the native VPN.
    /// A rejected addition therefore cannot discard a previously running VPN.
    pub fn new(mut networks: Vec<VpnNetwork>) -> anyhow::Result<Self> {
        anyhow::ensure!(networks.len() <= 64, "Too many VPN networks");
        networks.sort_by_key(|network| network.network_id);
        for (index, network) in networks.iter().enumerate() {
            anyhow::ensure!(
                !network.network_id.is_nil() && !network.instance_id.is_nil(),
                "Missing VPN identity"
            );
            anyhow::ensure!(
                (1..=30).contains(&network.subnet.prefix_len())
                    && network.subnet.addr() == network.subnet.network()
                    && network.subnet.contains(&network.address)
                    && network.address != network.subnet.network()
                    && network.address != network.subnet.broadcast()
                    && !network.address.is_loopback()
                    && !network.address.is_multicast()
                    && !network.address.is_unspecified(),
                "Invalid runtime VPN address or subnet"
            );
            for previous in &networks[..index] {
                anyhow::ensure!(
                    previous.network_id != network.network_id
                        && previous.instance_id != network.instance_id,
                    "Duplicate VPN identity"
                );
                anyhow::ensure!(
                    !crate::networks::subnets_overlap(previous.subnet, network.subnet),
                    "VPN network subnets overlap"
                );
            }
        }
        Ok(Self { networks })
    }

    pub fn networks(&self) -> &[VpnNetwork] {
        &self.networks
    }

    /// System TUN -> one EasyTier instance. Reject cross-network sources rather
    /// than silently forwarding one saved network's traffic through another.
    pub fn outbound_instance(&self, packet: &[u8]) -> Option<Uuid> {
        let (source, destination) = ipv4_endpoints(packet)?;
        self.networks
            .iter()
            .find(|network| {
                source == network.address
                    && network.subnet.contains(&destination)
                    && destination != network.subnet.network()
                    && destination != network.subnet.broadcast()
            })
            .map(|network| network.instance_id)
    }

    /// An instance may inject traffic only into its own assigned local address.
    pub fn accepts_inbound(&self, instance_id: Uuid, packet: &[u8]) -> bool {
        let Some((source, destination)) = ipv4_endpoints(packet) else {
            return false;
        };
        self.networks.iter().any(|network| {
            network.instance_id == instance_id
                && destination == network.address
                && network.subnet.contains(&source)
                && source != network.subnet.network()
                && source != network.subnet.broadcast()
        })
    }
}

fn ipv4_endpoints(packet: &[u8]) -> Option<(Ipv4Addr, Ipv4Addr)> {
    if packet.len() < 20 || packet[0] >> 4 != 4 {
        return None;
    }
    let header_length = usize::from(packet[0] & 15) * 4;
    let total_length = usize::from(u16::from_be_bytes([packet[2], packet[3]]));
    if header_length < 20 || header_length > total_length || total_length != packet.len() {
        return None;
    }
    Some((
        Ipv4Addr::new(packet[12], packet[13], packet[14], packet[15]),
        Ipv4Addr::new(packet[16], packet[17], packet[18], packet[19]),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn network(octet: u8) -> VpnNetwork {
        VpnNetwork {
            network_id: Uuid::new_v4(),
            instance_id: Uuid::new_v4(),
            address: Ipv4Addr::new(10, 203, octet, 2),
            subnet: format!("10.203.{octet}.0/24").parse().unwrap(),
        }
    }
    fn packet(source: Ipv4Addr, destination: Ipv4Addr) -> Vec<u8> {
        let mut packet = vec![0; 20];
        packet[0] = 0x45;
        packet[2..4].copy_from_slice(&20u16.to_be_bytes());
        packet[12..16].copy_from_slice(&source.octets());
        packet[16..20].copy_from_slice(&destination.octets());
        packet
    }
    #[test]
    fn disjoint_networks_select_only_their_own_instance() {
        let a = network(1);
        let b = network(2);
        let plan = VpnPlan::new(vec![a.clone(), b.clone()]).unwrap();
        let peer = Ipv4Addr::new(10, 203, 1, 3);
        assert_eq!(
            plan.outbound_instance(&packet(a.address, peer)),
            Some(a.instance_id)
        );
        assert_eq!(plan.outbound_instance(&packet(b.address, peer)), None);
        assert!(plan.accepts_inbound(a.instance_id, &packet(peer, a.address)));
        assert!(!plan.accepts_inbound(b.instance_id, &packet(peer, a.address)));
        assert!(!plan.accepts_inbound(a.instance_id, &packet(peer, b.address)));
        let after_stop = VpnPlan::new(vec![b.clone()]).unwrap();
        assert_eq!(after_stop.outbound_instance(&packet(a.address, peer)), None);
        assert_eq!(after_stop.networks(), &[b]);
    }
    #[test]
    fn reject_overlap_duplicates_default_routes_and_malformed_packets() {
        let a = network(1);
        assert!(VpnPlan::new(vec![a.clone(), a.clone()]).is_err());
        let mut overlap = network(2);
        overlap.subnet = "10.203.0.0/16".parse().unwrap();
        assert!(VpnPlan::new(vec![a.clone(), overlap]).is_err());
        let mut default = a.clone();
        default.subnet = "0.0.0.0/0".parse().unwrap();
        assert!(VpnPlan::new(vec![default]).is_err());
        let plan = VpnPlan::new(vec![a.clone()]).unwrap();
        let mut data = packet(a.address, Ipv4Addr::new(10, 203, 1, 3));
        data[0] = 0x65;
        assert_eq!(plan.outbound_instance(&data), None);
        data[0] = 0x45;
        data.pop();
        assert_eq!(plan.outbound_instance(&data), None);
        assert_eq!(plan.outbound_instance(&[]), None);
        assert_eq!(
            plan.outbound_instance(&packet(a.address, Ipv4Addr::new(1, 1, 1, 1))),
            None
        );
    }
}
