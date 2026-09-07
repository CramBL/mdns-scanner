//! Process-wide mirrors of config values read from hot paths that have no
//! `SharedConfig` handle. `SharedConfig` publishes them on construction and
//! after every modification.

use std::sync::atomic::{AtomicBool, Ordering};

use crate::AppConfig;

static EMOJIS: AtomicBool = AtomicBool::new(mds_default::UI_EMOJIS.value);

pub fn emojis() -> bool {
    EMOJIS.load(Ordering::Relaxed)
}

pub(crate) fn publish(cfg: &AppConfig) {
    EMOJIS.store(cfg.emojis(), Ordering::Relaxed);
}
