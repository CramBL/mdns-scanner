use ringbuffer::{AllocRingBuffer, RingBuffer};

use super::{LogLevel, LogMessage};

pub(crate) struct LogDb {
    // Keeps track of the longest log message seen
    longest_message: usize,
    logs: AllocRingBuffer<LogMessage>,
    frozen: bool,
}

impl Default for LogDb {
    fn default() -> Self {
        Self {
            logs: AllocRingBuffer::new(Self::MAX_LOGS),
            longest_message: 0,
            frozen: false,
        }
    }
}

impl LogDb {
    const MAX_LOGS: usize = 1000;

    pub(crate) fn unfreeze(&mut self) {
        self.frozen = false;
    }
    pub fn freeze(&mut self) {
        self.frozen = true;
    }

    pub(crate) fn len(&self) -> usize {
        self.logs.len()
    }

    pub(crate) fn longest_message(&self) -> usize {
        self.longest_message
    }

    pub(crate) fn push(&mut self, msg: LogMessage) {
        if self.frozen {
            return;
        }
        if msg.len() > self.longest_message {
            self.longest_message = msg.len();
        }
        self.logs.push(msg);
    }

    pub(crate) fn all_logs(&self, log_level: LogLevel) -> Vec<&LogMessage> {
        let mut latest_msgs = Vec::with_capacity(Self::MAX_LOGS / 2);
        for m in self.logs.iter().rev() {
            if m.is_within_verbosity(log_level) {
                latest_msgs.push(m);
            }
        }

        latest_msgs
    }
}
