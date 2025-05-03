use std::sync::{Arc, RwLock, mpsc::Sender};

use chrono::format::{DelayedFormat, StrftimeItems};

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

    pub(crate) fn error(&self, s: impl Into<String> + AsRef<str>) {
        self.log(LogLevel::Error, s);
    }

    pub(crate) fn warn(&self, s: impl Into<String> + AsRef<str>) {
        self.log(LogLevel::Warn, s);
    }

    pub(crate) fn info(&self, s: impl Into<String> + AsRef<str>) {
        self.log(LogLevel::Info, s);
    }

    pub(crate) fn debug(&self, s: impl Into<String> + AsRef<str>) {
        self.log(LogLevel::Debug, s);
    }

    pub(crate) fn trace(&self, s: impl Into<String> + AsRef<str>) {
        self.log(LogLevel::Trace, s);
    }

    pub(crate) fn verbosity(&self) -> LogLevel {
        *self.verbosity.read().unwrap()
    }

    pub(crate) fn increase_verbosity(&self) {
        let mut level = self.verbosity.write().unwrap();
        *level = level.increase();
    }

    pub(crate) fn decrease_verbosity(&self) {
        let mut level = self.verbosity.write().unwrap();
        *level = level.decrease();
    }

    fn log(&self, level: LogLevel, msg: impl AsRef<str>) {
        if level <= *self.verbosity.read().unwrap() {
            let prefix = level.prefix();
            let msg_ref = msg.as_ref();
            let mut full_msg = String::with_capacity(24 + prefix.len() + msg_ref.len());
            let initial_cap = full_msg.capacity();
            self.timestamp().write_to(&mut full_msg).unwrap();
            full_msg.push_str(prefix);
            full_msg.push_str(msg_ref);
            debug_assert_eq!(initial_cap, full_msg.capacity());
            self.tx.send(LogMessage::new(level, full_msg)).unwrap();
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
