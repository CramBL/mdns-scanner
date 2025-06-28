use chrono::format::{DelayedFormat, StrftimeItems};
use parking_lot::RwLock;
use std::sync::{Arc, mpsc::Sender};

use super::{LogLevel, LogMessage};

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

    pub fn error(&self, s: impl Into<String> + AsRef<str>) {
        self.log(LogLevel::Error, s);
    }

    pub fn warn(&self, s: impl Into<String> + AsRef<str>) {
        self.log(LogLevel::Warn, s);
    }

    pub fn info(&self, s: impl Into<String> + AsRef<str>) {
        self.log(LogLevel::Info, s);
    }

    pub fn debug(&self, s: impl Into<String> + AsRef<str>) {
        self.log(LogLevel::Debug, s);
    }

    pub fn trace(&self, s: impl Into<String> + AsRef<str>) {
        self.log(LogLevel::Trace, s);
    }

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

    fn log(&self, level: LogLevel, msg: impl AsRef<str>) {
        if level <= *self.verbosity.read() {
            let prefix = level.prefix();
            let msg_ref = msg.as_ref();
            let mut full_msg = String::with_capacity(24 + prefix.len() + msg_ref.len());
            let initial_cap = full_msg.capacity();
            self.timestamp().write_to(&mut full_msg).unwrap();
            full_msg.push_str(prefix);
            full_msg.push_str(msg_ref);
            debug_assert_eq!(initial_cap, full_msg.capacity());
            // Ignore the error here, it will happen if we quit the app while a bunch of threads are sending logs, that's fine.
            let _ = self.tx.send(LogMessage::new(level, full_msg));
        }
    }

    fn timestamp(&self) -> DelayedFormat<StrftimeItems<'_>> {
        chrono::Local::now().format("[%H:%M:%S%.3f] ")
    }
}

impl Clone for Logger {
    fn clone(&self) -> Self {
        Self {
            verbosity: Arc::clone(&self.verbosity),
            tx: self.tx.clone(),
        }
    }
}
