use std::{
    io,
    net::{IpAddr, SocketAddr, TcpStream},
    num::NonZero,
    str::FromStr,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender, TryRecvError},
    },
    thread,
    time::Duration,
};

use mds_dns_sd::lookup;
use mds_ipinfo::port_scan_result::MdnsProbeOutcome;
use mds_util::prelude::DISCOVERED_PREFIX;
use thiserror::Error;

pub const MIN_PORT: u16 = 1;
pub const MAX_PORT: u16 = u16::MAX;

/// Time the optional mDNS probe waits for the host to answer on 5353/udp.
pub const MDNS_PROBE_TIMEOUT: Duration = Duration::from_secs(2);

/// Ports probed by one scan, sorted ascending with duplicates removed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PortList(Vec<u16>);

impl PortList {
    pub fn ports(&self) -> &[u16] {
        &self.0
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl FromIterator<u16> for PortList {
    /// Port 0 is dropped: it cannot be connected to.
    fn from_iter<I: IntoIterator<Item = u16>>(iter: I) -> Self {
        let mut ports: Vec<u16> = iter.into_iter().filter(|p| *p >= MIN_PORT).collect();
        ports.sort_unstable();
        ports.dedup();
        Self(ports)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PortListParseError {
    #[error("no ports given")]
    Empty,
    #[error("{0:?} is not a port in {MIN_PORT}-{MAX_PORT}")]
    InvalidPort(String),
    #[error("range {start}-{end} starts above its end")]
    DescendingRange { start: u16, end: u16 },
}

impl FromStr for PortList {
    type Err = PortListParseError;

    /// Parses comma-separated ports and `start-end` ranges, e.g. `22,80,443,8000-8100`.
    /// Whitespace around any element is ignored. Empty segments (such as the
    /// trailing one in `80,`) are skipped, letting a value be typed comma by comma.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut ports: Vec<u16> = Vec::new();
        for element in s.split(',') {
            let element = element.trim();
            if element.is_empty() {
                continue;
            }
            if let Some((start, end)) = element.split_once('-') {
                let start = parse_port(start)?;
                let end = parse_port(end)?;
                if start > end {
                    return Err(PortListParseError::DescendingRange { start, end });
                }
                ports.extend(start..=end);
            } else {
                ports.push(parse_port(element)?);
            }
        }

        if ports.is_empty() {
            return Err(PortListParseError::Empty);
        }
        Ok(ports.into_iter().collect())
    }
}

fn parse_port(s: &str) -> Result<u16, PortListParseError> {
    match s.trim().parse::<u16>() {
        Ok(port) if port >= MIN_PORT => Ok(port),
        Ok(_) | Err(_) => Err(PortListParseError::InvalidPort(s.trim().to_owned())),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortOutcome {
    Open,
    Closed,
    Filtered,
}

impl PortOutcome {
    /// A refused connection proves the port is closed, while a timeout or a
    /// dropped packet leaves it indistinguishable from a firewall and is reported
    /// as filtered. Any other connect error is a local-side failure (such as fd
    /// exhaustion): it is still reported as filtered, but logged so an
    /// all-filtered run stays diagnosable.
    fn from_connect_result(result: io::Result<TcpStream>) -> Self {
        match result {
            Ok(stream) => {
                drop(stream);
                Self::Open
            }
            Err(e) if e.kind() == io::ErrorKind::ConnectionRefused => Self::Closed,
            Err(e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
                ) =>
            {
                Self::Filtered
            }
            Err(e) => {
                log::debug!(
                    "Port connect failed with an unexpected error, reporting filtered: {e}"
                );
                Self::Filtered
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortScanUpdate {
    PortScanned { port: u16, outcome: PortOutcome },
    MdnsProbed(MdnsProbeOutcome),
    Finished,
}

pub struct PortScanRequest {
    ip: IpAddr,
    ports: PortList,
    connect_timeout: Duration,
    worker_count: NonZero<u16>,
    mdns_probe_timeout: Option<Duration>,
}

impl PortScanRequest {
    pub fn new(
        ip: IpAddr,
        ports: PortList,
        connect_timeout: Duration,
        worker_count: NonZero<u16>,
        mdns_probe_timeout: Option<Duration>,
    ) -> Self {
        Self {
            ip,
            ports,
            connect_timeout,
            worker_count,
            mdns_probe_timeout,
        }
    }

    /// Workers actually spawned: the resolved thread count, capped at the port
    /// count so no worker sits idle, and at least one.
    fn worker_pool_size(&self) -> usize {
        NonZero::<usize>::from(self.worker_count)
            .get()
            .min(self.ports.len())
            .max(1)
    }
}

/// Receiving end of a running scan, plus the switch that stops it.
///
/// Cancelling the job, or dropping it, signals the scan to stop: no further
/// connect attempts or mDNS probing start, and the at-most `worker` connects
/// already in flight run out their connect timeout before the scan thread exits.
pub struct PortScanJob {
    updates: Receiver<PortScanUpdate>,
    cancellation_token: Arc<AtomicBool>,
}

impl Drop for PortScanJob {
    fn drop(&mut self) {
        self.cancel();
    }
}

impl PortScanJob {
    pub fn cancel(&self) {
        self.cancellation_token.store(true, Ordering::Relaxed);
    }

    pub fn try_recv(&self) -> Result<PortScanUpdate, TryRecvError> {
        self.updates.try_recv()
    }

    /// A job with no scanner thread behind it: the returned sender stands in for
    /// the workers so tests can drive a scan through fixed updates.
    #[cfg(any(test, feature = "test-utils"))]
    pub fn stub() -> (Self, Sender<PortScanUpdate>) {
        let (tx, rx) = mpsc::channel();
        let job = Self {
            updates: rx,
            cancellation_token: Arc::new(AtomicBool::new(false)),
        };
        (job, tx)
    }
}

pub fn spawn_port_scan(request: PortScanRequest) -> PortScanJob {
    let (tx, rx) = mpsc::channel();
    let cancellation_token = Arc::new(AtomicBool::new(false));
    let job = PortScanJob {
        updates: rx,
        cancellation_token: Arc::clone(&cancellation_token),
    };

    thread::Builder::new()
        .name("port_scan".into())
        .spawn(move || run_port_scan(request, &tx, &cancellation_token))
        .expect("Failed spawning port scan thread");

    job
}

fn run_port_scan(
    request: PortScanRequest,
    tx: &Sender<PortScanUpdate>,
    cancellation_token: &Arc<AtomicBool>,
) {
    let worker_pool_size = request.worker_pool_size();
    let PortScanRequest {
        ip,
        ports,
        connect_timeout,
        worker_count: _,
        mdns_probe_timeout,
    } = request;

    log::info!(
        "Port scanning {ip}: {port_count} ports, {worker_pool_size} workers, {connect_timeout:.0?} connect timeout, mDNS probe: {mdns}",
        port_count = ports.len(),
        mdns = mdns_probe_timeout.is_some()
    );

    if let Some(mdns_timeout) = mdns_probe_timeout
        && !cancellation_token.load(Ordering::Relaxed)
    {
        tx.send(PortScanUpdate::MdnsProbed(probe_mdns(
            ip,
            mdns_timeout,
            cancellation_token,
        )))
        .ok();
    }

    let pool = threadpool::Builder::new()
        .thread_name("port_scan_worker".to_owned())
        .num_threads(worker_pool_size)
        .build();

    for &port in ports.ports() {
        if cancellation_token.load(Ordering::Relaxed) {
            break;
        }
        pool.execute({
            let tx = tx.clone();
            let cancellation_token = Arc::clone(cancellation_token);
            move || {
                if cancellation_token.load(Ordering::Relaxed) {
                    return;
                }
                let socket_addr = SocketAddr::new(ip, port);
                let outcome = PortOutcome::from_connect_result(TcpStream::connect_timeout(
                    &socket_addr,
                    connect_timeout,
                ));
                if outcome == PortOutcome::Open {
                    log::info!("{DISCOVERED_PREFIX}Port scan: {socket_addr} is open");
                }
                tx.send(PortScanUpdate::PortScanned { port, outcome }).ok();
            }
        });
    }
    pool.join();

    log::debug!(
        "Port scan of {ip} ended (cancelled: {cancelled})",
        cancelled = cancellation_token.load(Ordering::Relaxed)
    );
    tx.send(PortScanUpdate::Finished).ok();
}

fn probe_mdns(ip: IpAddr, timeout: Duration, cancel: &AtomicBool) -> MdnsProbeOutcome {
    let IpAddr::V4(ipv4) = ip else {
        return MdnsProbeOutcome::NotApplicable;
    };
    match lookup::probe_mdns_responder(ipv4, timeout, cancel) {
        Ok(lookup::MdnsReplyMode::Silent) => MdnsProbeOutcome::Silent,
        Ok(lookup::MdnsReplyMode::Unicast) => MdnsProbeOutcome::Unicast,
        Ok(lookup::MdnsReplyMode::Multicast) => MdnsProbeOutcome::Multicast,
        Ok(lookup::MdnsReplyMode::UnicastAndMulticast) => MdnsProbeOutcome::UnicastAndMulticast,
        Err(e) => {
            log::warn!("mDNS probe of {ip} failed: {e}");
            MdnsProbeOutcome::Silent
        }
    }
}

/// Common services listening on `port`, for labelling scan results.
///
/// The label is a naming convention only: nothing is probed beyond the connect.
pub fn service_label(port: u16) -> Option<&'static str> {
    COMMON_PORT_SERVICE_LABELS
        .iter()
        .find(|(known_port, _)| *known_port == port)
        .map(|(_, label)| *label)
}

const COMMON_PORT_SERVICE_LABELS: &[(u16, &str)] = &[
    (20, "ftp-data"),
    (21, "ftp"),
    (22, "ssh"),
    (23, "telnet"),
    (25, "smtp"),
    (53, "dns"),
    (67, "dhcp"),
    (69, "tftp"),
    (80, "http"),
    (110, "pop3"),
    (111, "rpcbind"),
    (123, "ntp"),
    (135, "msrpc"),
    (139, "netbios-ssn"),
    (143, "imap"),
    (161, "snmp"),
    (389, "ldap"),
    (443, "https"),
    (445, "smb"),
    (465, "smtps"),
    (515, "printer"),
    (548, "afp"),
    (554, "rtsp"),
    (587, "submission"),
    (631, "ipp"),
    (636, "ldaps"),
    (873, "rsync"),
    (993, "imaps"),
    (995, "pop3s"),
    (1883, "mqtt"),
    (3306, "mysql"),
    (3389, "rdp"),
    (5000, "upnp"),
    (5060, "sip"),
    (5353, "mdns"),
    (5432, "postgresql"),
    (5900, "vnc"),
    (6379, "redis"),
    (7000, "airplay"),
    (8009, "googlecast"),
    (8080, "http-alt"),
    (8443, "https-alt"),
    (8883, "mqtts"),
    (9100, "jetdirect"),
    (32400, "plex"),
];

#[cfg(test)]
mod tests {
    use std::{
        iter,
        net::{Ipv4Addr, Ipv6Addr, TcpListener},
    };

    use mds_util::prelude::IP_TEST_NET_1_UNREACHABLE;
    use rstest::rstest;

    use super::*;

    fn drain_until_finished(job: &PortScanJob) -> Vec<PortScanUpdate> {
        let mut updates = Vec::new();
        while let Ok(update) = job.updates.recv() {
            let finished = update == PortScanUpdate::Finished;
            updates.push(update);
            if finished {
                break;
            }
        }
        updates
    }

    #[test]
    fn parse_single_port() {
        let list: PortList = "22".parse().unwrap();
        assert_eq!(list.ports(), [22]);
    }

    #[test]
    fn parse_mixed_ports_and_ranges_sorts_and_dedups() {
        let list: PortList = " 443, 22,80 , 22, 8000-8003 ".parse().unwrap();
        assert_eq!(list.ports(), [22, 80, 443, 8000, 8001, 8002, 8003]);
    }

    #[test]
    fn parse_full_range() {
        let list: PortList = "1-65535".parse().unwrap();
        assert_eq!(list.len(), 65535);
    }

    #[test]
    fn parse_single_port_range() {
        let list: PortList = "80-80".parse().unwrap();
        assert_eq!(list.ports(), [80]);
    }

    #[rstest]
    #[case("22,", &[22])]
    #[case("22,,80", &[22, 80])]
    #[case("  , 80 ,", &[80])]
    fn parse_skips_empty_segments(#[case] input: &str, #[case] expected: &[u16]) {
        assert_eq!(input.parse::<PortList>().unwrap().ports(), expected);
    }

    #[rstest]
    #[case("", PortListParseError::Empty)]
    #[case("   ", PortListParseError::Empty)]
    #[case(",", PortListParseError::Empty)]
    #[case(" , , ", PortListParseError::Empty)]
    #[case("0", PortListParseError::InvalidPort("0".to_owned()))]
    #[case("65536", PortListParseError::InvalidPort("65536".to_owned()))]
    #[case("http", PortListParseError::InvalidPort("http".to_owned()))]
    #[case("8000-8100-3", PortListParseError::InvalidPort("8100-3".to_owned()))]
    #[case("0-80", PortListParseError::InvalidPort("0".to_owned()))]
    #[case(
        "8100-8000",
        PortListParseError::DescendingRange {
            start: 8100,
            end: 8000
        }
    )]
    fn parse_rejects_invalid_input(#[case] input: &str, #[case] expected: PortListParseError) {
        assert_eq!(input.parse::<PortList>().unwrap_err(), expected);
    }

    #[test]
    fn collected_ports_are_sorted_deduplicated_and_drop_port_zero() {
        let list: PortList = [443, 0, 22, 443].into_iter().collect();
        assert_eq!(list.ports(), [22, 443]);
    }

    #[test]
    fn connect_to_listening_socket_is_open() {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let socket_addr = listener.local_addr().unwrap();
        let outcome = PortOutcome::from_connect_result(TcpStream::connect_timeout(
            &socket_addr,
            Duration::from_secs(1),
        ));
        assert_eq!(outcome, PortOutcome::Open);
    }

    #[rstest]
    #[case(io::ErrorKind::ConnectionRefused, PortOutcome::Closed)]
    #[case(io::ErrorKind::TimedOut, PortOutcome::Filtered)]
    #[case(io::ErrorKind::WouldBlock, PortOutcome::Filtered)]
    #[case(io::ErrorKind::NetworkUnreachable, PortOutcome::Filtered)]
    fn connect_error_classification(
        #[case] error_kind: io::ErrorKind,
        #[case] expected: PortOutcome,
    ) {
        let outcome = PortOutcome::from_connect_result(Err(io::Error::from(error_kind)));
        assert_eq!(outcome, expected);
    }

    #[test]
    fn scan_reports_every_port_and_finishes() {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let open_port = listener.local_addr().unwrap().port();
        let ports: PortList = iter::once(open_port).collect();

        let job = spawn_port_scan(PortScanRequest::new(
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            ports,
            Duration::from_secs(1),
            NonZero::new(4).unwrap(),
            None,
        ));

        assert_eq!(
            drain_until_finished(&job),
            [
                PortScanUpdate::PortScanned {
                    port: open_port,
                    outcome: PortOutcome::Open
                },
                PortScanUpdate::Finished
            ]
        );
    }

    #[test]
    fn cancelling_a_scan_stops_it_before_every_port_is_probed() {
        let ports: PortList = (MIN_PORT..=MAX_PORT).collect();
        let total_port_count = ports.len();
        let job = spawn_port_scan(PortScanRequest::new(
            IpAddr::V4(IP_TEST_NET_1_UNREACHABLE),
            ports,
            Duration::from_secs(1),
            NonZero::new(4).unwrap(),
            None,
        ));
        job.cancel();

        let mut scanned_count = 0;
        while let Ok(update) = job.updates.recv() {
            match update {
                PortScanUpdate::PortScanned { .. } => scanned_count += 1,
                PortScanUpdate::MdnsProbed(_) => (),
                PortScanUpdate::Finished => break,
            }
        }

        assert!(
            scanned_count < total_port_count,
            "cancelled scan probed all {total_port_count} ports"
        );
    }

    /// An unreachable IPv4 host reports no mDNS responder, emitted before the
    /// scan finishes.
    #[test]
    fn mdns_probe_of_unreachable_host_reports_silence() {
        let job = spawn_port_scan(PortScanRequest::new(
            IpAddr::V4(IP_TEST_NET_1_UNREACHABLE),
            PortList::default(),
            Duration::from_secs(1),
            NonZero::new(4).unwrap(),
            Some(Duration::from_millis(200)),
        ));

        assert_eq!(
            drain_until_finished(&job),
            [
                PortScanUpdate::MdnsProbed(MdnsProbeOutcome::Silent),
                PortScanUpdate::Finished
            ]
        );
    }

    #[test]
    fn mdns_probe_of_ipv6_target_is_not_applicable() {
        let job = spawn_port_scan(PortScanRequest::new(
            IpAddr::V6(Ipv6Addr::LOCALHOST),
            PortList::default(),
            Duration::from_secs(1),
            NonZero::new(4).unwrap(),
            Some(Duration::from_millis(200)),
        ));

        assert_eq!(
            drain_until_finished(&job),
            [
                PortScanUpdate::MdnsProbed(MdnsProbeOutcome::NotApplicable),
                PortScanUpdate::Finished
            ]
        );
    }

    #[rstest]
    #[case(22, Some("ssh"))]
    #[case(8009, Some("googlecast"))]
    #[case(64738, None)]
    fn service_labels(#[case] port: u16, #[case] expected: Option<&str>) {
        assert_eq!(service_label(port), expected);
    }

    #[rstest]
    #[case(64, 3, 3)]
    #[case(4, 1000, 4)]
    #[case(8192, 0, 1)]
    #[case(4, 4, 4)]
    fn worker_pool_never_exceeds_the_port_count(
        #[case] worker_count: u16,
        #[case] port_count: u16,
        #[case] expected: usize,
    ) {
        let ports: PortList = (MIN_PORT..MIN_PORT.saturating_add(port_count)).collect();
        let request = PortScanRequest::new(
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            ports,
            Duration::from_secs(1),
            NonZero::new(worker_count).unwrap(),
            None,
        );
        assert_eq!(request.worker_pool_size(), expected);
    }
}
