use hickory_proto::op::{Message, ResponseCode};
use hickory_proto::rr::{RData, Record};
use hickory_proto::serialize::binary::BinDecodable;
use mds_log::prelude::*;
use mds_util::prelude::*;
use std::net::{IpAddr, UdpSocket};

use super::{ServiceInfo, service_registry::ServiceRegistry};

mod query;

pub(crate) fn send_dns_sd_queries(log: &Logger) -> anyhow::Result<Vec<ServiceInfo>> {
    let mut registry = ServiceRegistry::default();

    let udp_socket = crate::setup_socket()?;
    let initial_query = query::DNS_SD_QUERY_ALL_BYTES;

    query::send_mdns_query(log, initial_query, &udp_socket, &mut registry)?;

    let service_info = registry.finalize();
    log.info(format!(
        "mdns: Discovered {} service(s)",
        service_info.len()
    ));
    for service in &service_info {
        log.info(format!(
            "mdns: DNS-SD: {} @ {}/{}:{}",
            service.name, service.host, service.ip, service.port
        ));
    }
    Ok(service_info)
}

pub(super) fn handle_mdns_response(
    log: &Logger,
    message: &Message,
    socket: &UdpSocket,
    registry: &mut ServiceRegistry,
) -> anyhow::Result<()> {
    log.info(format!("{}", message.message_type()));
    if message.response_code() != ResponseCode::NoError {
        log.warn(format!(
            "Received DNS response with error code: {:?}",
            message.response_code()
        ));
        return Ok(());
    }

    for answer in message.answers() {
        handle_dns_record(log, answer, socket, registry)?;
    }

    // Process additional records (often contain useful A/AAAA records)
    for additional in message.additionals() {
        handle_dns_record(log, additional, socket, registry)?;
    }

    Ok(())
}

fn handle_dns_record(
    log: &Logger,
    record: &Record,
    socket: &UdpSocket,
    registry: &mut ServiceRegistry,
) -> anyhow::Result<()> {
    let hostname = record.name().to_string();
    log.info(format!(
        "mdns: {}({}) {record:?}",
        record.record_type(),
        hostname == DNS_SD_QUERY_ALL
    ));

    match record.data() {
        RData::A(ip) => {
            let ip_addr = ip.0;
            log.info(format!("A: {hostname} -> {ip_addr}"));
            registry.set_ip_for_host(&hostname, IpAddr::V4(ip_addr));
        }
        RData::AAAA(ip6) => {
            let ip_addr = ip6.0;
            log.info(format!("AAAA: {hostname} -> {ip_addr}"));
            registry.set_ip_for_host(&hostname, IpAddr::V6(ip_addr));
        }
        RData::PTR(name) => {
            let instance = name.to_string();
            log.info(format!("PTR: {hostname} -> {instance}"));

            if hostname == DNS_SD_QUERY_ALL {
                log.info(format!("mdns: Discovered service type: '{instance}'"));
                query::query_ptr(&instance, socket)?;
            } else {
                // '_alexa._tcp.local.'
                log.info(format!("mdns: Discovered service instance: '{instance}'"));
                registry.insert_or_update_instance(instance.clone(), hostname);
                query::query_srv_and_txt(&instance, socket)?;
                //crate::discover_old::query::query_srv_and_txt( &instance, socket)?;
            }
        }
        RData::SRV(srv) => {
            let host = srv.target().to_string();
            let port = srv.port();
            log.info(format!("SRV: {hostname} -> {host}:{port}"));
            registry.set_srv(&hostname, host.clone(), port);
            query::query_a_and_aaaa(&host, socket)?;
        }
        RData::TXT(txt) => {
            let parsed_txt = txt
                .txt_data()
                .iter()
                .map(|bytes| String::from_utf8_lossy(bytes).into_owned())
                .collect::<Vec<_>>();
            log.info(format!("TXT: {hostname} -> {parsed_txt:?}"));
            registry.set_txt(&hostname, parsed_txt);
        }
        RData::CNAME(cname) => {
            let canonical = cname.to_string();
            log.info(format!("CNAME: {hostname} -> {canonical}"));

            registry.set_cname_alias(&hostname, canonical.clone());
            query::query_a_and_aaaa(&canonical, socket)?;
        }
        RData::MX(mx) => {
            let domain_hostname = hostname;
            let mail_server = mx.exchange().to_string();
            let priority = mx.preference();
            log.info(format!(
                "MX: {domain_hostname} -> {mail_server} (priority: {priority})"
            ));

            registry.set_mail_exchange(&domain_hostname, mail_server.clone(), priority);
            query::query_a_and_aaaa(&mail_server, socket)?;
        }
        RData::NS(ns) => {
            let domain_hostname = hostname;
            let nameserver = ns.to_string();
            log.info(format!("NS: {domain_hostname} -> {nameserver}"));

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

            log.info(format!(
                "SOA: {domain_hostname} -> NS: {primary_ns}, Admin: {admin_email}, Serial: {serial}"
            ));

            registry.set_soa(&domain_hostname, primary_ns.clone(), admin_email, serial);
            query::query_a_and_aaaa(&primary_ns, socket)?;
        }
        RData::ANAME(aname) => log.trace(format!("ANAME: {hostname} -> {aname} ignoring... ")),
        RData::CAA(caa) => log.trace(format!("CAA {hostname} -> {caa} ignoring...")),
        RData::CERT(cert) => log.trace(format!("CERT {hostname} -> {cert} ignoring...")),
        RData::CSYNC(csync) => log.trace(format!("CSYNC {hostname} -> {csync} ignoring...")),
        RData::HINFO(hinfo) => log.trace(format!("HINFO {hostname} -> {hinfo} ignoring...")),
        RData::HTTPS(https) => log.trace(format!("HTTPS {hostname} -> {https} ignoring...")),
        RData::NAPTR(naptr) => log.trace(format!("NAPTR {hostname} -> {naptr} ignoring...")),
        RData::NULL(null) => log.trace(format!("NULL {hostname} -> {null} ignoring...")),
        RData::OPENPGPKEY(openpgpkey) => log.trace(format!(
            "OPENPGPKEY: {hostname} -> {openpgpkey} ignoring..."
        )),
        RData::OPT(opt) => log.trace(format!("OPT: {hostname} -> {opt} ignoring...")),
        RData::SSHFP(sshfp) => log.trace(format!("SSHFP: {hostname} -> {sshfp} ignoring...")),
        RData::SVCB(svcb) => log.trace(format!("SVCB: {hostname} -> {svcb} ignoring...")),
        RData::TLSA(tlsa) => log.trace(format!("TLSA: {hostname} -> {tlsa} ignoring...")),
        RData::Unknown { code, rdata } => log.trace(format!(
            "Unknown: {hostname} -> {code}, {rdata} ignoring..."
        )),
        RData::Update0(record_type) => {
            log.trace(format!("Update0: {hostname} -> {record_type} ignoring..."))
        }
        other_data => {
            log.info(format!(
                "Other DNS record type for {hostname}: {other_data:?}",
            ));
        }
    }
    Ok(())
}

#[inline]
pub fn parse_dns_response(data: &[u8]) -> anyhow::Result<Message> {
    Message::from_bytes(data).map_err(|e| anyhow::anyhow!("Failed to parse DNS message: {e}"))
}
