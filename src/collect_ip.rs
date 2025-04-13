use crate::constants;
use crate::ip_info::IpInfo;
use crate::log::Logger;
use crate::util;
use dns_parser::Packet;
use socket2::{Domain, Protocol, Socket, Type};
use std::collections::{HashMap, HashSet};
use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddrV4, UdpSocket};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

pub(crate) fn setup_socket(log: &mut Logger) -> io::Result<UdpSocket> {
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    socket.set_reuse_address(true)?;
    let iface = Ipv4Addr::UNSPECIFIED;
    log.info(format!("Connecting to iface={iface}"));

    let bind_addr = SocketAddrV4::new(iface, constants::MULTICAST_PORT);
    socket.bind(&bind_addr.into())?;

    let udp_socket: UdpSocket = socket.into();
    udp_socket.join_multicast_v4(&constants::MULTICAST_ADDR, &iface)?;
    udp_socket.set_multicast_loop_v4(true)?;
    udp_socket.set_multicast_ttl_v4(5)?;
    udp_socket.set_broadcast(true)?;
    udp_socket.set_read_timeout(Some(Duration::from_secs(2)))?;
    Ok(udp_socket)
}

pub(crate) fn join_multicast_on_all_interfaces(
    log: &mut Logger,
    udp_socket: &UdpSocket,
) -> io::Result<()> {
    let interfaces = get_if_addrs::get_if_addrs()?;
    for iface in interfaces {
        if iface.is_loopback() {
            continue;
        }
        if let IpAddr::V4(ip) = iface.ip() {
            if let Err(e) = udp_socket.join_multicast_v4(&constants::MULTICAST_ADDR, &ip) {
                log.error(format!("Failed to join multicast on {}: {}", ip, e));
            } else {
                log.info(format!("🌐 Joined multicast on interface {ip}"));
            }
        }
    }
    Ok(())
}

pub(crate) fn send_mdns_queries(log: &mut Logger, udp_socket: &UdpSocket) {
    let query_packets = util::build_mdns_queries();
    for packet in &query_packets {
        if let Err(e) = udp_socket.send_to(packet, constants::MDNS_SOCKET_ADDR) {
            log.error(format!("Failed to send query: {}", e));
        }
    }
}

pub(crate) fn scan_all_networks(
    log: &mut Logger,
    sender: mpsc::Sender<IpInfo>,
    discovered_hosts: &Arc<Mutex<HashSet<IpAddr>>>,
    hostnames: &Arc<Mutex<HashMap<IpAddr, Vec<String>>>>,
) {
    let networks = util::get_network_params();

    for ifv4 in networks {
        let sender_clone = sender.clone();
        let log_clone = log.clone();
        let hosts_clone = Arc::clone(&discovered_hosts);
        let hostnames_clone = Arc::clone(&hostnames);

        std::thread::spawn(move || {
            crate::scan_ip::scan_ip_range(
                ifv4,
                hosts_clone,
                hostnames_clone,
                sender_clone,
                log_clone,
            );
        });
    }
}

