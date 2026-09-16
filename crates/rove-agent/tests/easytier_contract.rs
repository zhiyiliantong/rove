#![cfg(feature = "easytier")]

use easytier::{common::config::ConfigLoader, proto::api::manage::NetworkConfig};

// Pin upstream DHCP behavior. Rove now delegates addressing to DHCP without a
// requested subnet; this observation no longer blocks the single-network design.
#[test]
fn upstream_dhcp_discards_requested_subnet() {
    let mut requested = NetworkConfig {
        dhcp: Some(true),
        virtual_ipv4: Some("10.241.0.1".into()),
        network_length: Some(24),
        networking_method: Some(1),
        ..Default::default()
    };
    let automatic = requested.gen_config().unwrap();
    assert!(automatic.get_dhcp());
    assert!(automatic.get_ipv4().is_none());

    requested.dhcp = Some(false);
    let manual = requested.gen_config().unwrap();
    assert_eq!(manual.get_ipv4().unwrap().to_string(), "10.241.0.1/24");
}
