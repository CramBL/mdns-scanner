use std::sync::mpsc::Receiver;

use super::RunningState;
use super::search_box::SearchBox;
use super::table::TablePane;
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
    selected_pane: TuiPane,
    running_state: super::RunningState,
    log_level: log::LogLevel,
    rx_ip_info: Receiver<IpInfo>,
    acc_ip_info: AccumulatedIpInfo,
    rx_logs: Receiver<LogMessage>,
    logger: Logger,
    log_msg_buf: AllocRingBuffer<LogMessage>,
    search_active: bool,
    search_box: SearchBox<'a>,
    table_pane: TablePane,
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
            selected_pane: TuiPane::IpInfo,
            running_state: Default::default(),
            log_level: log::LogLevel::Info,
            rx_ip_info: rx,
            acc_ip_info: AccumulatedIpInfo::new(),
            rx_logs,
            logger: local_logger,
            log_msg_buf: AllocRingBuffer::new(1000),
            search_active: false,
            search_box: SearchBox::default(),
            table_pane: TablePane::default(),
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
        self.table_pane.next_row(self.acc_ip_info.len());
    }
    pub fn previous_row(&mut self) {
        self.table_pane.previous_row(self.acc_ip_info.len());
    }

    pub(super) fn render_table_pane(&mut self, frame: &mut Frame, area: Rect) {
        let mut ip_info_vec: Vec<&IpInfo> = self
            .acc_ip_info
            .collection()
            .iter()
            .map(|(_ip, ip_info)| ip_info)
            .collect();
        ip_info_vec.sort_unstable_by_key(|a| a.ip());
        if self.search_active {
            let search_pattern = self.search_box.contents();
            ip_info_vec.retain(|i| {
                i.ip.to_string().contains(search_pattern)
                    || i.names().iter().any(|n| n.contains(search_pattern))
            });
        }

        self.table_pane.render(
            frame,
            area,
            &ip_info_vec,
            self.selected_pane == TuiPane::IpInfo,
        );
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
