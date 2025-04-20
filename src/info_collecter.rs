use crate::ip_info::IpInfo;
///! Ip info collector receives [IpInfo] and sends new or modified [IpInfo] to the TUI
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::time::Duration;

#[derive(Debug)]
pub enum CollectorUpdate {
    IpInfo(IpInfo),
    PacketSeen(IpAddr),
}

pub fn spawn_collector(
    stop_flag: Arc<AtomicBool>,
    rx_from_scanners: Receiver<IpInfo>,
    tx_to_table_pane: Sender<CollectorUpdate>,
) {
    let mut collector = IpInfoCollector::new(stop_flag, rx_from_scanners, tx_to_table_pane);
    std::thread::spawn(move || {
        collector.run();
    });
}
pub struct IpInfoCollector {
    db: HashMap<IpAddr, IpInfo>,
    rx_info: Receiver<IpInfo>,
    tx_info: Sender<CollectorUpdate>,
    stop_flag: Arc<AtomicBool>,
    update_msgs: Vec<CollectorUpdate>,
}

impl IpInfoCollector {
    pub fn new(
        stop_flag: Arc<AtomicBool>,
        rx_info: Receiver<IpInfo>,
        tx_info: Sender<CollectorUpdate>,
    ) -> Self {
        Self {
            db: HashMap::new(),
            rx_info,
            tx_info,
            stop_flag,
            update_msgs: vec![],
        }
    }

    fn insert(&mut self, ip_info: IpInfo) {
        self.db.insert(ip_info.ip, ip_info);
    }

    fn insert_or_update(&mut self, mut new_ip_info: IpInfo) {
        let ip = new_ip_info.ip;
        if let Some(ip_info) = self.db.get_mut(&ip) {
            if *ip_info != new_ip_info {
                let mut item_modified = false;
                for n in new_ip_info.names {
                    if !ip_info.contains(&n) {
                        ip_info.names.push(n);
                        ip_info.names.sort();
                        item_modified = true;
                    }
                }
                ip_info.seen_count += 1;
                if item_modified {
                    self.update_msgs
                        .push(CollectorUpdate::IpInfo(ip_info.clone()));
                } else {
                    self.update_msgs.push(CollectorUpdate::PacketSeen(ip));
                }
            }
        } else {
            new_ip_info.names.dedup();
            new_ip_info.names.sort();
            self.insert(new_ip_info.clone());
            self.update_msgs.push(CollectorUpdate::IpInfo(new_ip_info));
        }
    }
    pub fn run(&mut self) {
        loop {
            while let Ok(ip_info) = self.rx_info.try_recv() {
                self.insert_or_update(ip_info);
            }

            // Send all modified ip info
            for msg in self.update_msgs.drain(..) {
                if let Err(e) = self.tx_info.send(msg) {
                    if self.stop_flag.load(Ordering::SeqCst) {
                        return;
                    } else {
                        panic!("Failed to send ip info: {}", e);
                    }
                }
            }
            if self.stop_flag.load(Ordering::Relaxed) {
                return;
            }
            thread::sleep(Duration::from_millis(100));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;
    use std::sync::mpsc;

    #[test]
    fn test_ip_info_collector_send_ip_info() {
        let stop_flag = Arc::new(AtomicBool::new(false));
        let (tx_input, rx_input) = mpsc::channel();
        let (tx_output, rx_output) = mpsc::channel();

        let mut collector = IpInfoCollector::new(stop_flag, rx_input, tx_output);

        // Test inserting new IP
        let mut ip_info_1 = IpInfo::from_ip(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)));
        ip_info_1.names.push("test1.local".to_string());

        tx_input.send(ip_info_1.clone()).unwrap();

        // Run collector
        std::thread::spawn(move || {
            collector.run();
        });

        let received = rx_output.recv().unwrap();
        match received {
            CollectorUpdate::IpInfo(ip_info) => assert_eq!(ip_info, ip_info_1),
            _ => panic!("Unexpected message received"),
        }
    }
}
