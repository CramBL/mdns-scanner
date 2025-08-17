use std::time::{Duration, Instant};

use arboard::Clipboard;

use crate::error_box::ErrorBox;

pub(super) struct CopiedCell {
    row: usize,
    col: usize,
    time: Instant,
}

impl CopiedCell {
    pub(crate) fn new(row: usize, col: usize) -> Self {
        Self {
            row,
            col,
            time: Instant::now(),
        }
    }

    pub(crate) fn copied_recently(&self) -> bool {
        Instant::now().duration_since(self.time) < Duration::from_millis(200)
    }

    pub(super) fn matches_coord(&self, row: usize, col: usize) -> bool {
        self.row == row && self.col == col
    }
}

pub(super) enum MdsClipboard {
    Supported(Clipboard),
    NotSupported { error: Box<str> },
}

impl MdsClipboard {
    pub(crate) fn new() -> Self {
        match Clipboard::new() {
            Ok(c) => MdsClipboard::Supported(c),
            Err(e) => {
                log::error!(
                    "Clipboard is not supported on this platform, copy actions will fail: {e}"
                );
                MdsClipboard::NotSupported {
                    error: format!("No clipboard support: {e}").into_boxed_str(),
                }
            }
        }
    }

    pub(crate) fn get(&mut self) -> Result<&mut Clipboard, ErrorBox> {
        match self {
            MdsClipboard::Supported(clipboard) => Ok(clipboard),
            MdsClipboard::NotSupported { error } => Err(ErrorBox::new(error)),
        }
    }
}