pub(crate) fn collect_ip_info(
    sender: mpsc::Sender<IpInfo>,
    mut log: Logger,
) -> std::io::Result<()> {
    let udp_socket = setup_socket(&mut log)?;

    log.info("🌐 Listening for mDNS packets on 224.0.0.251:5353...");

    for iface in get_if_addrs::get_if_addrs().unwrap() {
        log.info(format!(
            "🔌 Interface: {:<10} IP: {} is_loopback: {}",
            iface.name,
            iface.ip(),
            iface.is_loopback()
        ));
    }
    join_multicast_on_all_interfaces(&mut log, &udp_socket)?;

    let mut last_query_time: Option<Instant> = None;
    let mut last_ip_scan_time: Option<Instant> = None;
    let mut last_update_hostnames_for_ips = Instant::now();

    // Keep track of devices we've already discovered
    let discovered_hosts: Arc<Mutex<HashSet<IpAddr>>> = Arc::new(Mutex::new(HashSet::new()));

    // Keep track of hostname mapping
    let hostnames: Arc<Mutex<HashMap<IpAddr, Vec<String>>>> = Arc::new(Mutex::new(HashMap::new()));

    let mut buf = [0u8; 1500];
    loop {
        log.trace("Loop start...");
        let first_query_or_time_for_next = last_query_time.is_none()
            || last_query_time.is_some_and(|lqt| lqt.elapsed() >= Duration::from_secs(2));
        if first_query_or_time_for_next {
            log.info("Sending mDNS queries...");
            send_mdns_queries(&mut log, &udp_socket);
            last_query_time = Some(Instant::now());
        }

        let first_scan_or_time_for_next = last_ip_scan_time.is_none()
            || last_ip_scan_time.is_some_and(|lqt| lqt.elapsed() >= Duration::from_secs(30));
        if first_scan_or_time_for_next {
            log.debug("Getting network information from interfaces");
            scan_all_networks(&mut log, sender.clone(), &discovered_hosts, &hostnames);

            last_ip_scan_time = Some(Instant::now());
        }

        if last_update_hostnames_for_ips.elapsed() >= Duration::from_secs(4) {
            let mut ip_info_vec = vec![];
            for (ip, hostnames) in hostnames.lock().unwrap().iter() {
                let mut info = IpInfo::from_ip(*ip);
                for h in hostnames {
                    info.names.push(h.to_owned());
                }
                ip_info_vec.push(info);
            }
            log.info(format!("Sending {} ip info messages", ip_info_vec.len()));
            for ip_info in ip_info_vec {
                sender.send(ip_info).unwrap();
            }

            last_update_hostnames_for_ips = Instant::now();
        }

        log.debug("Receiving on udp_socket...");
        let (len, src) = match udp_socket.recv_from(&mut buf) {
            Ok(ls) => ls,
            Err(e) => match e.kind() {
                std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut => continue,
                _ => {
                    log.error(format!("Error receiving from socket: {e}"));
                    continue;
                }
            },
        };
        log.debug(format!(
            "Received packet from {}:{} with len={}",
            src.ip(),
            src.port(),
            len
        ));

        match Packet::parse(&buf[..len]) {
            Ok(packet) => {
                log.trace(format!("{packet:?}"));

                log.debug(format!("Extracting hostnames from {}", src.ip()));
                crate::scan_ip::extract_hostnames_from_mdns(
                    &packet,
                    src.ip(),
                    Arc::clone(&hostnames),
                    &mut log,
                );

                let mut ip_info = IpInfo::from_ip(src.ip());

                {
                    log.debug("Inserting hostname mapping");
                    let hostnames_map = hostnames.lock().unwrap();
                    if let Some(host_names) = hostnames_map.get(&src.ip()) {
                        for hostname in host_names {
                            ip_info.names.push(hostname.clone());
                        }
                    }
                }

                for answer in packet.answers {
                    ip_info.names.push(answer.name.to_string());
                }
                for additional in packet.additional {
                    ip_info.names.push(additional.name.to_string());
                }
                for ns in packet.nameservers {
                    ip_info.names.push(ns.name.to_string())
                }
                if let Some(o) = packet.opt {
                    log.trace(format!("Opt: {o:?}"));
                }
                for q in packet.questions {
                    log.trace(format!("Q: {q:?}"));
                }

                {
                    log.debug("Inserting discovered hosts");
                    let mut discovered = discovered_hosts.lock().unwrap();
                    discovered.insert(src.ip());
                }

                if let Err(e) = sender.send(ip_info) {
                    log.warn(format!("Receiver dropped, stopping listener. Err: {e}"));
                }
            }
            Err(e) => {
                log.error(format!("Failed to parse DNS packet: {}", e));
            }
        }
    }
}
