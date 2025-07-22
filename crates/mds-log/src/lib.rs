use serde::{Deserialize, Serialize};

pub(crate) mod db;
pub(crate) mod logger;
pub mod prelude;

#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialOrd,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    strum::Display,
    strum::EnumString,
)]
#[strum(ascii_case_insensitive)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
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
            LogLevel::Debug | LogLevel::Trace => Self::Trace,
        }
    }

    pub(crate) fn decrease(&self) -> Self {
        match self {
            LogLevel::Error | LogLevel::Warn => Self::Error,
            LogLevel::Info => Self::Warn,
            LogLevel::Debug => Self::Info,
            LogLevel::Trace => Self::Debug,
        }
    }
}

impl From<LogLevel> for log::Level {
    fn from(log_level: LogLevel) -> Self {
        match log_level {
            LogLevel::Error => log::Level::Error,
            LogLevel::Warn => log::Level::Warn,
            LogLevel::Info => log::Level::Info,
            LogLevel::Debug => log::Level::Debug,
            LogLevel::Trace => log::Level::Trace,
        }
    }
}

impl From<log::Level> for LogLevel {
    fn from(log_level: log::Level) -> Self {
        match log_level {
            log::Level::Error => LogLevel::Error,
            log::Level::Warn => LogLevel::Warn,
            log::Level::Info => LogLevel::Info,
            log::Level::Debug => LogLevel::Debug,
            log::Level::Trace => LogLevel::Trace,
        }
    }
}

#[derive(Debug, PartialOrd, PartialEq, Eq)]
pub enum LogMessage {
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

    /// Returns a reference to the inner message string.
    pub fn message(&self) -> &str {
        match self {
            LogMessage::Error(s)
            | LogMessage::Warn(s)
            | LogMessage::Info(s)
            | LogMessage::Debug(s)
            | LogMessage::Trace(s) => s.as_ref(),
        }
    }

    pub fn level(&self) -> LogLevel {
        match self {
            LogMessage::Error(_) => LogLevel::Error,
            LogMessage::Warn(_) => LogLevel::Warn,
            LogMessage::Info(_) => LogLevel::Info,
            LogMessage::Debug(_) => LogLevel::Debug,
            LogMessage::Trace(_) => LogLevel::Trace,
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

    pub fn len(&self) -> usize {
        match self {
            LogMessage::Error(s)
            | LogMessage::Warn(s)
            | LogMessage::Info(s)
            | LogMessage::Debug(s)
            | LogMessage::Trace(s) => s.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            LogMessage::Error(s)
            | LogMessage::Warn(s)
            | LogMessage::Info(s)
            | LogMessage::Debug(s)
            | LogMessage::Trace(s) => s.is_empty(),
        }
    }

    #[cfg(test)]
    pub fn test_assert_equal(&self, other: &LogMessage) {
        // If the timestamp format changes, this should be revisited
        const TIMESTAMP_LEN: usize = "[17:47:03.037] ".len();
        let self_msg_no_ts = self.message(); // Assumes no timestamp
        let other_msg_no_ts = &other.message()[TIMESTAMP_LEN..];
        pretty_assertions::assert_str_eq!(self_msg_no_ts, other_msg_no_ts);
        pretty_assertions::assert_eq!(self.level(), other.level());
    }
}
