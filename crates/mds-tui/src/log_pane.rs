use mds_log::prelude::*;
use mds_util::refresh::RefreshListener;
use std::{num::NonZeroUsize, sync::mpsc::Receiver};

use ratatui::{prelude::*, widgets::*};

use crate::table_pane::TableColors;

pub(crate) struct LogPane {
    log_db: LogDb,
    logger: Logger,
    log_rx: Receiver<LogMessage>,
    refresh_listener: RefreshListener,
    vertical_scroll_state: ScrollbarState,
    horizontal_scroll_state: ScrollbarState,
    vertical_scroll: usize,
    horizontal_scroll: usize,
    current_frame_area: Rect,
    log_limit: NonZeroUsize,
}

impl LogPane {
    pub fn new(
        refresh_listener: RefreshListener,
        log_limit: NonZeroUsize,
        (logger, log_rx): (Logger, Receiver<LogMessage>),
    ) -> Self {
        Self {
            log_db: LogDb::new(log_limit),
            logger,
            log_rx,
            refresh_listener,
            vertical_scroll_state: ScrollbarState::default(),
            horizontal_scroll_state: ScrollbarState::default(),
            vertical_scroll: 0,
            horizontal_scroll: 0,
            current_frame_area: Rect::ZERO,
            log_limit,
        }
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, in_focus: bool, theme: &TableColors) {
        let logs = self.log_db.all_logs(self.log_level());
        let mut lines: Vec<Line<'_>> = vec![];
        for msg in logs {
            match msg {
                LogMessage::Error(s) => lines.push(Line::from(s.as_ref()).style(theme.log_err())),
                LogMessage::Warn(s) => lines.push(Line::from(s.as_ref()).style(theme.log_warn())),
                LogMessage::Info(s) => lines.push(Line::from(s.as_ref()).style(theme.log_info())),
                LogMessage::Debug(s) => lines.push(Line::from(s.as_ref()).style(theme.log_debug())),
                LogMessage::Trace(s) => lines.push(Line::from(s.as_ref()).style(theme.log_trace())),
            }
        }
        let content_len = lines.len();

        self.vertical_scroll_state = self.vertical_scroll_state.content_length(content_len);
        self.horizontal_scroll_state = self
            .horizontal_scroll_state
            .content_length(self.log_db.longest_message());

        let log_block = self.pane_block(in_focus, content_len as u16, theme);

        let paragraph = Paragraph::new(lines)
            .block(log_block)
            .style(theme.log_bg())
            .scroll((self.vertical_scroll as u16, self.horizontal_scroll as u16));

        frame.render_widget(paragraph, area);
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

    fn pane_block<'a>(&self, in_focus: bool, content_len: u16, theme: &TableColors) -> Block<'a> {
        let block_border_symbol = if in_focus {
            symbols::border::PLAIN
        } else {
            symbols::border::EMPTY
        };

        let title = self.pane_title(content_len, theme);

        Block::bordered()
            .title(title)
            .border_set(block_border_symbol)
            .border_style(theme.border())
    }

    fn pane_title<'a>(&self, content_len: u16, theme: &TableColors) -> Vec<Span<'a>> {
        let log_level_span = match self.log_level() {
            LogLevel::Error => Span::styled(LogLevel::Error.to_string(), theme.log_err()),
            LogLevel::Warn => Span::styled(LogLevel::Warn.to_string(), theme.log_warn()),
            LogLevel::Info => Span::styled(LogLevel::Info.to_string(), theme.log_info()),
            LogLevel::Debug => Span::styled(LogLevel::Debug.to_string(), theme.log_debug()),
            LogLevel::Trace => Span::styled(LogLevel::Trace.to_string(), theme.log_trace()),
        };

        vec![
            Span::styled("Log Level: ", theme.title()),
            log_level_span,
            Span::styled(
                format!(", showing {content_len} msgs (max: {})", self.log_limit),
                theme.title(),
            ),
        ]
    }

    pub(crate) fn recv_new_logs(&mut self) {
        while let Ok(l) = self.log_rx.try_recv() {
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
        self.vertical_scroll = self.end_position();
        self.vertical_scroll_state = self.vertical_scroll_state.position(self.vertical_scroll);
        self.log_db.freeze();
    }

    pub(crate) fn scroll_down(&mut self) {
        self.vertical_scroll = self
            .vertical_scroll
            .saturating_add(1)
            .min(self.end_position() + 1);
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
        for _ in 0..self.lines_in_window() {
            self.scroll_up();
        }
    }

    pub(crate) fn scroll_page_down(&mut self) {
        for _ in 0..self.lines_in_window() {
            self.scroll_down();
        }
    }

    // The number of lines the current frame height covers
    fn lines_in_window(&self) -> u16 {
        self.current_frame_area.height.saturating_sub(2)
    }

    // Find the position that would place the oldest log message at the last line
    fn end_position(&self) -> usize {
        let log_num = self.log_db.len();
        let lines_in_window = self.lines_in_window() as usize;
        if log_num < lines_in_window {
            0
        } else {
            self.log_db.len().saturating_sub(lines_in_window)
        }
    }

    pub(crate) fn set_current_frame_area(&mut self, area: Rect) {
        self.current_frame_area = area;
    }
}
