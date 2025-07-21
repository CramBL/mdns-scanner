use mds_log::{LogLevel, prelude::setup_logger};

fn main() -> anyhow::Result<()> {
    let (_logger, rx_logs) = setup_logger(LogLevel::Trace);
    let h = mds_dns_sd::spawn_dns_sd_discoverer()?;

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
