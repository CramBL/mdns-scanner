use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::num::NonZeroUsize;
use std::sync::Arc;

use crate::timeouts::Timeouts;
use crate::{AppConfig, scan};

/// A thread-safe, cloneable handle to the app configuration.
///
/// This wrapper provides a safe and ergonomic API for accessing the [AppConfig]
/// from multiple threads by managing the underlying `RwLock`.
#[derive(Clone, Default)]
pub struct SharedConfig(Arc<RwLock<AppConfig>>);

impl SharedConfig {
    /// Creates a new shareable [AppConfig].
    pub fn new(config: AppConfig) -> Self {
        Self(Arc::new(RwLock::new(config)))
    }

    /// Acquires a read lock, returning a guard.
    ///
    /// The lock is released when the returned `RwLockReadGuard` is dropped.
    /// This is useful when the configuration data needs to be accessed across
    /// a short-lived scope. For single-value reads, prefer the direct accessors.
    /// For complex, multi-value reads within a single function, consider `with_read`.
    pub fn read(&self) -> RwLockReadGuard<'_, AppConfig> {
        self.0.read()
    }

    /// Acquires a write lock, returning a guard.
    ///
    /// The lock is released when the returned `RwLockWriteGuard` is dropped.
    /// Be mindful of the guard's lifetime to avoid holding the lock for too long.
    /// For most modifications, the closure-based `modify` method is preferred as it
    /// guarantees the lock is released immediately.
    ///
    /// # Example
    /// ```
    /// // self.cfg is a SharedConfig instance
    /// self.cfg.write().ui.compact = true;
    /// // Lock is released here at the end of the statement.
    /// ```
    pub fn write(&self) -> RwLockWriteGuard<'_, AppConfig> {
        self.0.write()
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
        f(&self.0.read())
    }

    /// Modifies the configuration within a write-locked scope using a closure.
    ///
    /// This is the recommended approach for most mutations, especially complex ones,
    /// as it ensures the write lock is held for the shortest possible duration.
    ///
    /// # Example
    /// ```
    /// self.cfg.modify(|cfg| {
    ///     cfg.ui.log_limit = 1000;
    ///     cfg.scan.service_discovery = false;
    /// });
    /// ```
    pub fn modify<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut AppConfig) -> R,
    {
        f(&mut self.0.write())
    }

    pub fn timeout_settings(&self) -> Timeouts {
        self.0.read().timeouts
    }

    pub fn compact(&self) -> bool {
        self.0.read().ui.compact
    }

    pub fn hide_bare_ips(&self) -> bool {
        self.0.read().ui.hide_bare_ips
    }

    pub fn log_limit(&self) -> NonZeroUsize {
        let limit = self.0.read().ui.log_limit.max(1) as usize;
        // This unwrap is safe due to the .max(1) check above.
        NonZeroUsize::new(limit).unwrap()
    }

    pub fn service_discovery_enabled(&self) -> bool {
        self.0.read().scan.service_discovery
    }

    pub fn scan_io_threads(&self) -> scan::IoThreads {
        self.0.read().scan.io_threads
    }

    pub fn scan_tcp_ports(&self) -> Vec<u16> {
        self.0.read().scan_tcp_ports()
    }

    pub fn iface_include_docker(&self) -> bool {
        self.0.read().iface_include_docker()
    }
}
