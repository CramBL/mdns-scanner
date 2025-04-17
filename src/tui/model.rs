use std::sync::mpsc::Receiver;

use super::RunningState;
use super::log_pane::LogPane;
use super::search_box::SearchBox;
use super::table_pane::TablePane;
use crate::collect_ip;
use crate::ip_info::{AccumulatedIpInfo, IpInfo};
use ratatui::crossterm::event;
use ratatui::prelude::*;
use std::{sync::mpsc, thread};

#[derive(Debug, PartialEq)]
enum TuiPane {
    Logs,
    IpInfo,
}

pub(crate) struct Model<'a> {
    selected_pane: TuiPane,
    running_state: super::RunningState,
    rx_ip_info: Receiver<IpInfo>,
    acc_ip_info: AccumulatedIpInfo,
    search_box: Option<SearchBox<'a>>,
    table_pane: TablePane,
    log_pane: LogPane,
}

impl Default for Model<'_> {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel();
        let log_pane = LogPane::default();
        let background_logger = log_pane.get_logger_clone();

        // Spawn the parser in a thread
        thread::spawn(move || {
            if let Err(e) = collect_ip::collect_ip_info(tx, background_logger) {
                eprintln!("Error in IP info collector: {e}");
            }
        });

        Self {
            selected_pane: TuiPane::IpInfo,
            running_state: Default::default(),
            rx_ip_info: rx,
            acc_ip_info: AccumulatedIpInfo::new(),
            search_box: None,
            table_pane: TablePane::default(),
            log_pane,
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
        self.log_pane.recv_new_logs();
    }

    pub(crate) fn increase_verbosity(&mut self) {
        self.log_pane.increase_verbosity();
    }
    pub(crate) fn decrease_verbosity(&mut self) {
        self.log_pane.decrease_verbosity();
    }

    pub fn next_row(&mut self) {
        self.table_pane.next_row(self.acc_ip_info.len());
    }
    pub fn previous_row(&mut self) {
        self.table_pane.previous_row(self.acc_ip_info.len());
    }

    pub(super) fn render_table_pane(&mut self, frame: &mut Frame, area: Rect) {
        let search_pattern = self.search_box.as_ref().map(|sb| sb.contents());

        let ip_info_vec = self.acc_ip_info.get_ip_info(search_pattern);

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
        self.log_pane
            .render(frame, area, self.selected_pane == TuiPane::Logs);
    }

    pub(crate) fn set_search_active(&mut self) {
        self.search_box = Some(SearchBox::default())
    }

    pub(crate) fn is_search_active(&self) -> bool {
        self.search_box.is_some()
    }

    pub(crate) fn set_search_disabled(&mut self) {
        self.search_box = None;
    }

    pub(crate) fn search_box_input(&mut self, key_event: event::KeyEvent) {
        if let Some(search) = &mut self.search_box {
            search.input(key_event);
        }
    }

    pub(crate) fn render_search_box(&mut self, frame: &mut Frame<'_>) {
        if let Some(search) = &mut self.search_box {
            search.render(frame);
        }
    }
}
