use mds_log::prelude::*;
use mds_util::refresh::RefreshListener;
use std::sync::mpsc::{self, Receiver};

use ratatui::{prelude::*, widgets::*};

pub(crate) struct LogPane {
    log_db: LogDb,
    logger: Logger,
    rx_logs: Receiver<LogMessage>,
    refresh_listener: RefreshListener,
    vertical_scroll_state: ScrollbarState,
    horizontal_scroll_state: ScrollbarState,
    vertical_scroll: usize,
    horizontal_scroll: usize,
    current_frame_area: Rect,
}

impl LogPane {
    const ERR_COLOR: Color = Color::Red;
    const WARN_COLOR: Color = Color::Yellow;
    const INFO_COLOR: Color = Color::White;
    const DEBUG_COLOR: Color = Color::Cyan;
    const TRACE_COLOR: Color = Color::Blue;

    pub fn new(refresh_listener: RefreshListener) -> Self {
        let (tx_logs, rx_logs) = mpsc::channel();
        let logger = Logger::new(tx_logs, LogLevel::default());

        Self {
            log_db: LogDb::default(),
            logger,
            rx_logs,
            refresh_listener,
            vertical_scroll_state: ScrollbarState::default(),
            horizontal_scroll_state: ScrollbarState::default(),
            vertical_scroll: 0,
            horizontal_scroll: 0,
            current_frame_area: Rect::ZERO,
        }
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, in_focus: bool) {
        let logs = self.log_db.all_logs(self.log_level());
        let mut lines: Vec<Line<'_>> = vec![];
        for msg in logs {
            match msg {
                LogMessage::Error(s) => {
                    lines.push(Line::from(s.as_ref()).fg(Self::ERR_COLOR));
                }
                LogMessage::Warn(s) => lines.push(Line::from(s.as_ref()).yellow()),
                LogMessage::Info(s) => lines.push(Line::from(s.as_ref())),
                LogMessage::Debug(s) => lines.push(Line::from(s.as_ref()).cyan()),
                LogMessage::Trace(s) => lines.push(Line::from(s.as_ref()).blue()),
            }
        }
        let content_len = lines.len();

        self.vertical_scroll_state = self.vertical_scroll_state.content_length(content_len);
        self.horizontal_scroll_state = self
            .horizontal_scroll_state
            .content_length(self.log_db.longest_message());

        let log_block = self.pane_block(in_focus, content_len as u16);

        let paragraph = Paragraph::new(lines)
            .block(log_block)
            .scroll((self.vertical_scroll as u16, self.horizontal_scroll as u16));

        frame.render_widget(paragraph, area);
        // Crashes if area is 0, but if area is tiny it still doesn't make sense to render the scrollbar
        if area.height > 1 {
            frame.render_stateful_widget(
                Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(Some("↑"))
                    .end_symbol(Some("↓")),
                area,
                &mut self.vertical_scroll_state,
            );
            frame.render_stateful_widget(
                Scrollbar::new(ScrollbarOrientation::HorizontalBottom)
                    .thumb_symbol("🬋")
                    .begin_symbol(None)
                    .end_symbol(None),
                area,
                &mut self.horizontal_scroll_state,
            );
        }
    }

    fn pane_block(&self, in_focus: bool, content_len: u16) -> Block<'_> {
        let block_border_symbol = if in_focus {
            symbols::border::PLAIN
        } else {
            symbols::border::EMPTY
        };

        let title = self.pane_title(content_len);

        Block::bordered()
            .title(title)
            .border_set(block_border_symbol)
    }

    fn pane_title(&self, content_len: u16) -> Vec<Span<'_>> {
        let log_level_span = match self.log_level() {
            LogLevel::Error => Span::styled(
                LogLevel::Error.to_string(),
                Style::new().fg(Self::ERR_COLOR),
            ),
            LogLevel::Warn => Span::styled(
                LogLevel::Warn.to_string(),
                Style::new().fg(Self::WARN_COLOR),
            ),
            LogLevel::Info => Span::styled(
                LogLevel::Info.to_string(),
                Style::new().fg(Self::INFO_COLOR),
            ),
            LogLevel::Debug => Span::styled(
                LogLevel::Debug.to_string(),
                Style::new().fg(Self::DEBUG_COLOR),
            ),
            LogLevel::Trace => Span::styled(
                LogLevel::Trace.to_string(),
                Style::new().fg(Self::TRACE_COLOR),
            ),
        };

        vec![
            Span::raw("Log Level: "),
            log_level_span,
            format!(", showing {content_len} msgs (max: {})", LogDb::MAX_LOGS).into(),
        ]
    }

    pub fn get_logger_clone(&self) -> Logger {
        self.logger.clone()
    }

    pub(crate) fn recv_new_logs(&mut self) {
        while let Ok(l) = self.rx_logs.try_recv() {
            self.log_db.push(l);
        }
        if self.refresh_listener.do_refresh() {
            self.log_db.clear();
            self.scroll_to_start();
        }
    }

    pub(crate) fn increase_verbosity(&self) {
        self.logger.increase_verbosity();
    }

    pub(crate) fn decrease_verbosity(&self) {
        self.logger.decrease_verbosity();
    }

    pub(crate) fn log_level(&self) -> LogLevel {
        self.logger.verbosity()
    }

    pub(crate) fn scroll_to_start(&mut self) {
        self.vertical_scroll = 0;
        self.vertical_scroll_state = self.vertical_scroll_state.position(self.vertical_scroll);
        self.horizontal_scroll = 0;
        self.horizontal_scroll_state = self
            .horizontal_scroll_state
            .position(self.horizontal_scroll);
        self.log_db.unfreeze();
    }

    pub(crate) fn scroll_to_end(&mut self) {
        self.vertical_scroll = self.log_db.len() - 1;
        self.vertical_scroll_state = self.vertical_scroll_state.position(self.vertical_scroll);
        self.log_db.freeze();
    }

    pub(crate) fn scroll_down(&mut self) {
        self.vertical_scroll = self.vertical_scroll.saturating_add(1);
        self.vertical_scroll_state = self.vertical_scroll_state.position(self.vertical_scroll);
        self.log_db.freeze();
    }

    pub(crate) fn scroll_up(&mut self) {
        self.vertical_scroll = self.vertical_scroll.saturating_sub(1);
        self.vertical_scroll_state = self.vertical_scroll_state.position(self.vertical_scroll);
        if self.vertical_scroll == 0 {
            self.log_db.unfreeze();
        }
    }

    pub(crate) fn scroll_left(&mut self) {
        self.horizontal_scroll = self.horizontal_scroll.saturating_sub(1);
        self.horizontal_scroll_state = self
            .horizontal_scroll_state
            .position(self.horizontal_scroll);
    }

    pub(crate) fn scroll_right(&mut self) {
        self.horizontal_scroll = self.horizontal_scroll.saturating_add(1);
        self.horizontal_scroll_state = self
            .horizontal_scroll_state
            .position(self.horizontal_scroll);
    }

    pub(crate) fn scroll_page_up(&mut self) {
        let window_height = self.current_frame_area.height;
        let lines_in_window = window_height.saturating_sub(2);
        for _ in 0..lines_in_window {
            self.scroll_up();
        }
    }

    pub(crate) fn scroll_page_down(&mut self) {
        let window_height = self.current_frame_area.height;
        let lines_in_window = window_height.saturating_sub(2);
        for _ in 0..lines_in_window {
            self.scroll_down();
        }
    }

    pub(crate) fn set_current_frame_area(&mut self, area: Rect) {
        self.current_frame_area = area;
    }
}
