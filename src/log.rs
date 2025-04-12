#[derive(Debug, Default, PartialEq, Eq, strum_macros::Display)]
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
}
