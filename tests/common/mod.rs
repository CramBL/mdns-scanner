#![allow(dead_code)]

use mds_collector::CollectorUpdate;
use mds_config::{AppConfig, shared_config::SharedConfig};
use mds_ipinfo::{IpInfo, service::ServiceInstance};
use mds_keybindings::KeyBindings;
use mds_log::{LogLevel, LogMessage, prelude::Logger};
use mds_netscan::progress::ScannerProgress;
use mds_tui::{Model, ScanBackend, message::Message};
use mds_util::refresh::Refresher;
use ratatui::{Terminal, backend::TestBackend};
use semver::Version;
use std::io;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::mpsc::Sender;

pub const TEST_APP_VERSION: Version = Version::new(1, 2, 3);

pub struct ModelHarness<'sb, 't, 'km> {
    pub model: Model<'sb, 't, 'km>,
    pub collector_tx: Sender<CollectorUpdate>,
    pub log_tx: Sender<LogMessage>,
}

impl<'sb, 't, 'km> ModelHarness<'sb, 't, 'km> {
    #[track_caller]
    pub fn new(cfg: AppConfig) -> ModelHarness<'static, 'static, 'static> {
        let (collector_tx, collector_rx) = std::sync::mpsc::channel();
        let backend = ScanBackend {
            cfg: SharedConfig::new(cfg),
            stop_flag: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            refresher: Refresher::new(),
            collector_rx,
            scanner_progress: ScannerProgress::default(),
        };
        let (log_tx, log_rx) = std::sync::mpsc::channel();
        let logger = Logger::new(log_tx.clone(), LogLevel::Info);
        let keymap = Box::leak(Box::new(KeyBindings::default()));
        let model = Model::new(keymap, &TEST_APP_VERSION, (logger, log_rx), backend);
        ModelHarness {
            model,
            collector_tx,
            log_tx,
        }
    }

    /// Drive a message chain to completion.
    pub fn run(&mut self, msg: impl Into<Message>) {
        let mut msg = self.model.update(msg);
        while msg.is_some() {
            msg = self.model.update(msg.unwrap());
        }
    }

    /// Inject an `IpInfo` into the model through the real collector channel and
    /// process it immediately so it is visible in the next render.
    pub fn inject_ip(&mut self, ip_info: IpInfo) {
        self.collector_tx
            .send(CollectorUpdate::IpInfo(ip_info))
            .expect("collector channel unexpectedly closed");
        self.model.recv_new_ip_info();
    }

    /// Send a log message directly to the model's log pane and process it.
    pub fn inject_log(&mut self, msg: LogMessage) {
        self.log_tx
            .send(msg)
            .expect("log channel unexpectedly closed");
        self.model.recv_new_logs();
    }

    #[track_caller]
    pub fn draw(&mut self) -> io::Result<Terminal<TestBackend>> {
        self.draw_sized(80, 20)
    }

    #[track_caller]
    pub fn draw_sized(&mut self, width: u16, height: u16) -> io::Result<Terminal<TestBackend>> {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("terminal new");
        terminal
            .draw(|frame| self.model.render(frame))
            .expect("terminal draw");
        Ok(terminal)
    }
}

/// Filters out host-specific values that vary between machines.
pub fn insta_filters() -> Vec<(&'static str, &'static str)> {
    vec![(
        ".*Scanning potential hosts 0/[0-9]+.*",
        "\"                         Scanning potential hosts 0/1337                        \"",
    )]
}

/// Build an `IpInfo` for a fixed test IP (10.0.0.1) with the given names.
pub fn ip_with_names(names: &[&str]) -> IpInfo {
    let mut info = IpInfo::from_ip(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)));
    for name in names {
        info.add_name((*name).to_owned());
    }
    info
}

/// Build an `IpInfo` for any IP, with names and optional services.
pub fn make_ip(
    ip: IpAddr,
    names: &[&str],
    services: &[(&str, &str, u16)],
    seen_count: u64,
) -> IpInfo {
    let mut info = IpInfo::from_ip(ip);
    info.seen_count = seen_count;
    for name in names {
        info.add_name((*name).to_owned());
    }
    for (name, svc_type, port) in services {
        info.update_with_service_instance(ServiceInstance::new(
            (*name).to_owned(),
            (*svc_type).to_owned(),
            None,
            *port,
            None,
        ));
    }
    info
}

