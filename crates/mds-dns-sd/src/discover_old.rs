use dns_parser::{Packet, RData};
use mds_log::prelude::*;
use mds_util::prelude::*;
use std::net::{IpAddr, UdpSocket};

use super::{ServiceInfo, service_registry::ServiceRegistry};

mod query;

pub(super) fn send_dns_sd_queries(log: &Logger) -> anyhow::Result<Vec<ServiceInfo>> {
    let udp_socket = crate::setup_socket()?;

    let query = query::build_dns_sd_query_all()?;
    // udp_socket.send_to(&query, MDNS_SOCKET_ADDR)?;

    let mut registry = ServiceRegistry::default();

    query::send_mdns_query(log, &query, &udp_socket, &mut registry)?;

    let service_info = registry.finalize();
    log.info(format!("Discovered {} service(s)", service_info.len()));
    for service in &service_info {
        log.info(format!(
            "DNS-SD: {} @ {}/{}:{}",
            service.name, service.host, service.ip, service.port
        ));
    }
    Ok(service_info)
}

pub(super) fn handle_mdns_response(
    log: &Logger,
    packet: &Packet,
    socket: &UdpSocket,
    registry: &mut ServiceRegistry,
) -> anyhow::Result<()> {
    log.info(format!("mdns: {packet:?}"));

    for answer in &packet.answers {
        let hostname = answer.name.to_string();
        match &answer.data {
            // Here we actually resolved an ip
            RData::A(ip) => {
                let ip = ip.0;
                log.debug(format!("A: {hostname} -> {ip}"));
                registry.set_ip_for_host(&hostname, IpAddr::V4(ip));
            }
            RData::AAAA(ip6) => {
                let ip = ip6.0;
                log.debug(format!("AAAA: {hostname} -> {ip}"));
                registry.set_ip_for_host(&hostname, IpAddr::V6(ip));
            }
            RData::PTR(name) => {
                let instance = name.to_string();
                log.trace(format!("PTR: {hostname} -> {instance}"));

                if hostname == DNS_SD_QUERY_ALL {
                    log.info(format!("Discovered service type: {instance}"));
                    query::query_ptr(log, &instance, socket, registry)?;
                } else {
                    log.info(format!("Discovered service instance: {instance}"));
                    registry.insert_or_update_instance(instance.clone(), hostname);
                    query::query_srv_and_txt(log, &instance, socket, registry)?;
                }
            }
            RData::SRV(srv) => {
                let host = srv.target.to_string();
                let port = srv.port;
                log.debug(format!("SRV: {hostname} -> {host}:{port}"));
                registry.set_srv(&hostname, host.clone(), port);
                query::query_a_and_aaaa(log, &host, socket, registry)?;
            }
            RData::TXT(txt) => {
                let parsed_txt = txt
                    .iter()
                    .map(|s| String::from_utf8_lossy(s).into_owned())
                    .collect::<Vec<_>>();
                log.debug(format!("TXT: {hostname} -> {parsed_txt:?}"));
                registry.set_txt(&hostname, parsed_txt);
            }

            RData::CNAME(cname) => {
                let canonical = cname.to_string();
                log.debug(format!("CNAME: {hostname} -> {canonical}"));

                registry.set_cname_alias(&hostname, canonical.clone());
                query::query_a_and_aaaa(log, &canonical, socket, registry)?;
            }
            RData::MX(mx) => {
                let domain_hostname = hostname;
                let mail_server = mx.exchange.to_string();
                let priority = mx.preference;
                log.debug(format!(
                    "MX: {domain_hostname} -> {mail_server} (priority: {priority})"
                ));

                registry.set_mail_exchange(&domain_hostname, mail_server.clone(), priority);

                query::query_a_and_aaaa(log, &mail_server, socket, registry)?;
            }
            RData::NS(ns) => {
                let domain_hostname = hostname;
                let nameserver = ns.to_string();
                log.debug(format!("NS: {domain_hostname} -> {nameserver}"));

                registry.set_nameserver(&domain_hostname, nameserver.clone());

                query::query_a_and_aaaa(log, &nameserver, socket, registry)?;
            }
            RData::SOA(soa) => {
                let domain_hostname = hostname;
                let primary_ns = soa.primary_ns.to_string();
                let admin_email = soa.mailbox.to_string();
                let serial = soa.serial;
                // Is this information worth anything..?
                let _refresh = soa.refresh;
                let _retry = soa.retry;
                let _expire = soa.expire;
                let _minimum = soa.minimum_ttl;

                log.debug(format!(
                    "SOA: {domain_hostname} -> NS: {primary_ns}, Admin: {admin_email}, Serial: {serial}"
                ));

                registry.set_soa(&domain_hostname, primary_ns.clone(), admin_email, serial);

                query::query_a_and_aaaa(log, &primary_ns, socket, registry)?;
            }
            RData::Unknown(data) => {
                log.warn(format!("Unknown record for {hostname}, contents: {data:?}",));
            }
        }
    }
    Ok(())
}
