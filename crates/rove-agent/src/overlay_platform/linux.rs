//! Linux kernel interface validation and binding, shared by Linux and Android.
pub(crate) fn bind_overlay(
    interface: &str,
    address: std::net::SocketAddrV4,
) -> anyhow::Result<tokio::net::TcpListener> {
    validate_overlay(interface, *address.ip())?;
    let socket = tokio::net::TcpSocket::new_v4()?;
    socket.bind_device(Some(interface.as_bytes()))?;
    socket.bind(address.into())?;
    Ok(socket.listen(128)?)
}

pub(crate) fn validate_overlay(
    interface: &str,
    address: std::net::Ipv4Addr,
) -> anyhow::Result<u32> {
    anyhow::ensure!(
        !interface.is_empty()
            && interface.len() < libc::IFNAMSIZ
            && interface
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_-.".contains(&b))
            && interface != "."
            && interface != "..",
        "Invalid overlay interface"
    );
    anyhow::ensure!(
        !address.is_unspecified()
            && !address.is_loopback()
            && !address.is_multicast()
            && !address.is_broadcast(),
        "Invalid overlay address"
    );
    let flags = std::fs::read_to_string(format!("/sys/class/net/{interface}/tun_flags"))?;
    let flags = u32::from_str_radix(flags.trim().trim_start_matches("0x"), 16)?;
    anyhow::ensure!(flags & 3 == 1, "Overlay listener requires a TUN interface");
    anyhow::ensure!(
        interface_has_address(interface, address)?,
        "Address is not assigned to the overlay interface"
    );
    check_main_routes(
        &std::fs::read_to_string("/proc/net/route")?,
        interface,
        address,
    )?;
    let name = std::ffi::CString::new(interface)?;
    // SAFETY: name is a live NUL-terminated interface name.
    let index = unsafe { libc::if_nametoindex(name.as_ptr()) };
    anyhow::ensure!(index != 0, "Overlay interface disappeared");
    Ok(index)
}

pub(crate) fn check_main_routes(
    routes: &str,
    interface: &str,
    address: std::net::Ipv4Addr,
) -> std::io::Result<()> {
    check_subnet_routes(routes, interface, ipnet::Ipv4Net::new(address, 32).unwrap())
}

#[cfg(feature = "easytier")]
pub(crate) fn check_overlay_subnet(interface: &str, subnet: ipnet::Ipv4Net) -> std::io::Result<()> {
    check_subnet_routes(
        &std::fs::read_to_string("/proc/net/route")?,
        interface,
        subnet,
    )
}

pub(crate) fn check_subnet_routes(
    routes: &str,
    interface: &str,
    subnet: ipnet::Ipv4Net,
) -> std::io::Result<()> {
    for line in routes.lines().skip(1) {
        let fields: Vec<_> = line.split_whitespace().collect();
        if fields.len() < 8 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid kernel route table",
            ));
        }
        let parse = |value: &str| {
            u32::from_str_radix(value, 16).map_err(|_| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Invalid kernel route field",
                )
            })
        };
        let destination = u32::from(std::net::Ipv4Addr::from(parse(fields[1])?.to_ne_bytes()));
        let mask = u32::from(std::net::Ipv4Addr::from(parse(fields[7])?.to_ne_bytes()));
        let flags = parse(fields[3])?;
        // Default routes are expected for Internet access, not overlapping
        // specific prefixes. Every non-overlay, non-loopback specific route
        // overlapping our subnet is a conflict, including gateway routes.
        if fields[0] != interface
            && fields[0] != "lo"
            && flags & 1 != 0
            && mask != 0
            && ((destination & mask == u32::from(subnet.network()) & mask)
                || subnet.contains(&std::net::Ipv4Addr::from(destination & mask)))
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AddrInUse,
                "Overlay address overlaps another interface route",
            ));
        }
    }
    Ok(())
}

pub(crate) fn interface_has_address(
    interface: &str,
    address: std::net::Ipv4Addr,
) -> std::io::Result<bool> {
    use std::{ffi::CStr, ptr};
    let mut head = ptr::null_mut();
    // SAFETY: getifaddrs initializes the pointer; it is freed once after traversal.
    if unsafe { libc::getifaddrs(&mut head) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    struct Addresses(*mut libc::ifaddrs);
    impl Drop for Addresses {
        fn drop(&mut self) {
            // SAFETY: this is the unmodified list returned by getifaddrs.
            unsafe { libc::freeifaddrs(self.0) };
        }
    }
    let addresses = Addresses(head);
    let mut current = addresses.0;
    let mut found = false;
    while !current.is_null() {
        // SAFETY: the linked list and sockaddr pointers remain alive until drop.
        let item = unsafe { &*current };
        if !item.ifa_name.is_null()
            && !item.ifa_addr.is_null()
            && unsafe { (*item.ifa_addr).sa_family } as i32 == libc::AF_INET
        {
            let ip = unsafe { &*item.ifa_addr.cast::<libc::sockaddr_in>() };
            let assigned = std::net::Ipv4Addr::from(ip.sin_addr.s_addr.to_ne_bytes());
            // Android assigns secondary IPv4 addresses with kernel labels
            // such as tun0:1. They are addresses of tun0, not another NIC.
            let label = unsafe { CStr::from_ptr(item.ifa_name) }.to_bytes();
            if address_label_interface(label) == interface.as_bytes() {
                found |= assigned == address;
            } else if item.ifa_flags & libc::IFF_LOOPBACK as u32 == 0 && !item.ifa_netmask.is_null()
            {
                // SAFETY: IPv4 getifaddrs entries supply an IPv4 netmask.
                let mask = unsafe { &*item.ifa_netmask.cast::<libc::sockaddr_in>() };
                let mask = u32::from_be(mask.sin_addr.s_addr);
                if (u32::from(assigned) & mask) == (u32::from(address) & mask) {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::AddrInUse,
                        "Overlay address overlaps another interface subnet",
                    ));
                }
            }
        }
        current = item.ifa_next;
    }
    Ok(found)
}

fn address_label_interface(label: &[u8]) -> &[u8] {
    match label.iter().position(|byte| *byte == b':') {
        Some(index)
            if index > 0
                && index + 1 < label.len()
                && label[index + 1..].iter().all(u8::is_ascii_digit) =>
        {
            &label[..index]
        }
        _ => label,
    }
}

#[cfg(test)]
mod address_tests {
    use super::*;

    #[test]
    fn secondary_address_labels_belong_to_exact_base_interface() {
        assert_eq!(address_label_interface(b"tun0"), b"tun0");
        assert_eq!(address_label_interface(b"tun0:1"), b"tun0");
        assert_eq!(address_label_interface(b"tun0:63"), b"tun0");
        assert_ne!(address_label_interface(b"tun01:1"), b"tun0");
        for invalid in [b"tun0:".as_slice(), b"tun0:other", b":1", b"tun0:1:2"] {
            assert_eq!(address_label_interface(invalid), invalid);
        }
    }
}
