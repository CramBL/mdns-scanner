use std::time::Instant;

use mds_ipinfo::IpInfo;
use mds_keybindings::{Action, KeyBindings};
use ratatui::{
    Frame,
    layout::Constraint,
    style::Modifier,
    text::{Line, Span},
    widgets::Paragraph,
};
use tui_popup::{KnownSizeWrapper, Popup};

use crate::{format, table_pane::TableColors, util};

#[derive(Default)]
pub(super) struct IpInfoPopUp {
    pub(super) is_open: bool,
}

impl IpInfoPopUp {
    pub(super) fn render(
        &self,
        frame: &mut Frame,
        info: Option<&IpInfo>,
        theme: &TableColors,
        keymap: &KeyBindings,
    ) {
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
            msg_lines.push(Line::styled(line.display_name(), theme.gauge_accent()));
        }

        let description = Span::styled("Updated ", theme.row());
        let val = Span::styled(
            format::format_age(Instant::now().duration_since(info.last_updated)),
            theme.log_warn(),
        );
        msg_lines.push(Line::from(vec![
            description,
            val,
            Span::styled(" ago", theme.row()),
        ]));

        if let Some(rtt_stats) = info.rtt {
            let on_discover = rtt_stats.on_discover();
            let latest = rtt_stats.latest();
            let description = Span::styled("RTT on discover/latest: ", theme.row());
            let val_first = Span::styled(format!("{on_discover:.1?} "), theme.log_warn());
            let val_latest = Span::styled(format!("{latest:.1?}"), theme.title());
            msg_lines.push(Line::from(vec![description, val_first, val_latest]));

            let description = Span::styled("RTT min/avg/max:", theme.row());
            let min = rtt_stats.min;
            let avg = rtt_stats.avg;
            let max = rtt_stats.max;
            let min_val = Span::styled(format!(" {min:.1?}"), theme.success());
            let avg_val = Span::styled(format!(" {avg:.1?}"), theme.log_warn());
            let max_val = Span::styled(format!(" {max:.1?}"), theme.log_err());
            msg_lines.push(Line::from(vec![description, min_val, avg_val, max_val]));
        }

        if let Some(reached_by) = info.reached_by() {
            let description = Span::styled("Reached by: ", theme.row());
            let val = Span::styled(reached_by.to_string(), theme.success());
            msg_lines.push(Line::from(vec![description, val]));
        }

        let description = Span::styled("Last known status: ", theme.row());
        let status_style = if matches!(info.last_known_status, mds_ipinfo::LastKnownStatus::Online)
        {
            theme.success()
        } else {
            theme.log_err()
        };
        let val = Span::styled(info.last_known_status.to_string(), status_style);
        msg_lines.push(Line::from(vec![description, val]));

        if let Some(port_scan) = info.port_scan_result() {
            let (open_ports, open_ports_style) = if port_scan.open_ports().is_empty() {
                ("none".to_owned(), theme.row().add_modifier(Modifier::DIM))
            } else {
                (port_scan.open_ports_comma_separated(), theme.success())
            };
            msg_lines.push(Line::from(vec![
                Span::styled("Open ports: ", theme.row()),
                Span::styled(open_ports, open_ports_style),
                Span::styled(" (", theme.row()),
                Span::styled(format::format_age(port_scan.age()), theme.log_warn()),
                Span::styled(" ago)", theme.row()),
            ]));
        }

        let hints = util::key_hint_footer(
            theme,
            &[(
                keymap.get_key_display_for_action(Action::PortScan),
                "port scan",
            )],
        );

        let mut text = msg_lines;
        let mut max_width = 0;
        for t in &text {
            if t.width() > max_width {
                max_width = t.width();
            }
        }
        max_width = max_width.max(hints.width());
        text.push(Line::styled("─".repeat(max_width), theme.border()));
        text.push(hints);
        let height = text.len();

        let paragraph = Paragraph::new(text).style(theme.base());
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
        let title = vec![Span::styled(title, theme.gauge_accent())];

        let popup = Popup::new(sized_paragraph)
            .title(title)
            .border_style(theme.gauge_accent())
            .style(theme.base());

        frame.render_widget(&popup, area);
    }
}
