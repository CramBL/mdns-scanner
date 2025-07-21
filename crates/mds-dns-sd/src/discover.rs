use hickory_proto::op::{Message, ResponseCode};
use hickory_proto::rr::{RData, Record};
use hickory_proto::serialize::binary::BinDecodable;
use mds_util::prelude::*;
use std::io;
use std::net::{IpAddr, ToSocketAddrs, UdpSocket};

use super::{ServiceInfo, service_registry::ServiceRegistry};

mod query;

pub(crate) fn send_dns_sd_queries() -> anyhow::Result<Vec<ServiceInfo>> {
    let mut registry = ServiceRegistry::default();

    let udp_socket = crate::setup_socket()?;
    let initial_query = query::DNS_SD_QUERY_ALL_BYTES;

    query::send_mdns_query(initial_query, &udp_socket, &mut registry)?;

    let service_info = registry.finalize();
    log::info!("Discovered {} service(s)", service_info.len());
    for service in &service_info {
        log::info!(
            "DNS-SD: {} @ {}/{}:{}",
            service.name,
            service.host,
            service.ip,
            service.port
        );
    }
    Ok(service_info)
}

pub(super) fn handle_mdns_response(
    message: &Message,
    socket: &impl UdpSocketSender,
    registry: &mut ServiceRegistry,
) -> anyhow::Result<()> {
    if message.response_code() != ResponseCode::NoError {
        log::warn!(
            "Received DNS response with error code: {:?}",
            message.response_code()
        );
        return Ok(());
    }

    for answer in message.answers() {
        handle_dns_record(answer, socket, registry)?;
    }

    // Process additional records (often contain useful A/AAAA records)
    for additional in message.additionals() {
        handle_dns_record(additional, socket, registry)?;
    }

    Ok(())
}

pub(crate) trait UdpSocketSender {
    fn send_to<A>(&self, buf: &[u8], addr: A) -> io::Result<usize>
    where
        A: ToSocketAddrs;
}

impl UdpSocketSender for UdpSocket {
    fn send_to<A>(&self, buf: &[u8], addr: A) -> io::Result<usize>
    where
        A: ToSocketAddrs,
    {
        self.send_to(buf, addr)
    }
}

