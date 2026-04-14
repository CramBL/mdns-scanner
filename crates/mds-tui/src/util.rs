use ratatui::layout::{Constraint, Flex, Layout, Rect};
use tui_textarea::TextArea;

/// Centers a [`Rect`] within another [`Rect`] using the provided [`Constraint`]s.
///
/// # Examples
///
/// ```rust
/// use ratatui::layout::{Constraint, Rect};
///
/// let area = Rect::new(0, 0, 100, 100);
/// let horizontal = Constraint::Percentage(20);
/// let vertical = Constraint::Percentage(30);
///
/// let centered = center(area, horizontal, vertical);
/// ```
pub(crate) fn center(area: Rect, horizontal: Constraint, vertical: Constraint) -> Rect {
    let [area] = Layout::horizontal([horizontal])
        .flex(Flex::Center)
        .areas(area);
    let [area] = Layout::vertical([vertical]).flex(Flex::Center).areas(area);
    area
}

pub(crate) fn text_edit_content_len(txt_edit: &TextArea) -> u16 {
    txt_edit
        .lines()
        .first()
        .map(|l| l.len())
        .unwrap_or_default() as u16
}
