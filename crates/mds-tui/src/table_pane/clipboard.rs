use std::time::{Duration, Instant};

use arboard::Clipboard;
use mds_ipinfo::{IpForHost, IpInfo};
use unicode_width::UnicodeWidthStr as _;

use crate::error_box::ErrorBox;

const COPY_FLASH_DURATION: Duration = Duration::from_millis(200);

/// Tracks sub-line selection state when a multi-line cell is targeted for copy.
///
/// The row is identified by `IpForHost` so that newly-discovered hosts inserting
/// themselves at a lower sort position cannot shift the selector onto the wrong row.
///
/// Cell lines are **snapshotted at open time**.  This keeps the display and row
/// height stable for the duration of the interaction: new hostnames / services
/// discovered while the selector is open are simply not shown until the user
/// dismisses and re-enters selection mode.  The cursor is therefore a plain
/// positional index into the snapshot (no text-search fallback required.)
///
/// A virtual "copy all" option sits one past the last snapshot line.
pub(super) struct SubLineSelector {
    pub(super) ip: IpForHost,
    pub(super) col: usize,
    lines: Vec<String>,
    selected: usize,
    /// Maximum display width of any snapshot line, used to freeze the column
    /// width at the value it had when the selector was opened.
    max_line_width: u16,
}

impl SubLineSelector {
    pub(super) fn new(ip: IpForHost, col: usize, lines: Vec<String>) -> Self {
        let max_line_width = lines.iter().map(|l| l.width() as u16).max().unwrap_or(0);
        Self {
            ip,
            col,
            lines,
            selected: 0,
            max_line_width,
        }
    }

    /// Advance the cursor.  Stops at the "copy all" virtual option.
    pub(super) fn next(&mut self) {
        if self.selected < self.lines.len() {
            self.selected += 1;
        }
    }

    pub(super) fn prev(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    /// `true` when the "copy all" virtual option is highlighted.
    pub(super) fn is_copy_all(&self) -> bool {
        self.selected == self.lines.len()
    }

    /// The text of the highlighted individual line, or `None` when "copy all"
    /// is selected.
    pub(super) fn selected_line(&self) -> Option<&str> {
        self.lines.get(self.selected).map(String::as_str)
    }

    pub(super) fn selected_index(&self) -> usize {
        self.selected
    }

    pub(super) fn lines(&self) -> &[String] {
        &self.lines
    }

    /// Maximum display width of any snapshotted line, used to keep the column
    /// layout stable while new (possibly longer) names arrive during selection.
    pub(super) fn max_line_width(&self) -> u16 {
        self.max_line_width
    }

    /// Whether this selector targets the given row, regardless of column.
    pub(super) fn matches_ip(&self, info: &IpInfo) -> bool {
        self.ip.shares_ip_with(&info.ip())
    }

    pub(super) fn matches_ip_and_col(&self, info: &IpInfo, col: usize) -> bool {
        self.ip.shares_ip_with(&info.ip()) && self.col == col
    }
}

/// Tracks which cell was most recently copied so it can be briefly flashed.
///
/// The row is identified by `IpForHost` so the flash remains correct even if
/// newly-discovered hosts shift the table's sort order while the flash window
/// is still open.
///
/// For sub-line copies the copied line is identified by its **text content**,
/// not a positional index.  This means a line inserted above the copied one
/// between selection and render time does not cause the flash to appear on the
/// wrong line.
pub(super) struct CopiedCell {
    ip: IpForHost,
    col: usize,
    /// When `Some`, only the line with this text flashes.
    /// When `None`, the entire cell flashes (single-line copy or "copy all").
    line_text: Option<String>,
    time: Instant,
}

impl CopiedCell {
    pub(crate) fn new(ip: IpForHost, col: usize) -> Self {
        Self {
            ip,
            col,
            line_text: None,
            time: Instant::now(),
        }
    }

    pub(crate) fn new_sub_line(ip: IpForHost, col: usize, line_text: String) -> Self {
        Self {
            ip,
            col,
            line_text: Some(line_text),
            time: Instant::now(),
        }
    }

    pub(crate) fn copied_recently(&self) -> bool {
        Instant::now().duration_since(self.time) < COPY_FLASH_DURATION
    }

    pub(super) fn matches_ip_and_col(&self, info: &IpInfo, col: usize) -> bool {
        self.ip.shares_ip_with(&info.ip()) && self.col == col
    }

    /// The text of the copied line for a sub-line copy, or `None` for a
    /// whole-cell copy (single line or "copy all").
    pub(super) fn line_text(&self) -> Option<&str> {
        self.line_text.as_deref()
    }
}

pub(super) enum MdsClipboard {
    Supported(Clipboard),
    NotSupported {
        error: Box<str>,
    },
    /// Used in tests to avoid requiring a real clipboard (no X11/Wayland on CI).
    #[cfg(any(test, feature = "test-utils"))]
    Stub,
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

