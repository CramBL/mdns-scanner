use std::{cmp, sync::mpsc::Receiver};

use super::RunningState;
use super::search_box::SearchBox;
use super::table::{self, TableColors};
use crate::collect_ip;
use crate::ip_info::{AccumulatedIpInfo, IpInfo};
use crate::log::{self, LogLevel, LogMessage, Logger};
use ratatui::crossterm::event;
use ratatui::{prelude::*, widgets::*};
use ringbuffer::{AllocRingBuffer, RingBuffer};
use std::{sync::mpsc, thread};

#[derive(Debug, PartialEq)]
enum TuiPane {
    Logs,
    IpInfo,
}

pub(crate) struct Model<'a> {
    state: TableState,
    selected_pane: TuiPane,
    scroll_state: ScrollbarState,
    colors: TableColors,
    running_state: super::RunningState,
    log_level: log::LogLevel,
    rx_ip_info: Receiver<IpInfo>,
    acc_ip_info: AccumulatedIpInfo,
    longest_item_lens: (u16, u16, u16), // order is (IP, name, seen count)
    rx_logs: Receiver<LogMessage>,
    logger: Logger,
    log_msg_buf: AllocRingBuffer<LogMessage>,
    search_active: bool,
    search_box: SearchBox<'a>,
}

impl Default for Model<'_> {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel();
        let (tx_logs, rx_logs) = mpsc::channel();
        let local_logger = Logger::new(tx_logs, LogLevel::default());
        let background_logger = local_logger.clone();

        // Spawn the parser in a thread
        thread::spawn(move || {
            if let Err(e) = collect_ip::collect_ip_info(tx, background_logger) {
                eprintln!("Error in IP info collector: {}", e);
            }
        });

        Self {
            state: TableState::default().with_selected(0),
            selected_pane: TuiPane::IpInfo,
            scroll_state: ScrollbarState::new(0),
            colors: TableColors::default(),
            running_state: Default::default(),
            log_level: log::LogLevel::Info,
            rx_ip_info: rx,
            acc_ip_info: AccumulatedIpInfo::new(),
            longest_item_lens: (10, 10, 10),
            rx_logs,
            logger: local_logger,
            log_msg_buf: AllocRingBuffer::new(1000),
            search_active: false,
            search_box: SearchBox::default(),
        }
    }
}

