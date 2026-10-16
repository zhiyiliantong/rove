//! Restricted helper-side WFP rules. Never changes a user's forwarding mode,
//! proxy, default route, or existing firewall rules. The ordinary agent accepts
//! these guards only through an authenticated, live helper connection.
use std::{io, net::Ipv4Addr, ptr};
use windows_sys::{
    Win32::{Foundation::HANDLE, NetworkManagement::WindowsFilteringPlatform::*},
    core::GUID,
};

fn status(code: u32) -> io::Result<()> {
    if code == 0 {
        Ok(())
    } else {
        Err(io::Error::from_raw_os_error(code as i32))
    }
}

/// A session owns only its own dynamic sublayer and four narrowly scoped block
/// rules. The helper must stop its TUN and agent admission before dropping it.
pub(crate) struct IsolationGuard(HANDLE);

impl Drop for IsolationGuard {
    fn drop(&mut self) {
        // SAFETY: this is the live session opened by install; close also
        // removes its dynamic objects, including after a partial failure.
        unsafe {
            FwpmEngineClose0(self.0);
        }
    }
}

#[derive(Clone, Copy)]
struct Rule {
    layer: GUID,
    address_field: GUID,
    interface_field: GUID,
    subnet: ipnet::Ipv4Net,
}

fn rules(address: Ipv4Addr, subnet: ipnet::Ipv4Net) -> io::Result<[Rule; 4]> {
    if !(1..=30).contains(&subnet.prefix_len())
        || subnet.addr() != subnet.network()
        || !subnet.contains(&address)
        || address == subnet.network()
        || address == subnet.broadcast()
        || address.is_loopback()
        || address.is_multicast()
        || address.is_unspecified()
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Invalid isolated overlay prefix",
        ));
    }
    let host = ipnet::Ipv4Net::new(address, 32).unwrap();
    Ok([
        Rule {
            layer: FWPM_LAYER_INBOUND_IPPACKET_V4,
            address_field: FWPM_CONDITION_IP_LOCAL_ADDRESS,
            interface_field: FWPM_CONDITION_INTERFACE_INDEX,
            subnet: host,
        },
        Rule {
            layer: FWPM_LAYER_OUTBOUND_IPPACKET_V4,
            address_field: FWPM_CONDITION_IP_LOCAL_ADDRESS,
            interface_field: FWPM_CONDITION_INTERFACE_INDEX,
            subnet: host,
        },
        Rule {
            layer: FWPM_LAYER_IPFORWARD_V4,
            address_field: FWPM_CONDITION_IP_DESTINATION_ADDRESS,
            interface_field: FWPM_CONDITION_SOURCE_INTERFACE_INDEX,
            subnet,
        },
        Rule {
            layer: FWPM_LAYER_IPFORWARD_V4,
            address_field: FWPM_CONDITION_IP_SOURCE_ADDRESS,
            interface_field: FWPM_CONDITION_DESTINATION_INTERFACE_INDEX,
            subnet,
        },
    ])
}

impl IsolationGuard {
    /// Called only by the privileged helper after validating that `index` is
    /// its own live Wintun and `address` is actually assigned to that adapter.
    pub(crate) fn install(
        index: u32,
        address: Ipv4Addr,
        subnet: ipnet::Ipv4Net,
    ) -> io::Result<Self> {
        if index == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Missing overlay interface",
            ));
        }
        let rules = rules(address, subnet)?;
        let mut session = FWPM_SESSION0::default();
        session.flags = FWPM_SESSION_FLAG_DYNAMIC;
        session.txnWaitTimeoutInMSec = 3000;
        let mut engine = ptr::null_mut();
        // SAFETY: local authenticated RPC session; no remote BFE and no caller
        // supplied security descriptor. All pointers remain live during calls.
        unsafe {
            status(FwpmEngineOpen0(
                ptr::null(),
                10,
                ptr::null(),
                &session,
                &mut engine,
            ))?;
        }
        let owner = Self(engine);
        let layer_key = GUID::from_u128(uuid::Uuid::new_v4().as_u128());
        let mut name: Vec<u16> = "Rove overlay isolation"
            .encode_utf16()
            .chain(Some(0))
            .collect();
        let mut layer = FWPM_SUBLAYER0::default();
        layer.subLayerKey = layer_key;
        layer.displayData.name = name.as_mut_ptr();
        layer.weight = u16::MAX;
        unsafe {
            status(FwpmTransactionBegin0(engine, 0))?;
            let result = (|| {
                status(FwpmSubLayerAdd0(engine, &layer, ptr::null_mut()))?;
                for rule in rules {
                    add_rule(engine, layer_key, index, rule, &mut name)?;
                }
                status(FwpmTransactionCommit0(engine))
            })();
            if result.is_err() {
                FwpmTransactionAbort0(engine);
            }
            result?;
        }
        Ok(owner)
    }
}

fn add_rule(
    engine: HANDLE,
    layer: GUID,
    index: u32,
    rule: Rule,
    name: &mut [u16],
) -> io::Result<()> {
    // WFP IPv4 addresses and masks use host byte order, unlike sockaddr fields.
    let mut address = FWP_V4_ADDR_AND_MASK {
        addr: u32::from(rule.subnet.network()),
        mask: u32::from(rule.subnet.netmask()),
    };
    let mut conditions = [
        FWPM_FILTER_CONDITION0 {
            fieldKey: rule.address_field,
            matchType: FWP_MATCH_EQUAL,
            conditionValue: FWP_CONDITION_VALUE0 {
                r#type: FWP_V4_ADDR_MASK,
                Anonymous: FWP_CONDITION_VALUE0_0 {
                    v4AddrMask: &mut address,
                },
            },
        },
        FWPM_FILTER_CONDITION0 {
            fieldKey: rule.interface_field,
            matchType: FWP_MATCH_NOT_EQUAL,
            conditionValue: FWP_CONDITION_VALUE0 {
                r#type: FWP_UINT32,
                Anonymous: FWP_CONDITION_VALUE0_0 { uint32: index },
            },
        },
    ];
    let mut filter = FWPM_FILTER0::default();
    filter.filterKey = GUID::from_u128(uuid::Uuid::new_v4().as_u128());
    filter.displayData.name = name.as_mut_ptr();
    filter.layerKey = rule.layer;
    filter.subLayerKey = layer;
    filter.weight.r#type = FWP_EMPTY;
    filter.numFilterConditions = conditions.len() as u32;
    filter.filterCondition = conditions.as_mut_ptr();
    filter.action.r#type = FWP_ACTION_BLOCK;
    // SAFETY: the synchronous API copies the filter and condition data.
    unsafe {
        status(FwpmFilterAdd0(
            engine,
            &filter,
            ptr::null_mut(),
            ptr::null_mut(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn isolation_is_scoped_to_owned_address_and_subnet() {
        let address = "10.203.240.2".parse().unwrap();
        let subnet = "10.203.240.0/24".parse().unwrap();
        let plan = rules(address, subnet).unwrap();
        assert_eq!(plan[0].subnet.to_string(), "10.203.240.2/32");
        assert_eq!(plan[1].subnet, plan[0].subnet);
        assert_eq!(plan[2].subnet, subnet);
        assert_eq!(plan[3].subnet, subnet);
        for invalid in ["0.0.0.0/0", "10.203.241.0/24", "10.203.240.0/32"] {
            assert!(rules(address, invalid.parse().unwrap()).is_err());
        }
    }
}
