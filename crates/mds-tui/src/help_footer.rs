use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Paragraph},
};
use semver::Version;

use crate::table_pane::colors::TableColors;

fn info_text_line1<'a>() -> Vec<Span<'a>> {
    vec![
        Span::raw("<"),
        Span::styled("TAB", Style::new().fg(Color::Green)),
        Span::raw(">: Toggle pane"),
        Span::raw(" | <"),
        Span::styled("Spacebar/Enter", Style::new().fg(Color::Green)),
        Span::raw(">: Select"),
        Span::raw(" | <"),
        Span::styled(
            "←↓↑→/hjkl, PgUp/PgDn, Home/End",
            Style::new().fg(Color::Green),
        ),
        Span::raw(">: Navigate"),
        Span::raw(" | <"),
        Span::styled("+/-", Style::new().fg(Color::Green)),
        Span::raw(">: Increase/Decrease Pane size"),
    ]
}

fn info_text_line2<'a>() -> Vec<Span<'a>> {
    vec![
        Span::raw("<"),
        Span::styled("Q", Style::new().fg(Color::Green)),
        Span::raw(">: Quit"),
        Span::raw(" | <"),
        Span::styled("Shift+C", Style::new().fg(Color::Green)),
        Span::raw(">: Settings"),
        Span::raw(" | <"),
        Span::styled("Ctrl+R", Style::new().fg(Color::Green)),
        Span::raw(">: Refresh"),
        Span::raw(" | <"),
        Span::styled("Ctrl+F", Style::new().fg(Color::Green)),
        Span::raw(">: Search"),
        Span::raw(" | <"),
        Span::styled("v/g", Style::new().fg(Color::Green)),
        Span::raw(">: Increase/Decrease verbosity"),
    ]
}

pub(crate) struct HelpFooter {
    colors: TableColors,
    footer_title: String,
}

impl HelpFooter {
    pub(crate) fn new(version: &Version) -> Self {
        Self {
            colors: TableColors::default(),
            footer_title: format!("v{version}"),
        }
    }

    pub(crate) fn render(&self, frame: &mut Frame, area: Rect) {
        let info_footer = Paragraph::new(Text::from_iter([info_text_line1(), info_text_line2()]))
            .style(
                Style::new()
                    .fg(self.colors.row_fg)
                    .bg(self.colors.buffer_bg),
            )
            .centered()
            .block(
                Block::bordered()
                    .border_type(BorderType::Plain)
                    .border_style(Style::new().fg(self.colors.footer_border_color))
                    .title(Line::from(self.footer_title.clone()).centered()),
            );
        frame.render_widget(info_footer, area);
    }
}
