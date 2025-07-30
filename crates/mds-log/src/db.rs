use std::num::NonZeroUsize;

use ringbuffer::{AllocRingBuffer, RingBuffer};

use super::{LogLevel, LogMessage};

pub struct LogDb {
    // Keeps track of the longest log message seen
    longest_message: usize,
    logs: AllocRingBuffer<LogMessage>,
    frozen: bool,
    log_limit: usize,
}

impl LogDb {
    pub fn new(log_limit: NonZeroUsize) -> Self {
        Self {
            logs: AllocRingBuffer::new(log_limit.get()),
            longest_message: 0,
            frozen: false,
            log_limit: log_limit.get(),
        }
    }

    pub fn unfreeze(&mut self) {
        self.frozen = false;
    }
    pub fn freeze(&mut self) {
        self.frozen = true;
    }

    pub fn len(&self) -> usize {
        self.logs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.logs.is_empty()
    }

    pub fn longest_message(&self) -> usize {
        self.longest_message
    }

    pub fn push(&mut self, msg: LogMessage) {
        if self.frozen {
            return;
        }
        if msg.len() > self.longest_message {
            self.longest_message = msg.len();
        }
        // Returns previous value if buffer is full, we just discard that value
        _ = self.logs.enqueue(msg);
    }

    pub fn all_logs(&self, log_level: LogLevel) -> Vec<&LogMessage> {
        let mut latest_msgs = Vec::with_capacity(self.log_limit / 2);
        for m in self.logs.iter().rev() {
            if m.is_within_verbosity(log_level) {
                latest_msgs.push(m);
            }
        }

        latest_msgs
    }

    pub fn clear(&mut self) {
        self.logs.clear();
    }
}
