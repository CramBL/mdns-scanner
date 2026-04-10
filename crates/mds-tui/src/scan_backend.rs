use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{self, Receiver};

use mds_collector::CollectorUpdate;
use mds_config::AppConfig;
use mds_config::shared_config::SharedConfig;
use mds_netscan::{NetworkScanner, progress::ScannerProgress};
use mds_util::refresh::Refresher;

/// Owns the network scanning infrastructure wired to a channel that feeds
/// [`CollectorUpdate`] messages into the TUI.
pub struct ScanBackend {
    pub cfg: SharedConfig,
    pub stop_flag: Arc<AtomicBool>,
    pub refresher: Refresher,
    pub collector_rx: Receiver<CollectorUpdate>,
    pub scanner_progress: ScannerProgress,
}

impl ScanBackend {
    /// Launch the full scanning infrastructure: spawns the collector and
    /// network-scanner threads and returns a wired-up backend.
    pub fn launch(cfg: AppConfig) -> Self {
        let cfg = SharedConfig::new(cfg);
        let stop_flag = Arc::new(AtomicBool::new(false));
        let refresher = Refresher::new();

        let (collector_tx, collector_rx) = mpsc::channel();
        let (scanner_tx, scanner_rx) = mpsc::channel();

        mds_collector::spawn_collector(
            Arc::clone(&stop_flag),
            scanner_rx,
            collector_tx,
            cfg.clone(),
            refresher.listen(),
        );

        let scanner = NetworkScanner::new(
            Arc::clone(&stop_flag),
            scanner_tx,
            cfg.clone(),
            refresher.listen(),
        );
        let scanner_progress = scanner.spawn();

        ScanBackend {
            cfg,
            stop_flag,
            refresher,
            collector_rx,
            scanner_progress,
        }
    }
}
