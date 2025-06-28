use std::sync::mpsc;

use mds_log::{LogLevel, prelude::Logger};

fn main() -> anyhow::Result<()> {
    let args = std::env::args();
    let new = args.into_iter().any(|a| a.contains("new"));

    let (tx_logs, rx_logs) = mpsc::channel();
    let logger = Logger::new(tx_logs, LogLevel::default());
    let h = mds_dns_sd::spawn_dns_sd_discoverer_test(new, logger)?;
    if new {
        eprintln!("RUNNING NEW");
    } else {
        eprintln!("RUNNING OLD")
    }
    while let Ok(m) = rx_logs.recv() {
        match m {
            mds_log::LogMessage::Error(m)
            | mds_log::LogMessage::Warn(m)
            | mds_log::LogMessage::Info(m)
            | mds_log::LogMessage::Debug(m)
            | mds_log::LogMessage::Trace(m) => {
                eprintln!("{m}");
            }
        }
    }
    eprintln!("\n----");
    let r = h.join().unwrap().unwrap();
    for s in r {
        eprintln!("{s:?}");
    }

    Ok(())
}
