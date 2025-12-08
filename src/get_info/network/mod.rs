use crate::data_struct::{Connections, Network};
use log::trace;
use std::collections::HashSet;
use sysinfo::Networks;
use tokio::sync::mpsc::Receiver;

#[cfg(target_os = "linux")]
mod netlink;
pub mod network_saver;

static mut LAST_NETWORK: (u64, u64) = (0, 0);

pub static mut DURATION: f64 = 0.0;
pub fn realtime_network(
    network: &Networks,
    network_saver_rx: &mut Receiver<(u64, u64)>,
) -> Network {
    let (up, down, _, _) = filter_network(network);

    if let Ok(network_saver_rx) = network_saver_rx.try_recv() {
        unsafe {
            LAST_NETWORK = (network_saver_rx.0, network_saver_rx.1);
        }
    }

    unsafe {
        let network_info = Network {
            up: (up as f64 / (DURATION / 1000.0)) as u64,
            down: (down as f64 / (DURATION / 1000.0)) as u64,
            total_up: LAST_NETWORK.0,
            total_down: LAST_NETWORK.1,
        };
        trace!("REALTIME NETWORK successfully retrieved: {network_info:?}");
        network_info
    }
}

#[cfg(target_os = "linux")]
pub fn realtime_connections() -> Connections {
    use netlink::connections_count_with_protocol;
    let tcp4 =
        connections_count_with_protocol(libc::AF_INET as u8, libc::IPPROTO_TCP as u8).unwrap_or(0);
    let tcp6 =
        connections_count_with_protocol(libc::AF_INET6 as u8, libc::IPPROTO_TCP as u8).unwrap_or(0);
    let udp4 =
        connections_count_with_protocol(libc::AF_INET as u8, libc::IPPROTO_UDP as u8).unwrap_or(0);
    let udp6 =
        connections_count_with_protocol(libc::AF_INET6 as u8, libc::IPPROTO_UDP as u8).unwrap_or(0);
    let connections = Connections {
        tcp: tcp4 + tcp6,
        udp: udp4 + udp6,
    };
    trace!(
        "REALTIME CONNECTIONS successfully retrieved: {:?}",
        connections
    );
    connections
}

#[cfg(target_os = "windows")]
pub fn realtime_connections() -> Connections {
    use netstat2::{ProtocolFlags, ProtocolSocketInfo, iterate_sockets_info_without_pids};
    let proto_flags = ProtocolFlags::TCP | ProtocolFlags::UDP;

    let Ok(sockets_iterator) = iterate_sockets_info_without_pids(proto_flags) else {
        let connections = Connections { tcp: 0, udp: 0 };
        trace!("REALTIME CONNECTIONS successfully retrieved: {connections:?}");
        return connections;
    };

    let (mut tcp_count, mut udp_count) = (0, 0);

    for info_result in sockets_iterator.flatten() {
        match info_result.protocol_socket_info {
            ProtocolSocketInfo::Tcp(_) => tcp_count += 1,
            ProtocolSocketInfo::Udp(_) => udp_count += 1,
        }
    }

    let connections = Connections {
        tcp: tcp_count,
        udp: udp_count,
    };
    trace!("REALTIME CONNECTIONS successfully retrieved: {connections:?}");
    connections
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn realtime_connections() -> Connections {
    let connections = Connections { tcp: 0, udp: 0 };
    trace!(
        "REALTIME CONNECTIONS successfully retrieved: {:?}",
        connections
    );
    connections
}

pub fn filter_network(network: &Networks) -> (u64, u64, u64, u64) {
    let mut total_up = 0;
    let mut total_down = 0;
    let mut up = 0;
    let mut down = 0;

    let filter_keywords: HashSet<&str> = [
        "br", "cni", "docker", "podman", "flannel", "lo", "veth", "virbr", "vmbr", "tap", "tun",
        "fwln", "fwpr",
    ]
    .iter()
    .cloned()
    .collect();

    for (name, data) in network {
        let should_filter = filter_keywords
            .iter()
            .any(|&keyword| name.contains(keyword));

        if should_filter || data.mac_address().0 == [0, 0, 0, 0, 0, 0] {
            continue;
        }

        total_up += data.total_transmitted();
        total_down += data.total_received();
        up += data.transmitted();
        down += data.received();
    }

    (up, down, total_up, total_down)
}
