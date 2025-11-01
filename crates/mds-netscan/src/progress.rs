use parking_lot::Mutex;
use std::sync::Arc;

#[derive(Default, Clone)]
pub struct ScannerProgress {
    status: Arc<Mutex<(u32, u32)>>,
}

impl ScannerProgress {
    /// Get scanner progress 0-1
    pub fn progress(&self) -> f32 {
        let (scanned, total) = self.progress_scanned_total();
        scanned as f32 / total as f32
    }

    /// Returns the scanned count and the total count
    pub fn progress_scanned_total(&self) -> (u32, u32) {
        *self.status.lock()
    }

    /// Mark the scanner progress as started
    pub(crate) fn start(&self, total: u32) {
        let mut status = self.status.lock();
        *status = (0, total);
        log::error!("Scanner progress STARTED: {}/{}", status.0, status.0);
    }

    /// Mark the scanner progress as finished
    pub(crate) fn finish(&self) {
        let mut status = self.status.lock();
        status.0 = status.1;
        log::error!("Scanner progress FINISHED: {}/{}", status.0, status.0);
    }

    /// Update the scanner progress with a new `scanned` count
    pub(crate) fn update(&self, scanned: u32) {
        let mut status = self.status.lock();
        log::error!("Updating scanner progress: {}/{}", status.0, status.1);
        debug_assert!(status.0 < scanned);
        status.0 = scanned;
    }
}

#[cfg(test)]
mod tests {
    use std::thread;

    use super::*;

    #[test]
    fn test_initial_progress() {
        let progress = ScannerProgress::default();
        assert!(progress.progress().is_nan());
    }

    #[test]
    fn test_start_and_progress() {
        let progress = ScannerProgress::default();
        progress.start(100);
        assert_eq!(progress.progress(), 0.0);
    }

    #[test]
    fn test_update_progress() {
        let progress = ScannerProgress::default();
        progress.start(100);
        progress.update(50);
        assert_eq!(progress.progress(), 0.5);
    }

    #[test]
    fn test_finish_progress() {
        let progress = ScannerProgress::default();
        progress.start(100);
        progress.update(50);
        progress.finish();
        assert_eq!(progress.progress(), 1.0);
    }

    #[test]
    fn test_clone() {
        let progress = ScannerProgress::default();
        progress.start(100);
        progress.update(50);

        let cloned = progress.clone();
        assert_eq!(cloned.progress(), 0.5);

        progress.update(75);
        assert_eq!(cloned.progress(), 0.75);
    }

    #[test]
    fn test_thread_update() {
        let progress = ScannerProgress::default();
        progress.start(100);

        let progress_clone = progress.clone();
        let handle = thread::Builder::new()
            .name("test_scanner_progress".into())
            .spawn(move || {
                progress_clone.update(50);
            })
            .unwrap();

        handle.join().unwrap();
        assert_eq!(progress.progress(), 0.5);
    }
}
