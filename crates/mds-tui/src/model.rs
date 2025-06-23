use crate::Message;
use crate::config_box::ConfigBox;
use crate::error_box::{ErrorBox, PromptResponse};
use crate::help_footer::HelpFooter;

use super::RunningState;
use super::log_pane::LogPane;
use super::search_box::SearchBox;
use super::table_pane::TablePane;
use mds_config::AppConfig;
use mds_log::prelude::Logger;
use mds_util::refresh::Refresher;
use parking_lot::RwLock;
use ratatui::crossterm::event;
use ratatui::prelude::*;
use semver::Version;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

#[derive(Debug, PartialEq)]
enum TuiPane {
    Logs,
    IpInfo,
}

pub struct Model<'sb> {
    cfg: Arc<RwLock<AppConfig>>,
    error_box: Option<ErrorBox>,
    refresher: Refresher,
    stop_flag: Arc<AtomicBool>,
    selected_pane: TuiPane,
    running_state: RunningState,
    search_box: Option<SearchBox<'sb>>,
    config_box: ConfigBox,
    table_pane: TablePane,
    log_pane: LogPane,
    logger: Logger,
    pane_constraints: [u16; 2],
    footer: HelpFooter,
}

impl Model<'_> {
    pub fn new(cfg: AppConfig, version: &Version) -> Self {
        let cfg = Arc::new(RwLock::new(cfg));
        let stop_flag = Arc::new(AtomicBool::new(false));
        let refresher = Refresher::new();
        let log_pane = LogPane::new(refresher.listen());
        let background_logger = log_pane.get_logger_clone();

        let table_pane = TablePane::new(
            Arc::clone(&stop_flag),
            background_logger,
            Arc::clone(&cfg),
            refresher.listen(),
        );
        let background_logger = log_pane.get_logger_clone();
        let config_box = ConfigBox::new(Arc::clone(&cfg));

        Self {
            cfg,
            error_box: None,
            refresher,
            stop_flag,
            selected_pane: TuiPane::IpInfo,
            running_state: Default::default(),
            search_box: None,
            config_box,
            table_pane,
            log_pane,
            logger: background_logger,
            pane_constraints: [70, 30],
            footer: HelpFooter::new(version),
        }
    }

    pub fn is_done(&self) -> bool {
        self.running_state == RunningState::Done
    }
    pub(crate) fn set_done(&mut self) {
        self.stop_flag
            .store(true, std::sync::atomic::Ordering::SeqCst);
        self.running_state = RunningState::Done;
    }

    pub fn recv_new_ip_info(&mut self) {
        self.table_pane.recv_new_ip_info();
    }

    pub fn recv_new_logs(&mut self) {
        self.log_pane.recv_new_logs();
    }

    pub(crate) fn increase_verbosity(&self) {
        self.log_pane.increase_verbosity();
    }
    pub(crate) fn decrease_verbosity(&self) {
        self.log_pane.decrease_verbosity();
    }

    pub(super) fn render_table_pane(&mut self, frame: &mut Frame, area: Rect) {
        let search_pattern = self.search_box.as_ref().map(|sb| sb.contents());

        self.table_pane.render(
            frame,
            area,
            search_pattern,
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

    pub(crate) fn render_search_box(&mut self, frame: &mut Frame<'_>, table_area: Rect) {
        if let Some(search) = &mut self.search_box {
            search.render(frame, table_area);
        }
    }

    pub(crate) fn open_config(&mut self) {
        self.config_box.open();
    }

    pub(crate) fn is_config_open(&self) -> bool {
        self.config_box.is_open()
    }

    pub(crate) fn render_config_box(&mut self, frame: &mut Frame<'_>) {
        self.config_box.render(frame);
    }

    pub(crate) fn close_config(&mut self) {
        self.config_box.close();
    }

    pub(crate) fn config_box_input(&mut self, key_event: event::KeyEvent) {
        if let Err(e) = self.config_box.input(key_event) {
            self.error_box = Some(e);
        }
    }

    pub(crate) fn set_current_frame_log_pane_area(&mut self, area: Rect) {
        self.log_pane.set_current_frame_area(area);
    }

    pub(crate) fn set_current_frame_table_pane_area(&mut self, area: Rect) {
        self.table_pane.set_current_frame_area(area);
    }

    pub fn next_row(&mut self) {
        match self.selected_pane {
            TuiPane::Logs => self.log_pane.scroll_down(),
            TuiPane::IpInfo => self.table_pane.next_row(),
        }
    }
    pub fn previous_row(&mut self) {
        match self.selected_pane {
            TuiPane::Logs => self.log_pane.scroll_up(),
            TuiPane::IpInfo => self.table_pane.previous_row(),
        }
    }

    pub(crate) fn scroll_to_start(&mut self) {
        match self.selected_pane {
            TuiPane::Logs => self.log_pane.scroll_to_start(),
            TuiPane::IpInfo => self.table_pane.scroll_to_start(),
        }
    }

    pub(crate) fn scroll_to_end(&mut self) {
        match self.selected_pane {
            TuiPane::Logs => self.log_pane.scroll_to_end(),
            TuiPane::IpInfo => self.table_pane.scroll_to_end(),
        }
    }

    pub(crate) fn navigate_right(&mut self) {
        match self.selected_pane {
            TuiPane::Logs => self.log_pane.scroll_right(),
            TuiPane::IpInfo => self.table_pane.next_column(),
        }
    }
    pub(crate) fn navigate_left(&mut self) {
        match self.selected_pane {
            TuiPane::Logs => self.log_pane.scroll_left(),
            TuiPane::IpInfo => self.table_pane.previous_column(),
        }
    }

    pub(crate) fn navigate_page_up(&mut self) {
        match self.selected_pane {
            TuiPane::Logs => self.log_pane.scroll_page_up(),
            TuiPane::IpInfo => (),
        }
    }

    pub(crate) fn navigate_page_down(&mut self) {
        match self.selected_pane {
            TuiPane::Logs => self.log_pane.scroll_page_down(),
            TuiPane::IpInfo => (),
        }
    }

    pub(crate) fn pane_constraints(&self) -> Vec<Constraint> {
        Constraint::from_percentages(self.pane_constraints)
    }

    pub(crate) fn increase_layout_fill(&mut self) {
        let (grow, shrink) = match self.selected_pane {
            TuiPane::IpInfo => (0, 1),
            TuiPane::Logs => (1, 0),
        };
        self.adjust_panes(grow, shrink);
    }

    pub(crate) fn decrease_layout_fill(&mut self) {
        let (grow, shrink) = match self.selected_pane {
            TuiPane::IpInfo => (1, 0),
            TuiPane::Logs => (0, 1),
        };
        self.adjust_panes(grow, shrink);
    }

    fn adjust_panes(&mut self, grow_idx: usize, shrink_idx: usize) {
        self.pane_constraints[grow_idx] =
            std::cmp::min(self.pane_constraints[grow_idx].saturating_add(5), 100);
        self.pane_constraints[shrink_idx] =
            std::cmp::max(self.pane_constraints[shrink_idx].saturating_sub(5), 2);
    }

    pub(crate) fn render_error_box(&self, frame: &mut Frame<'_>) {
        if let Some(err) = &self.error_box {
            err.render(frame);
        }
    }

    pub(crate) fn is_error_open(&self) -> bool {
        self.error_box.is_some()
    }

    pub(crate) fn close_error(&mut self) {
        self.error_box = None;
    }

    pub(crate) fn error_box_input(&mut self, key_event: event::KeyEvent) -> Option<Message> {
        if let Some(err) = &mut self.error_box {
            if let Some(resp) = err.input(key_event) {
                self.error_box = None;
                return match resp {
                    PromptResponse::Ok => Some(Message::Confirm),
                    PromptResponse::Cancel => Some(Message::Cancel),
                };
            }
        }
        None
    }

    pub(crate) fn confirm_action(&mut self) {
        if self.is_config_open() {
            if let Err(e) = self.config_box.confirm_action() {
                self.error_box = Some(e);
            }
        }
    }

    pub(crate) fn cancel_action(&mut self) {
        if self.is_config_open() {
            self.config_box.cancel_action();
        }
    }

    pub(crate) fn refresh(&self) {
        self.logger.info("Refreshing!");
        self.refresher.signal();
    }

    pub(crate) fn compact_ui(&self) -> bool {
        self.cfg.read().compact()
    }

    pub(crate) fn render_footer(&self, frame: &mut Frame, area: Rect) {
        self.footer.render(frame, area);
    }
}
