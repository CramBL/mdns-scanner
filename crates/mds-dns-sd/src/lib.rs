use std::{io, net::IpAddr, thread::JoinHandle};

use mds_log::prelude::*;

mod discover;
pub mod prelude;
mod service_registry;

#[derive(Debug)]
pub struct ServiceInfo {
    pub name: String,
    pub _type: String,
    pub txt: Option<Vec<String>>,
    pub host: String,
    pub ip: IpAddr,
    pub port: u16,
}

pub fn spawn_dns_sd_discoverer(
    log: Logger,
) -> io::Result<JoinHandle<anyhow::Result<Vec<ServiceInfo>>>> {
    std::thread::Builder::new()
        .name("dns_sd_discoverer".into())
        .spawn(move || discover::send_dns_sd_queries(&log))
}

#[cfg(test)]
mod tests {
    use std::{sync::mpsc, time::Duration};

    use super::*;

    #[ignore = "Can take a long time, since it runs until all discovered services have all the info resolved"]
    #[test]
    fn test_handle_mdns_response_ptr() {
        let (tx_logs, rx_logs) = mpsc::channel();
        let logger = Logger::new(tx_logs, LogLevel::default());
        let h = spawn_dns_sd_discoverer(logger.clone()).unwrap();
        while let Ok(msg) = rx_logs.recv_timeout(Duration::from_secs(2)) {
            println!("{msg:?}");
        }

        let _services = h.join().unwrap().unwrap();

        while let Ok(msg) = rx_logs.recv_timeout(Duration::from_secs(2)) {
            println!("{msg:?}");
        }
    }
}