/// A diverse fleet of hosts for rendering tests: multiple IPs, varying name
/// counts, and several service instances across different service types.
pub fn rich_host_fleet() -> Vec<IpInfo> {
    vec![
        make_ip(
            IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)),
            &["gateway.local", "router.home"],
            &[("admin-http", "_http._tcp", 80), ("ssh", "_ssh._tcp", 22)],
            412,
        ),
        make_ip(
            IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2)),
            &["macbook-pro.local"],
            &[("Timothys MacBook Pro", "_smb._tcp", 445)],
            7,
        ),
        make_ip(
            IpAddr::V4(Ipv4Addr::new(10, 0, 0, 3)),
            &["nas.local", "synology.local", "diskstation.home"],
            &[
                ("Photos", "_http._tcp", 5000),
                ("SMB", "_smb._tcp", 445),
                ("AFP", "_afpovertcp._tcp", 548),
            ],
            1024,
        ),
        make_ip(
            IpAddr::V4(Ipv4Addr::new(10, 0, 0, 5)),
            &["raspberry-pi.local"],
            &[("homeassistant", "_http._tcp", 8123)],
            3,
        ),
        make_ip(
            IpAddr::V4(Ipv4Addr::new(10, 0, 0, 8)),
            &["printer.local", "hp-laserjet.home"],
            &[("HP LaserJet", "_ipp._tcp", 631)],
            55,
        ),
        make_ip(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 20)), &[], &[], 1),
        make_ip(
            IpAddr::V4(Ipv4Addr::new(10, 0, 0, 42)),
            &["smart-tv.local"],
            &[
                ("Samsung TV", "_airplay._tcp", 7000),
                ("DLNA", "_http._tcp", 8080),
            ],
            88,
        ),
        make_ip(
            IpAddr::V4(Ipv4Addr::new(10, 0, 0, 100)),
            &["desktop.local", "workstation.home"],
            &[("SSH", "_ssh._tcp", 22)],
            200,
        ),
        make_ip(
            IpAddr::V4(Ipv4Addr::new(10, 0, 0, 200)),
            &[
                "chromecast.local",
                "google-cast.local",
                "living-room-tv.home",
            ],
            &[("Chromecast", "_googlecast._tcp", 8009)],
            31,
        ),
        make_ip(
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
            &["second-router.local"],
            &[("admin", "_http._tcp", 80)],
            6,
        ),
    ]
}

/// Log messages covering Error, Warn, Info, and Debug levels.
pub fn mixed_log_messages() -> Vec<LogMessage> {
    vec![
        LogMessage::Info("[10:00:01.000] Scan started on 10.0.0.0/24".into()),
        LogMessage::Info("[10:00:01.050] Probing 254 addresses".into()),
        LogMessage::Debug("[10:00:01.100] ICMP echo sent to 10.0.0.1".into()),
        LogMessage::Info("[10:00:01.200] Host up: 10.0.0.1 (ping, 3ms)".into()),
        LogMessage::Info("[10:00:01.350] Host up: 10.0.0.2 (ping, 12ms)".into()),
        LogMessage::Warn(
            "[10:00:01.400] mDNS multicast join failed on eth1: permission denied".into(),
        ),
        LogMessage::Info("[10:00:01.500] Host up: 10.0.0.3 (ping, 1ms)".into()),
        LogMessage::Debug(
            "[10:00:01.600] DNS-SD query sent for _services._dns-sd._udp.local".into(),
        ),
        LogMessage::Info("[10:00:01.700] Resolved: nas.local -> 10.0.0.3".into()),
        LogMessage::Info("[10:00:01.800] Host up: 10.0.0.5 (TCP:8123, 45ms)".into()),
        LogMessage::Error("[10:00:01.900] Failed to probe 10.0.0.6: connection refused".into()),
        LogMessage::Info(
            "[10:00:02.000] Service found: Photos._http._tcp.local at 10.0.0.3:5000".into(),
        ),
        LogMessage::Info(
            "[10:00:02.100] Service found: SMB._smb._tcp.local at 10.0.0.3:445".into(),
        ),
        LogMessage::Warn(
            "[10:00:02.200] Duplicate mDNS announcement for gateway.local ignored".into(),
        ),
        LogMessage::Info("[10:00:02.300] Host up: 10.0.0.8 (ping, 8ms)".into()),
        LogMessage::Debug("[10:00:02.400] TTL expired for cached record diskstation.home".into()),
        LogMessage::Info("[10:00:02.500] Resolved: printer.local -> 10.0.0.8".into()),
        LogMessage::Info("[10:00:02.600] Host up: 10.0.0.42 (TCP:7000, 2ms)".into()),
        LogMessage::Info(
            "[10:00:02.700] Service found: Samsung TV._airplay._tcp.local at 10.0.0.42:7000".into(),
        ),
        LogMessage::Error(
            "[10:00:02.800] Socket error on interface wlan0: network unreachable".into(),
        ),
        LogMessage::Info("[10:00:02.900] Scan progress: 180/254 hosts probed".into()),
        LogMessage::Info("[10:00:03.000] Host up: 10.0.0.100 (ping, 5ms)".into()),
        LogMessage::Info("[10:00:03.100] Scan complete. 10 hosts discovered.".into()),
    ]
}
