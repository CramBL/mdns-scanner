use std::time::Duration;

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

/// Formats a measured or estimated span as e.g. `420ms`, `4.2s`, `1m 40s`, `2h 3m`.
pub(crate) fn format_duration(duration: Duration) -> String {
    let secs = duration.as_secs();
    if secs >= 3600 {
        format!("{h}h {m}m", h = secs / 3600, m = (secs % 3600) / 60)
    } else if secs >= 60 {
        format!("{m}m {s}s", m = secs / 60, s = secs % 60)
    } else if secs >= 1 {
        format!("{:.1}s", duration.as_secs_f32())
    } else {
        format!("{}ms", duration.as_millis())
    }
}

/// Formats how long ago something happened, in a single unit: `12s`, `3m`, `2h`.
pub(crate) fn format_age(age: Duration) -> String {
    let secs = age.as_secs();
    if secs >= 3600 {
        format!("{}h", secs / 3600)
    } else if secs >= 60 {
        format!("{}m", secs / 60)
    } else {
        format!("{secs}s")
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(Duration::from_millis(420), "420ms")]
    #[case(Duration::from_millis(4200), "4.2s")]
    #[case(Duration::from_secs(100), "1m 40s")]
    #[case(Duration::from_secs(7380), "2h 3m")]
    fn duration_formatting(#[case] duration: Duration, #[case] expected: &str) {
        assert_eq!(format_duration(duration), expected);
    }

    #[rstest]
    #[case(Duration::from_secs(12), "12s")]
    #[case(Duration::from_secs(190), "3m")]
    #[case(Duration::from_secs(7380), "2h")]
    fn age_formatting(#[case] age: Duration, #[case] expected: &str) {
        assert_eq!(format_age(age), expected);
    }
}
