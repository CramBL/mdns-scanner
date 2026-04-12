use parking_lot::{RwLock, RwLockReadGuard};
use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};

use crate::AppConfig;

#[derive(Default)]
struct SharedConfigInner {
    config: RwLock<AppConfig>,
    config_gen: AtomicU32,
}

/// A thread-safe, cloneable handle to the app configuration.
///
/// This wrapper provides a safe and ergonomic API for accessing the [AppConfig]
/// from multiple threads by managing the underlying `RwLock`.
#[derive(Clone, Default)]
pub struct SharedConfig(Arc<SharedConfigInner>);

impl SharedConfig {
    /// Creates a new shareable [AppConfig].
    pub fn new(config: AppConfig) -> Self {
        Self(Arc::new(SharedConfigInner {
            config: RwLock::new(config),
            config_gen: AtomicU32::new(0),
        }))
    }

    /// Acquires a read lock, returning a guard.
    ///
    /// The lock is released when the returned `RwLockReadGuard` is dropped.
    /// For one-time multi-field reads within a single scope, consider `with_read`.
    pub fn read(&self) -> RwLockReadGuard<'_, AppConfig> {
        self.0.config.read()
    }

    /// Executes a closure with a read-locked reference to the configuration.
    ///
    /// This is useful for performing multiple read operations atomically without
    /// needing to manually manage a lock guard.
    ///
    /// # Example
    /// ```
    /// let (compact, hide_ips) = self.cfg.with_read(|cfg| {
    ///     (cfg.ui.compact, cfg.ui.hide_bare_ips)
    /// });
    /// ```
    pub fn with_read<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&AppConfig) -> R,
    {
        f(&self.0.config.read())
    }

    /// Modifies the configuration within a write-locked scope, eagerly recompiles
    /// interface ignore patterns so read paths never need `&mut AppConfig`, then
    /// bumps the config version counter so consumers can detect the change with a
    /// cheap atomic load instead of acquiring a read lock on every frame.
    pub fn modify<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut AppConfig) -> R,
    {
        let result = {
            let mut guard = self.0.config.write();
            let r = f(&mut guard);
            // Eagerly recompile so iface_ignore_patterns() is always valid via &self.
            let _ = guard.interfaces.compile_ignore_patterns();
            r
        };
        self.0.config_gen.fetch_add(1, Ordering::Release);
        result
    }

    /// Returns the current config generation counter.
    ///
    /// Compare against a locally cached value; if they differ, re-read
    /// whichever config fields the consumer cares about.
    pub fn config_version(&self) -> u32 {
        self.0.config_gen.load(Ordering::Acquire)
    }
}
