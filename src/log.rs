use std::sync::{Arc, RwLock, mpsc::Sender};

#[derive(Debug, Clone, Copy, Default, PartialOrd, PartialEq, Eq, strum_macros::Display)]
pub(crate) enum LogLevel {
    Error,
    Warn,
    #[default]
    Info,
    Debug,
    Trace,
}

impl LogLevel {
    pub(crate) fn increase(&self) -> Self {
        match self {
            LogLevel::Error => Self::Warn,
            LogLevel::Warn => Self::Info,
            LogLevel::Info => Self::Debug,
            LogLevel::Debug => Self::Trace,
            LogLevel::Trace => Self::Trace,
        }
    }

    pub(crate) fn decrease(&self) -> Self {
        match self {
            LogLevel::Error => Self::Error,
            LogLevel::Warn => Self::Error,
            LogLevel::Info => Self::Warn,
            LogLevel::Debug => Self::Info,
            LogLevel::Trace => Self::Debug,
        }
    }

    const fn prefix(&self) -> &'static str {
        match self {
            LogLevel::Error => "[E] ",
            LogLevel::Warn => "[W] ",
            LogLevel::Info => "[I] ",
            LogLevel::Debug => "[D] ",
            LogLevel::Trace => "[T] ",
        }
    }
}

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

    pub(crate) fn error(&mut self, s: impl Into<String> + AsRef<str>) {
        self.log(LogLevel::Error, s);
    }

    pub(crate) fn warn(&mut self, s: impl Into<String> + AsRef<str>) {
        self.log(LogLevel::Warn, s);
    }

    pub(crate) fn info(&mut self, s: impl Into<String> + AsRef<str>) {
        self.log(LogLevel::Info, s);
    }

    pub(crate) fn debug(&mut self, s: impl Into<String> + AsRef<str>) {
        self.log(LogLevel::Debug, s);
    }

    pub(crate) fn trace(&mut self, s: impl Into<String> + AsRef<str>) {
        self.log(LogLevel::Trace, s);
    }

    pub(crate) fn increase_verbosity(&mut self) {
        let mut level = self.verbosity.write().unwrap();
        *level = level.increase();
    }

    pub(crate) fn decrease_verbosity(&mut self) {
        let mut level = self.verbosity.write().unwrap();
        *level = level.decrease();
    }

    fn log(&mut self, level: LogLevel, msg: impl AsRef<str>) {
        if level <= *self.verbosity.read().unwrap() {
            let now = chrono::Local::now();
            let timestamp = now.format("[%H:%M:%S%.3f] ");
            let prefix = level.prefix();
            let msg_ref = msg.as_ref();
            let mut full_msg = String::with_capacity(24 + prefix.len() + msg_ref.len());
            timestamp.write_to(&mut full_msg).unwrap();
            full_msg.push_str(prefix);
            full_msg.push_str(msg_ref);
            self.tx.send(LogMessage::new(level, full_msg)).unwrap();
        }
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

#[derive(Debug, PartialOrd, PartialEq, Eq)]
pub(crate) enum LogMessage {
    Error(Box<str>),
    Warn(Box<str>),
    Info(Box<str>),
    Debug(Box<str>),
    Trace(Box<str>),
}

impl LogMessage {
    pub fn new(lvl: LogLevel, msg: String) -> Self {
        let msg = msg.into_boxed_str();
        match lvl {
            LogLevel::Error => Self::Error(msg),
            LogLevel::Warn => Self::Warn(msg),
            LogLevel::Info => Self::Info(msg),
            LogLevel::Debug => Self::Debug(msg),
            LogLevel::Trace => Self::Trace(msg),
        }
    }

    pub fn is_within_verbosity(&self, lvl: LogLevel) -> bool {
        match self {
            LogMessage::Error(_) => lvl >= LogLevel::Error,
            LogMessage::Warn(_) => lvl >= LogLevel::Warn,
            LogMessage::Info(_) => lvl >= LogLevel::Info,
            LogMessage::Debug(_) => lvl >= LogLevel::Debug,
            LogMessage::Trace(_) => lvl == LogLevel::Trace,
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            LogMessage::Error(s)
            | LogMessage::Warn(s)
            | LogMessage::Info(s)
            | LogMessage::Debug(s)
            | LogMessage::Trace(s) => s.as_ref(),
        }
    }
}

pub(crate) fn latest_messages(msgs: &[LogMessage], lvl: LogLevel, max: u16) -> Vec<&LogMessage> {
    let mut latest_msgs = Vec::with_capacity(max.into());
    for m in msgs.iter().rev() {
        if m.is_within_verbosity(lvl) {
            latest_msgs.push(m);
        }
    }
    latest_msgs
}
