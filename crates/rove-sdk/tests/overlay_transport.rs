#![cfg(any(target_os = "linux", target_os = "android", windows))]
use rove_sdk::RemoteClient;
use std::net::Ipv4Addr;
use uuid::Uuid;

#[tokio::test]
async fn overlay_transport_requires_explicit_interface_source_and_ip_endpoint() {
    let network = Uuid::new_v4();
    let device = Uuid::new_v4();
    let address: Ipv4Addr = "10.203.1.2".parse().unwrap();
    let endpoint: reqwest::Url = "http://10.203.1.3:43190".parse().unwrap();
    for source in [
        Ipv4Addr::UNSPECIFIED,
        Ipv4Addr::LOCALHOST,
        Ipv4Addr::BROADCAST,
        "224.0.0.1".parse().unwrap(),
    ] {
        assert!(
            RemoteClient::on_overlay(endpoint.clone(), network, device, "rove12345678", source)
                .is_err()
        );
    }
    assert!(RemoteClient::on_overlay(endpoint.clone(), network, device, "", address).is_err());
    for endpoint in [
        "http://peer.example:43190",
        "http://[::1]:43190",
        "https://10.203.1.3:43190",
    ] {
        assert!(
            RemoteClient::on_overlay(
                endpoint.parse().unwrap(),
                network,
                device,
                "rove12345678",
                address
            )
            .is_err()
        );
    }
    assert!(RemoteClient::on_overlay(endpoint, network, device, "rove12345678", address).is_ok());
}

#[tokio::test]
async fn unavailable_overlay_source_never_falls_back_to_host_network() {
    // A live physical/loopback listener must not be reached when the selected
    // overlay source does not exist. This is a transport regression, not native
    // Windows/Android membership evidence.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let result = RemoteClient::discover_on_overlay(
        format!("http://{}", listener.local_addr().unwrap())
            .parse()
            .unwrap(),
        Uuid::new_v4(),
        "rove-missing",
        "192.0.2.203".parse().unwrap(),
    )
    .await;
    assert!(result.is_err());
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(100), listener.accept())
            .await
            .is_err()
    );
}
