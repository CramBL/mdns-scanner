use ringbuffer::{AllocRingBuffer, RingBuffer};

use super::{LogLevel, LogMessage};

pub(crate) struct LogDb {
    logs: AllocRingBuffer<LogMessage>,
}

impl Default for LogDb {
    fn default() -> Self {
        Self {
            logs: AllocRingBuffer::new(1000),
        }
    }
}

impl LogDb {
    pub(crate) fn push(&mut self, msg: LogMessage) {
        self.logs.push(msg);
    }

    pub(crate) fn latest_logs(&self, log_level: LogLevel) -> Vec<&LogMessage> {
        let max: u16 = 50;
        let mut latest_msgs = Vec::with_capacity(max.into());
        for m in self.logs.iter().rev() {
            if m.is_within_verbosity(log_level) {
                latest_msgs.push(m);
            }
        }
        latest_msgs
    }
}
