use chrono::format::{DelayedFormat, StrftimeItems};
use parking_lot::RwLock;
use std::sync::{
    Arc,
    mpsc::{Receiver, Sender},
};

use super::{LogLevel, LogMessage};

/// Initialize the global logger
pub fn setup_logger(level: LogLevel) -> (Logger, Receiver<LogMessage>) {
    let (tx, rx) = std::sync::mpsc::channel();
    let logger = Logger::new(tx, level);

    let static_logger: &'static dyn log::Log = Box::leak(Box::new(logger.clone()));
    log::set_logger(static_logger).expect("Failed to initialize logger");
    log::set_max_level(log::LevelFilter::Trace);
    (logger, rx)
}

/// Custom logger that sends formatted log messages through an MPSC channel.
pub struct Logger {
    verbosity: Arc<RwLock<LogLevel>>,
    tx: Sender<LogMessage>,
}

impl Logger {
    pub fn new(tx: Sender<LogMessage>, verbosity: LogLevel) -> Self {
        Self {
            verbosity: Arc::new(RwLock::new(verbosity)),
            tx,
        }
    }

    /// Returns the current verbosity level of the logger.
    pub fn verbosity(&self) -> LogLevel {
        *self.verbosity.read()
    }

    pub fn increase_verbosity(&self) {
        let mut level = self.verbosity.write();
        *level = level.increase();
    }

    pub fn decrease_verbosity(&self) {
        let mut level = self.verbosity.write();
        *level = level.decrease();
    }

    fn log_internal(&self, level: LogLevel, msg: String) {
        // Ignore send errors, typically happens during application shutdown
        let _ = self.tx.send(LogMessage::new(level, msg));
    }

    /// Generates a formatted timestamp for log messages.
    fn timestamp(&self) -> DelayedFormat<StrftimeItems<'_>> {
        chrono::Local::now().format("[%H:%M:%S%.3f]")
    }
}

impl Clone for Logger {
    /// Enables cloning of the `Logger` instance, sharing the verbosity and sender.
    fn clone(&self) -> Self {
        Self {
            verbosity: Arc::clone(&self.verbosity),
            tx: self.tx.clone(),
        }
    }
}

/// Implements the `log::Log` trait, allowing this `Logger` to be registered as the global logger.
impl log::Log for Logger {
    /// Determines if a log message with the given metadata should be processed.
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        let verbosity: log::Level = self.verbosity().into();
        metadata.level() <= verbosity
    }

    /// Processes a log record from the `log` facade.
    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            let level: LogLevel = record.level().into();
            let log_args = record.args();
            let timestamp = self.timestamp();
            let msg = format!("{timestamp} {log_args}");
            self.log_internal(level, msg);
        }
    }

    /// Flushes any buffered log messages.
    ///
    /// For an MPSC sender, messages are sent immediately, so this is a no-op.
    fn flush(&self) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use log::{debug, error, info, trace, warn};

    #[test]
    fn test_logger_functionality() {
        let (logger, rx) = setup_logger(LogLevel::Info);

        // --- Scenario 1: Logger verbosity = Info ---
        eprintln!("\n--- Scenario 1: Initial filtering (Logger verbosity: Info) ---");
        error!("Test error message 1");
        warn!("Test warn message 1");
        info!("Test info message 1");
        debug!("Test debug message 1 (should be filtered)");
        trace!("Test trace message 1 (should be filtered)");

        let mut received_messages_s1 = Vec::new();
        while let Ok(msg) = rx.try_recv() {
            received_messages_s1.push(msg);
        }

        let expected_s1 = [
            LogMessage::Error("Test error message 1".into()),
            LogMessage::Warn("Test warn message 1".into()),
            LogMessage::Info("Test info message 1".into()),
        ];

        assert_eq!(
            received_messages_s1.len(),
            expected_s1.len(),
            "S1: Unexpected number of log messages"
        );

        for (expected, actual) in expected_s1.iter().zip(received_messages_s1.iter()) {
            expected.test_assert_equal(actual);
        }

        // --- Scenario 2: Increase verbosity to Debug ---
        eprintln!("\n--- Scenario 2: Increase verbosity to Debug ---");
        logger.increase_verbosity();
        assert_eq!(
            logger.verbosity(),
            LogLevel::Debug,
            "S2: Logger verbosity should be Debug"
        );

        info!("Test info message 2 (should show)");
        debug!("Test debug message 2 (should show)");
        trace!("Test trace message 2 (should still not show)");

        let mut received_messages_s2 = Vec::new();
        while let Ok(msg) = rx.try_recv() {
            received_messages_s2.push(msg);
        }

        let expected_s2 = [
            LogMessage::Info("Test info message 2 (should show)".into()),
            LogMessage::Debug("Test debug message 2 (should show)".into()),
        ];

        assert_eq!(
            received_messages_s2.len(),
            expected_s2.len(),
            "S2: Unexpected number of log messages"
        );

        for (expected, actual) in expected_s2.iter().zip(received_messages_s2.iter()) {
            expected.test_assert_equal(actual);
        }

        // --- Scenario 3: Decrease verbosity to Warn ---
        eprintln!("\n--- Scenario 3: Decrease verbosity to Warn ---");
        logger.decrease_verbosity(); // To Info
        logger.decrease_verbosity(); // To Warn
        assert_eq!(
            logger.verbosity(),
            LogLevel::Warn,
            "S3: Logger verbosity should be Warn"
        );

        warn!("Test warn message 3 (should show)");
        info!("Test info message 3 (should not show)");

        let mut received_messages_s3 = Vec::new();
        while let Ok(msg) = rx.try_recv() {
            received_messages_s3.push(msg);
        }

        let expected_s3 = [LogMessage::Warn("Test warn message 3 (should show)".into())];

        assert_eq!(
            received_messages_s3.len(),
            expected_s3.len(),
            "S3: Unexpected number of log messages"
        );

        for (expected, actual) in expected_s3.iter().zip(received_messages_s3.iter()) {
            expected.test_assert_equal(actual);
        }
    }
}
