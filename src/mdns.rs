use crate::mdns_info::MdnsInfo;
use dns_parser::{Builder, Packet, QueryClass, QueryType};
use socket2::{Domain, Protocol, Socket, Type};
use std::net::{IpAddr, Ipv4Addr, SocketAddrV4, UdpSocket};
use std::str::FromStr;
use std::sync::mpsc;
use std::time::{Duration, Instant};

fn build_mdns_queries() -> Vec<Vec<u8>> {
    let mut packets = Vec::new();

    // 1. Discover available services
    let mut builder = Builder::new_query(1, false);
    builder.add_question(
        "_services._dns-sd._udp.local",
        false,
        QueryType::PTR,
        QueryClass::IN,
    );
    packets.push(builder.build().unwrap());

    // 3. Try an "all records" query for the `.local` domain
    let mut builder = Builder::new_query(2, false);
    builder.add_question(".local", true, QueryType::All, QueryClass::IN);
    packets.push(builder.build().unwrap());

    packets
}

pub(crate) fn parse_mdns(
    sender: mpsc::Sender<MdnsInfo>,
    log: mpsc::Sender<String>,
) -> std::io::Result<()> {
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    socket.set_reuse_address(true)?;
    // TODO: Make it possible to select interface in TUI (make dropdown menu?)
    let iface = Ipv4Addr::UNSPECIFIED;
    let iface = Ipv4Addr::from_str("192.168.0.181").unwrap();
    log.send(format!("Connectin to iface={iface}")).unwrap();

    // Bind to UDP port 5353 on all interfaces
    let bind_addr = SocketAddrV4::new(iface, 5353);
    socket.bind(&bind_addr.into())?;

    // Join mDNS multicast group
    let multicast_addr = Ipv4Addr::new(224, 0, 0, 251);

    let udp_socket: UdpSocket = socket.into();
    udp_socket.join_multicast_v4(&multicast_addr, &iface)?;
    udp_socket.set_multicast_loop_v4(true)?;
    udp_socket.set_multicast_ttl_v4(255)?;
    udp_socket.set_broadcast(true)?;

    log.send("🌐 Listening for mDNS packets on 224.0.0.251:5353...".into())
        .unwrap();

    for iface in get_if_addrs::get_if_addrs().unwrap() {
        log.send(format!(
            "🔌 Interface: {:<10} IP: {} is_loopback: {}",
            iface.name,
            iface.ip(),
            iface.is_loopback()
        ))
        .unwrap();
    }
    let interfaces = get_if_addrs::get_if_addrs()?;
    for iface in interfaces {
        if iface.is_loopback() {
            continue;
        }
        if let IpAddr::V4(ip) = iface.ip() {
            if let Err(e) = udp_socket.join_multicast_v4(&multicast_addr, &ip) {
                log.send(format!("⚠️ Failed to join multicast on {}: {}", ip, e))
                    .unwrap();
            } else {
                log.send(format!("🌐 Joined multicast on interface {}", ip))
                    .unwrap();
            }
        }
    }

    let mut last_query_time: Option<Instant> = None;

    let mut buf = [0u8; 1500];
    loop {
        if last_query_time.is_none()
            || last_query_time.is_some_and(|lqt| lqt.elapsed() >= Duration::from_secs(2))
        {
            log.send("Sending mDNS queries...".into()).unwrap();
            let mdns_addr = ("224.0.0.251", 5353);
            let query_packets = build_mdns_queries();
            for packet in &query_packets {
                if let Err(e) = udp_socket.send_to(packet, mdns_addr) {
                    log.send(format!("❌ Failed to send query: {}", e)).unwrap();
                }
            }
            last_query_time = Some(Instant::now());
        }

        let (len, src) = udp_socket.recv_from(&mut buf)?;

        match Packet::parse(&buf[..len]) {
            Ok(packet) => {
                log.send(format!("{packet:?}")).unwrap();
                let mut mdns_info = MdnsInfo::from_ip(src.ip());
                for answer in packet.answers {
                    mdns_info.names.push(answer.name.to_string());
                }
                for additional in packet.additional {
                    mdns_info.names.push(additional.name.to_string());
                }
                for ns in packet.nameservers {
                    mdns_info.names.push(ns.name.to_string())
                }
                if let Some(o) = packet.opt {
                    log.send(format!("Opt: {o:?}")).unwrap();
                }
                for q in packet.questions {
                    log.send(format!("Q: {q:?}")).unwrap();
                }

                if let Err(e) = sender.send(mdns_info) {
                    eprintln!("⚠️  Receiver dropped, stopping listener. Err: {e}");
                }
            }
            Err(e) => {
                eprintln!("⚠️  Failed to parse DNS packet: {}", e);
            }
        }
    }
}
