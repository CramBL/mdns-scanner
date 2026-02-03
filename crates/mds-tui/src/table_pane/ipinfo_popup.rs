use std::time::Instant;

use mds_ipinfo::IpInfo;
use ratatui::{
    Frame,
    layout::Constraint,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
};
use tui_popup::{KnownSizeWrapper, Popup};

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

        if let Some(rtt_stats) = info.rtt {
            let on_discover = rtt_stats.on_discover();
            let latest = rtt_stats.latest();
            let description = Span::raw("RTT on discover/latest: ");
            let val_first = Span::styled(format!("{on_discover:.1?} "), Style::new().yellow());
            let val_latest = Span::styled(format!("{latest:.1?}"), Style::new().white());
            msg_lines.push(Line::from(vec![description, val_first, val_latest]));

            let description = Span::raw("RTT min/avg/max:");
            let min = rtt_stats.min;
            let avg = rtt_stats.avg;
            let max = rtt_stats.max;
            let min_val = Span::styled(format!(" {min:.1?}"), Style::new().light_green());
            let avg_val = Span::styled(format!(" {avg:.1?}"), Style::new().yellow());
            let max_val = Span::styled(format!(" {max:.1?}"), Style::new().red());
            msg_lines.push(Line::from(vec![description, min_val, avg_val, max_val]));
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
        let sized_paragraph = KnownSizeWrapper {
            inner: paragraph,
            width: max_width,
            height,
        };

        let title = match info.ip() {
            mds_ipinfo::IpForHost::V4(ipv4) => ipv4.to_string(),
            mds_ipinfo::IpForHost::V6(ipv6) => ipv6.to_string(),
            mds_ipinfo::IpForHost::V4andV6((ipv4, ipv6)) => format!("{ipv4}/{ipv6}"),
        };
        let title = vec![Span::raw(title)];

        let popup = Popup::new(sized_paragraph)
            .title(title)
            .border_style(Style::new().blue())
            .style(Style::new().white());

        frame.render_widget(&popup, area);
    }
}
