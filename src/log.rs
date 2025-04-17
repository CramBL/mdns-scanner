pub(crate) mod db;
pub(crate) mod logger;

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

    pub fn len(&self) -> usize {
        match self {
            LogMessage::Error(s)
            | LogMessage::Warn(s)
            | LogMessage::Info(s)
            | LogMessage::Debug(s)
            | LogMessage::Trace(s) => s.len(),
        }
    }
}
