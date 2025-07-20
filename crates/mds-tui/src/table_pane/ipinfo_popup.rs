use std::time::Instant;

use mds_ipinfo::IpInfo;
use ratatui::{
    Frame,
    layout::Constraint,
    style::{Color, Style, Stylize as _},
    text::{Line, Span},
    widgets::Paragraph,
};
use tui_popup::{Popup, SizedWrapper};

use crate::util;

#[derive(Default)]
pub(super) struct IpInfoPopUp {
    pub(super) is_open: bool,
}

impl IpInfoPopUp {
    pub(super) fn render(&self, frame: &mut Frame, info: Option<&IpInfo>) {
        if !self.is_open {
            return;
        }
        let Some(info) = info else {
            return;
        };
        let area = util::center(
            frame.area(),
            Constraint::Min(40),
            Constraint::Percentage(50),
        );

        let mut msg_lines = vec![];
        for line in info.names() {
            msg_lines.push(Line::styled(line.as_str(), Style::new().blue()));
        }

        let description = Span::raw("Updated ");
        let val = Span::styled(
            format!(
                "{:.0?}s",
                Instant::now()
                    .duration_since(info.last_updated)
                    .as_secs_f32()
            ),
            Style::new().yellow(),
        );
        msg_lines.push(Line::from(vec![description, val, Span::raw(" ago")]));

        if let Some(first_rtt) = info.first_rtt {
            let description = Span::raw("RTT on discover: ");
            let val = Span::styled(format!("{first_rtt:.2?}"), Style::new().yellow());
            msg_lines.push(Line::from(vec![description, val]));
        }
        if let Some(reached_by) = info.reached_by() {
            let description = Span::raw("Reached by: ");
            let val = Span::styled(reached_by.to_string(), Style::new().green());
            msg_lines.push(Line::from(vec![description, val]));
        }

        let description = Span::raw("Last known status: ");
        let val = Span::styled(
            info.last_known_status.to_string(),
            Style::new().fg(match info.last_known_status {
                mds_ipinfo::LastKnownStatus::Online => Color::Green,
                mds_ipinfo::LastKnownStatus::Offline => Color::Red,
            }),
        );
        msg_lines.push(Line::from(vec![description, val]));

        let text = msg_lines;
        let mut max_width = 0;
        for t in &text {
            if t.width() > max_width {
                max_width = t.width();
            }
        }
        let height = text.len();

        let paragraph = Paragraph::new(text);
        let sized_paragraph = SizedWrapper {
            inner: paragraph,
            width: max_width,
            height,
        };

        let title = vec![Span::raw(info.ip().to_string())];
        let popup = Popup::new(sized_paragraph)
            .title(title)
            .border_style(Style::new().blue())
            .style(Style::new().white());

        frame.render_widget(&popup, area);
    }
}