fn handle_dns_record(
    record: &Record,
    socket: &impl UdpSocketSender,
    registry: &mut ServiceRegistry,
) -> anyhow::Result<()> {
    let hostname = record.name().to_string();

    match record.data() {
        RData::A(ip) => {
            let ip_addr = ip.0;
            log::info!("A: {hostname} -> {ip_addr}");
            registry.set_ip_for_host(&hostname, IpAddr::V4(ip_addr));
        }
        RData::AAAA(ip6) => {
            let ip_addr = ip6.0;
            log::info!("AAAA: {hostname} -> {ip_addr}");
            registry.set_ip_for_host(&hostname, IpAddr::V6(ip_addr));
        }
        RData::PTR(name) => {
            let instance = name.to_string();
            log::info!("PTR: {hostname} -> {instance}");

            if hostname == DNS_SD_QUERY_ALL {
                log::info!("Discovered service type: '{instance}'");
                query::query_ptr(&instance, socket)?;
            } else {
                log::info!("Discovered service instance: '{instance}'");
                registry.insert_or_update_instance(instance.clone(), hostname);
                query::query_srv_and_txt(&instance, socket)?;
            }
        }
        RData::SRV(srv) => {
            let host = srv.target().to_string();
            let port = srv.port();
            log::info!("SRV: {hostname} -> {host}:{port}");
            registry.set_srv(&hostname, host.clone(), port);
            query::query_a_and_aaaa(&host, socket)?;
        }
        RData::TXT(txt) => {
            let parsed_txt = txt
                .txt_data()
                .iter()
                .map(|bytes| String::from_utf8_lossy(bytes).into_owned())
                .collect::<Vec<_>>();
            log::info!("TXT: {hostname} -> {parsed_txt:?}");
            registry.set_txt(&hostname, parsed_txt);
        }
        RData::CNAME(cname) => {
            let canonical = cname.to_string();
            log::info!("CNAME: {hostname} -> {canonical}");

            registry.set_cname_alias(&hostname, canonical.clone());
            query::query_a_and_aaaa(&canonical, socket)?;
        }
        RData::MX(mx) => {
            let domain_hostname = hostname;
            let mail_server = mx.exchange().to_string();
            let priority = mx.preference();
            log::info!("MX: {domain_hostname} -> {mail_server} (priority: {priority})");

            registry.set_mail_exchange(&domain_hostname, mail_server.clone(), priority);
            query::query_a_and_aaaa(&mail_server, socket)?;
        }
        RData::NS(ns) => {
            let domain_hostname = hostname;
            let nameserver = ns.to_string();
            log::info!("NS: {domain_hostname} -> {nameserver}");

            registry.set_nameserver(&domain_hostname, nameserver.clone());
            query::query_a_and_aaaa(&nameserver, socket)?;
        }
        RData::SOA(soa) => {
            let domain_hostname = hostname;
            let primary_ns = soa.mname().to_string();
            let admin_email = soa.rname().to_string();
            let serial = soa.serial();
            // Additional SOA fields might be interesting..?
            let _refresh = soa.refresh();
            let _retry = soa.retry();
            let _expire = soa.expire();
            let _minimum = soa.minimum();

            log::info!(
                "SOA: {domain_hostname} -> NS: {primary_ns}, Admin: {admin_email}, Serial: {serial}"
            );

            registry.set_soa(&domain_hostname, primary_ns.clone(), admin_email, serial);
            query::query_a_and_aaaa(&primary_ns, socket)?;
        }
        RData::ANAME(aname) => log::trace!("ANAME: {hostname} -> {aname} ignoring..."),
        RData::CAA(caa) => log::trace!("CAA {hostname} -> {caa} ignoring..."),
        RData::CERT(cert) => log::trace!("CERT {hostname} -> {cert} ignoring..."),
        RData::CSYNC(csync) => log::trace!("CSYNC {hostname} -> {csync} ignoring..."),
        RData::HINFO(hinfo) => log::trace!("HINFO {hostname} -> {hinfo} ignoring..."),
        RData::HTTPS(https) => log::trace!("HTTPS {hostname} -> {https} ignoring..."),
        RData::NAPTR(naptr) => log::trace!("NAPTR {hostname} -> {naptr} ignoring..."),
        RData::NULL(null) => log::trace!("NULL {hostname} -> {null} ignoring..."),
        RData::OPENPGPKEY(openpgpkey) => {
            log::trace!("OPENPGPKEY: {hostname} -> {openpgpkey} ignoring...")
        }
        RData::OPT(opt) => log::trace!("OPT: {hostname} -> {opt} ignoring..."),
        RData::SSHFP(sshfp) => log::trace!("SSHFP: {hostname} -> {sshfp} ignoring..."),
        RData::SVCB(svcb) => log::trace!("SVCB: {hostname} -> {svcb} ignoring..."),
        RData::TLSA(tlsa) => log::trace!("TLSA: {hostname} -> {tlsa} ignoring..."),
        RData::Unknown { code, rdata } => {
            log::trace!("Unknown: {hostname} -> {code}, {rdata} ignoring...")
        }
        RData::Update0(record_type) => {
            log::trace!("Update0: {hostname} -> {record_type} ignoring...")
        }
        other_data => {
            log::info!("Other DNS record type for {hostname}: {other_data:?}",);
        }
    }
    Ok(())
}

#[inline]
pub fn parse_dns_response(data: &[u8]) -> anyhow::Result<Message> {
    Message::from_bytes(data).map_err(|e| anyhow::anyhow!("Failed to parse DNS message: {e}"))
}

#[cfg(test)]
mod tests {
    use crate::discover::{ServiceRegistry, UdpSocketSender, handle_dns_record};
    use std::net::{IpAddr, Ipv4Addr};

    use hickory_proto::{op::MessageType, rr::RecordType};

    use crate::{ServiceInfo, discover::parse_dns_response};

    const FIRST_MDNS_ROUTER_RESPONSE: &[u8] = &[
        0, 0, 132, 0, 0, 0, 0, 1, 0, 0, 0, 0, 9, 95, 115, 101, 114, 118, 105, 99, 101, 115, 7, 95,
        100, 110, 115, 45, 115, 100, 4, 95, 117, 100, 112, 5, 108, 111, 99, 97, 108, 0, 0, 12, 0,
        1, 0, 0, 17, 148, 0, 14, 6, 95, 97, 108, 101, 120, 97, 4, 95, 116, 99, 112, 192, 35,
    ];

