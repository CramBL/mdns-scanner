use std::sync::{
    Arc,
    atomic::{AtomicU8, Ordering},
};

#[derive(Clone)]
pub struct RefreshListener {
    prev_refresh: u8,
    listen: Arc<AtomicU8>,
}

impl RefreshListener {
    pub fn peek(&self) -> bool {
        self.listen.load(Ordering::Relaxed) != self.prev_refresh
    }

    pub fn reset(&mut self, current: u8) {
        self.prev_refresh = current;
    }

    pub fn do_refresh(&mut self) -> bool {
        let curr = self.listen.load(Ordering::Relaxed);
        if self.prev_refresh == curr {
            false
        } else {
            self.reset(curr);
            true
        }
    }
}

pub struct Refresher {
    state: Arc<AtomicU8>,
}

impl Default for Refresher {
    fn default() -> Self {
        Self::new()
    }
}

impl Refresher {
    pub fn new() -> Self {
        Self {
            state: Arc::new(AtomicU8::new(0)),
        }
    }

    pub fn signal(&self) {
        let curr = self.state.load(Ordering::SeqCst);
        let new = u8::from(curr == 0);

        debug_assert_ne!(curr, new, "something went wrong");
        self.state.store(new, Ordering::SeqCst);
    }

    pub fn listen(&self) -> RefreshListener {
        RefreshListener {
            prev_refresh: self.state.load(Ordering::Relaxed),
            listen: Arc::clone(&self.state),
        }
    }
}