impl Model<'_> {
    pub(crate) fn is_done(&self) -> bool {
        self.running_state == RunningState::Done
    }
    pub(crate) fn set_done(&mut self) {
        self.running_state = RunningState::Done;
    }

    pub(crate) fn recv_new_ip_info(&mut self) {
        while let Ok(ip_info) = self.rx_ip_info.try_recv() {
            self.acc_ip_info.insert(ip_info);
        }
    }

    pub(crate) fn recv_new_logs(&mut self) {
        while let Ok(l) = self.rx_logs.try_recv() {
            self.log_msg_buf.push(l);
        }
    }

    pub(crate) fn increase_verbosity(&mut self) {
        self.log_level = self.log_level.increase();
        self.logger.increase_verbosity();
    }
    pub(crate) fn decrease_verbosity(&mut self) {
        self.log_level = self.log_level.decrease();
        self.logger.decrease_verbosity();
    }

    pub(super) fn log_level(&self) -> LogLevel {
        self.log_level
    }
    pub(super) fn latest_logs(&self) -> Vec<&LogMessage> {
        let max: u16 = 50;
        let mut latest_msgs = Vec::with_capacity(max.into());
        for m in self.log_msg_buf.iter().rev() {
            if m.is_within_verbosity(self.log_level) {
                latest_msgs.push(m);
            }
        }
        latest_msgs
    }

    pub fn next_row(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.acc_ip_info.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i * table::ITEM_HEIGHT);
    }
    pub fn previous_row(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.acc_ip_info.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i * table::ITEM_HEIGHT);
    }
    pub fn next_column(&mut self) {
        self.state.select_next_column();
    }

    pub fn previous_column(&mut self) {
        self.state.select_previous_column();
    }

    pub(super) fn render_table_pane(&mut self, frame: &mut Frame, area: Rect) {
        let mut ip_info_vec: Vec<&IpInfo> = self
            .acc_ip_info
            .collection()
            .iter()
            .map(|(_ip, ip_info)| ip_info)
            .collect();
        ip_info_vec.sort_unstable_by_key(|a| a.ip());

        self.longest_item_lens = table::constraint_len_calculator(ip_info_vec.as_slice());
        let header_style = Style::default()
            .fg(self.colors.header_fg)
            .bg(self.colors.header_bg);
        let selected_row_style = Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(self.colors.selected_row_style_fg);
        let selected_col_style = Style::default().fg(self.colors.selected_column_style_fg);
        let selected_cell_style = Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(self.colors.selected_cell_style_fg);

        let header = ["IP", "Name(s)", "Packets"]
            .into_iter()
            .map(Cell::from)
            .collect::<Row>()
            .style(header_style)
            .height(1);
        let rows = ip_info_vec.iter().enumerate().map(|(i, ip_info)| {
            let color = match i % 2 {
                0 => self.colors.normal_row_color,
                _ => self.colors.alt_row_color,
            };
            let hostname_count = ip_info.names().len() as u16;

            let item = ip_info.ref_array();
            item.into_iter()
                .map(|content| Cell::from(Text::from(format!("\n{content}\n"))))
                .collect::<Row>()
                .style(Style::new().fg(self.colors.row_fg).bg(color))
                .height(cmp::max(2, hostname_count + 1))
        });
        let bar = " █ ";
        let table_width = [
            // + 1 is for padding.
            Constraint::Length(self.longest_item_lens.0 + 1),
            Constraint::Min(self.longest_item_lens.1 + 1),
            Constraint::Min(self.longest_item_lens.2),
        ];
        let table = Table::new(rows, table_width)
            .header(header)
            .row_highlight_style(selected_row_style)
            .column_highlight_style(selected_col_style)
            .cell_highlight_style(selected_cell_style)
            .highlight_symbol(Text::from(vec![
                "".into(),
                bar.into(),
                bar.into(),
                "".into(),
            ]))
            .bg(self.colors.buffer_bg)
            .highlight_spacing(HighlightSpacing::Always);
        let block_border_symbol = if self.selected_pane == TuiPane::IpInfo {
            symbols::border::PLAIN
        } else {
            symbols::border::EMPTY
        };
        let table_block = Block::bordered()
            .title(format!("{} IPs discovered", ip_info_vec.len()))
            .border_set(block_border_symbol);
        let table = table.block(table_block);
        frame.render_stateful_widget(table, area, &mut self.state);
    }

    pub(super) fn render_table_scrollbar(&mut self, frame: &mut Frame, area: Rect) {
        let mut state = self.scroll_state.content_length(self.acc_ip_info.len());
        frame.render_stateful_widget(
            Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None),
            area.inner(Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut state,
        );
    }

    pub fn set_table_colors(&mut self) {
        self.colors = table::TableColors::default();
    }

    pub(crate) fn toggle_selected_pane(&mut self) {
        self.selected_pane = match self.selected_pane {
            TuiPane::Logs => TuiPane::IpInfo,
            TuiPane::IpInfo => TuiPane::Logs,
        };
    }

    pub fn render_log_pane(&mut self, frame: &mut Frame, area: Rect) {
        let logs = self.latest_logs();
        let mut list_items: Vec<ListItem> = vec![];
        for msg in logs {
            match msg {
                LogMessage::Error(s) => {
                    list_items.push(ListItem::new(s.as_ref()).red());
                }
                LogMessage::Warn(s) => list_items.push(ListItem::new(s.as_ref()).yellow()),
                LogMessage::Info(s) => list_items.push(ListItem::new(s.as_ref())),
                LogMessage::Debug(s) => list_items.push(ListItem::new(s.as_ref()).cyan()),
                LogMessage::Trace(s) => list_items.push(ListItem::new(s.as_ref()).blue()),
            }
        }

        let list = List::new(list_items);

        let block_border_symbol = if self.selected_pane == TuiPane::Logs {
            symbols::border::PLAIN
        } else {
            symbols::border::EMPTY
        };

        let log_block = Block::bordered()
            .title(format!("Log Level: {}", self.log_level()))
            .border_set(block_border_symbol);

        let log_widget = list.block(log_block);

        frame.render_widget(log_widget, area);
    }

    pub(crate) fn set_search_active(&mut self) {
        self.search_active = true;
    }

    pub(crate) fn is_search_active(&self) -> bool {
        self.search_active
    }

    pub(crate) fn set_search_disabled(&mut self) {
        self.search_active = false;
    }

    pub(crate) fn search_box_input(&mut self, key_event: event::KeyEvent) {
        self.search_box.input(key_event);
    }

    pub(crate) fn render_search_box(&mut self, frame: &mut Frame<'_>) {
        self.search_box.render(frame);
    }
}