    const SECOND_OUTGOING_PTR: &[u8] = &[
        0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 6, 95, 97, 108, 101, 120, 97, 4, 95, 116, 99, 112, 5,
        108, 111, 99, 97, 108, 0, 0, 12, 0, 1,
    ];
    const FINAL_MDNS_ROUTER_RESPONSE: &[u8] = &[
        0, 0, 132, 0, 0, 0, 0, 4, 0, 0, 0, 0, 6, 95, 97, 108, 101, 120, 97, 4, 95, 116, 99, 112, 5,
        108, 111, 99, 97, 108, 0, 0, 12, 0, 1, 0, 0, 17, 148, 0, 16, 13, 82, 84, 45, 65, 88, 53,
        54, 85, 45, 57, 69, 57, 48, 192, 12, 192, 41, 0, 16, 128, 1, 0, 0, 17, 148, 0, 60, 49, 115,
        107, 105, 108, 108, 83, 101, 116, 117, 112, 73, 100, 61, 56, 98, 49, 56, 51, 56, 54, 99,
        45, 49, 51, 53, 51, 45, 52, 54, 49, 50, 45, 57, 54, 50, 54, 45, 55, 49, 52, 57, 51, 55,
        100, 101, 99, 102, 51, 101, 9, 118, 101, 114, 115, 105, 111, 110, 61, 49, 192, 41, 0, 33,
        128, 1, 0, 0, 0, 120, 0, 22, 0, 0, 0, 0, 0, 80, 13, 82, 84, 45, 65, 88, 53, 54, 85, 45, 57,
        69, 57, 48, 192, 24, 192, 147, 0, 1, 128, 1, 0, 0, 0, 120, 0, 4, 192, 168, 0, 1,
    ];

    struct MockUdpSocket;
    impl UdpSocketSender for MockUdpSocket {
        fn send_to<A>(&self, buf: &[u8], addr: A) -> std::io::Result<usize>
        where
            A: std::net::ToSocketAddrs,
        {
            let _ = addr;
            eprintln!("Sending {buf:?}");
            Ok(buf.len())
        }
    }

    #[test]
    fn test_dns_sd_query_all_const_matches_builder() {
        let first_packet = parse_dns_response(FIRST_MDNS_ROUTER_RESPONSE).unwrap();
        assert_eq!(
            first_packet.answers().first().unwrap().record_type(),
            RecordType::PTR
        );
    }

    #[test]
    fn test_parse_second_outgoing_ptr_packet() {
        let message = parse_dns_response(SECOND_OUTGOING_PTR).unwrap();

        assert_eq!(message.message_type(), MessageType::Query);
        assert_eq!(message.queries().len(), 1);
        assert!(message.answers().is_empty());

        let query = &message.queries()[0];
        assert_eq!(query.query_type(), RecordType::PTR);
        assert_eq!(query.name().to_utf8(), "_alexa._tcp.local.");
    }

    #[test]
    fn test_handle_dns_record_a_record() {
        // Create a test registry
        let mut registry = ServiceRegistry::default();

        let socket = MockUdpSocket;

        let first_message = parse_dns_response(FIRST_MDNS_ROUTER_RESPONSE).unwrap();
        let second_message = parse_dns_response(FINAL_MDNS_ROUTER_RESPONSE).unwrap();

        for a in first_message.answers() {
            handle_dns_record(a, &socket, &mut registry).unwrap();
        }

        for a in second_message.answers() {
            handle_dns_record(a, &socket, &mut registry).unwrap();
        }

        // Assert that the IP was recorded correctly
        let res = registry.finalize();

        assert_eq!(
            res,
            vec![ServiceInfo {
                name: "RT-AX56U-9E90".to_owned(),
                _type: "_alexa._tcp.local.".to_owned(),
                txt: Some(vec![
                    "skillSetupId=8b18386c-1353-4612-9626-714937decf3e".to_owned(),
                    "version=1".to_owned()
                ]),
                host: "RT-AX56U-9E90.local.".to_owned(),
                ip: IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1)),
                port: 80
            }]
        )
    }
}
