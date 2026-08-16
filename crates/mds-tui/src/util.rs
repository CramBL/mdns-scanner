use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::text::{Line, Span};
use tui_textarea::TextArea;

use crate::table_pane::TableColors;

/// Centers a [`Rect`] within another [`Rect`] using the provided [`Constraint`]s.
///
/// # Examples
///
/// ```rust
/// use ratatui::layout::{Constraint, Rect};
/// use mds_tui::util::center;
///
/// let area = Rect::new(0, 0, 100, 100);
/// let horizontal = Constraint::Percentage(20);
/// let vertical = Constraint::Percentage(30);
///
/// let centered = center(area, horizontal, vertical);
/// ```
pub fn center(area: Rect, horizontal: Constraint, vertical: Constraint) -> Rect {
    let [area] = Layout::horizontal([horizontal])
        .flex(Flex::Center)
        .areas(area);
    let [area] = Layout::vertical([vertical]).flex(Flex::Center).areas(area);
    area
}

/// A footer of `<KEY>: label` hints joined by ` | `, styled like the config editor footer:
/// the key uses [`TableColors::config_doc`] and the surrounding markup uses [`TableColors::title`].
pub(crate) fn key_hint_footer(theme: &TableColors, hints: &[(String, &str)]) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    for (index, (key, label)) in hints.iter().enumerate() {
        if index > 0 {
            spans.push(Span::styled(" | ", theme.title()));
        }
        spans.push(Span::styled("<", theme.title()));
        spans.push(Span::styled(key.clone(), theme.config_doc()));
        spans.push(Span::styled(format!(">: {label}"), theme.title()));
    }
    Line::from(spans)
}

pub(crate) fn text_edit_content_len(txt_edit: &TextArea) -> u16 {
    txt_edit
        .lines()
        .first()
        .map(|l| l.len())
        .unwrap_or_default() as u16
}
