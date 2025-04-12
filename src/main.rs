use socket2::{Socket, Domain, Type, Protocol};
use std::net::{Ipv4Addr, SocketAddrV4, UdpSocket};
use dns_parser::Packet;

fn main() -> std::io::Result<()> {
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    socket.set_reuse_address(true)?;

    // Bind to UDP port 5353 on all interfaces
    let bind_addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 5353);
    socket.bind(&bind_addr.into())?;

    // Join mDNS multicast group
    let multicast_addr = Ipv4Addr::new(224, 0, 0, 251);
    let iface = Ipv4Addr::UNSPECIFIED;
    socket.join_multicast_v4(&multicast_addr, &iface)?;

    let udp_socket: UdpSocket = socket.into();

    println!("🌐 Listening for mDNS packets on 224.0.0.251:5353...\n");

    let mut buf = [0u8; 1500];
    loop {
        let (len, src) = udp_socket.recv_from(&mut buf)?;
        println!("📨 Packet received from {}", src);

        match Packet::parse(&buf[..len]) {
            Ok(packet) => {
                for answer in packet.answers {
                    println!("→ Answer: {:?}", answer.name);
                }
            }
            Err(e) => {
                println!("⚠️  Failed to parse DNS packet: {}", e);
            }
        }
    }
}