    /// Returns an error if clipboard access is known to be unavailable.
    /// Used as an early-exit check before entering sub-line selection mode,
    /// so the user gets feedback before navigating rather than after confirming.
    pub(crate) fn check_supported(&self) -> Result<(), ErrorBox> {
        match self {
            MdsClipboard::Supported(_) => Ok(()),
            MdsClipboard::NotSupported { error } => Err(ErrorBox::new(error)),
            #[cfg(any(test, feature = "test-utils"))]
            MdsClipboard::Stub => Ok(()),
        }
    }

    pub(crate) fn set_text(&mut self, text: String) -> Result<(), ErrorBox> {
        match self {
            MdsClipboard::Supported(c) => c
                .set_text(text)
                .map_err(|e| format!("Failed setting clipboard content: {e}").into()),
            MdsClipboard::NotSupported { error } => Err(ErrorBox::new(error)),
            #[cfg(any(test, feature = "test-utils"))]
            MdsClipboard::Stub => Ok(()),
        }
    }

    #[cfg(any(test, feature = "test-utils"))]
    pub(super) fn stub() -> Self {
        MdsClipboard::Stub
    }
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;

    use super::*;

    fn make_ip() -> IpForHost {
        IpForHost::V4(Ipv4Addr::new(192, 168, 1, 1))
    }

    fn make_selector(lines: Vec<&str>) -> SubLineSelector {
        SubLineSelector::new(make_ip(), 1, lines.into_iter().map(str::to_owned).collect())
    }

    #[test]
    fn test_snapshot_is_frozen() {
        let lines = vec!["alpha.local", "beta.local"];
        let sel = make_selector(lines.clone());
        let expected: Vec<String> = lines.into_iter().map(str::to_owned).collect();
        assert_eq!(sel.lines(), expected.as_slice());
    }

    #[test]
    fn test_navigation_within_snapshot() {
        let mut sel = make_selector(vec!["a", "b", "c"]);

        assert_eq!(sel.selected_index(), 0);
        assert_eq!(sel.selected_line(), Some("a"));
        assert!(!sel.is_copy_all());

        sel.next();
        assert_eq!(sel.selected_index(), 1);
        assert_eq!(sel.selected_line(), Some("b"));

        sel.next();
        assert_eq!(sel.selected_index(), 2);
        assert_eq!(sel.selected_line(), Some("c"));
        assert!(!sel.is_copy_all());

        sel.next();
        assert_eq!(sel.selected_index(), 3);
        assert!(sel.is_copy_all());
        assert_eq!(sel.selected_line(), None);
    }

    #[test]
    fn test_next_stops_at_copy_all() {
        let mut sel = make_selector(vec!["a", "b"]);
        sel.next();
        sel.next(); // now at copy-all
        sel.next(); // should not advance past copy-all
        assert_eq!(sel.selected_index(), 2);
        assert!(sel.is_copy_all());
    }

    #[test]
    fn test_prev_from_copy_all() {
        let mut sel = make_selector(vec!["a", "b"]);
        sel.next();
        sel.next(); // at copy-all
        sel.prev(); // back to "b"
        assert!(!sel.is_copy_all());
        assert_eq!(sel.selected_line(), Some("b"));
    }

    #[test]
    fn test_prev_stops_at_zero() {
        let mut sel = make_selector(vec!["a", "b"]);
        sel.prev(); // already at 0, should stay
        assert_eq!(sel.selected_index(), 0);
    }

    /// Navigation is bounded by the snapshot even if the caller does not pass
    /// updated line counts.  This verifies that new hostnames discovered for
    /// the same IP after the selector was opened cannot shift the cursor.
    #[test]
    fn test_cursor_bounded_by_snapshot_not_live_data() {
        let mut sel = make_selector(vec!["alpha.local", "beta.local"]);
        // Simulate navigating to the last snapshot line then to copy-all
        sel.next();
        sel.next();
        assert!(sel.is_copy_all());
        // Even if "gamma.local" was inserted into live data, next() stays put
        sel.next();
        assert!(sel.is_copy_all());
        assert_eq!(sel.selected_index(), 2);
    }

    #[test]
    fn test_not_supported_check_returns_error() {
        let cb = MdsClipboard::NotSupported {
            error: "no display".into(),
        };
        assert!(cb.check_supported().is_err());
    }

    #[test]
    fn test_not_supported_set_text_returns_error() {
        let mut cb = MdsClipboard::NotSupported {
            error: "no display".into(),
        };
        assert!(cb.set_text("hello".to_owned()).is_err());
    }
}
