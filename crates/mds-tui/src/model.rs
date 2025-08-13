use crate::Message;
use crate::components::MdsKeyHandler;
use crate::config_window::ConfigWindow;
use crate::error_box::ErrorBox;
use crate::help_footer::HelpFooter;
use crate::message::{Navigate, Open};
use crate::table_pane::ipinfo_popup::IpInfoPopUp;

use super::RunningState;
use super::log_pane::LogPane;
use super::search_box::SearchBox;
use super::table_pane::TablePane;
use mds_config::AppConfig;
use mds_config::shared_config::SharedConfig;
use mds_log::LogMessage;
use mds_log::prelude::Logger;
use mds_util::refresh::Refresher;
use mds_util::resource_scaling::HostResources;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::prelude::*;
use semver::Version;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Receiver;
use std::time::Duration;

#[derive(Debug, Default, PartialEq)]
enum TuiPane {
    Logs,
    #[default]
    IpInfo,
}

pub struct Model {
    cfg: SharedConfig,
    error_box: Option<ErrorBox>,
    refresher: Refresher,
    host_resources: HostResources,
    stop_flag: Arc<AtomicBool>,
    selected_pane: TuiPane,
    running_state: RunningState,
    table_pane: TablePane,
    log_pane: LogPane,
    pane_constraints: [u16; 2],
    footer: HelpFooter,
    components: VecDeque<Box<dyn MdsKeyHandler>>,
}

impl Model {
    pub fn new(
        cfg: AppConfig,
        version: &Version,
        (logger, log_rx): (Logger, Receiver<LogMessage>),
    ) -> Self {
        let cfg = SharedConfig::new(cfg);
        let stop_flag = Arc::new(AtomicBool::new(false));
        let refresher = Refresher::new();
        let log_pane = LogPane::new(refresher.listen(), cfg.read().log_limit(), (logger, log_rx));

        let table_pane = TablePane::new(Arc::clone(&stop_flag), cfg.clone(), refresher.listen());

        let components: VecDeque<Box<dyn MdsKeyHandler>> = vec![].into();

        Self {
            cfg,
            error_box: None,
            refresher,
            stop_flag,
            selected_pane: TuiPane::default(),
            host_resources: HostResources::default(),
            running_state: Default::default(),
            table_pane,
            log_pane,
            pane_constraints: [70, 30],
            footer: HelpFooter::new(version),
            components,
        }
    }

    pub fn render(&mut self, frame: &mut Frame) {
        let constr = self.pane_constraints();
        let pane_constraints = vec![constr[0], constr[1]];
        let layout = Layout::default()
            .constraints(pane_constraints)
            .split(frame.area());
        let top = layout[0];
        let mut bottom = layout[1];

        if !self.compact_ui() {
            let vertical = &Layout::vertical([Constraint::Min(5), Constraint::Length(4)]);
            let rects = vertical.split(bottom);
            self.render_footer(frame, rects[1]);
            bottom = rects[0];
        }

        self.set_current_frame_log_pane_area(bottom);
        self.set_current_frame_table_pane_area(top);
        self.render_log_pane(frame, bottom);
        self.render_table_pane(frame, top);

        let top_idx = self.components.len();
        let mut curr_idx = 0;
        while curr_idx < top_idx {
            if let Some(comp) = self.components.get_mut(curr_idx) {
                comp.render(frame);
            }
            curr_idx += 1;
        }
    }

    pub fn update(&mut self, msg: Message) -> Option<Message> {
        log::info!("update: {msg:?}");
        let mut cont = false;
        match msg.clone() {
            Message::CloseBox | Message::GlobalClose => {
                _ = self.components.pop_back();
                if self.is_error_open() {
                    self.close_error();
                }
            }
            Message::Open(open) => match open {
                Open::Config => self
                    .components
                    .push_back(Box::new(ConfigWindow::new(self.cfg.clone()))),
                Open::Search => {
                    let search_box = SearchBox::new();
                    let pattern = search_box.pattern();
                    self.table_pane.search_pattern = pattern;
                    self.components.push_back(Box::new(search_box))
                }
            },
            Message::Error(error_box) => self.error_box = Some(error_box),
            _ => cont = true,
        }

        if !cont {
            return None;
        }

        let mut top_idx = self.components.len();
        while top_idx > 0 {
            top_idx -= 1;

            if let Some(comp) = self.components.get_mut(top_idx) {
                let msg_opt = comp.update(msg.clone());
                match msg_opt {
                    Ok(msg) => {
                        if msg.is_some() {
                            return msg;
                        }
                    }
                    Err(e) => {
                        self.error_box = Some(e);
                        return None;
                    }
                }
            }
        }

        match msg {
            Message::IncreaseVerbosity => {
                self.increase_verbosity();
            }
            Message::DecreaseVerbosity => {
                self.decrease_verbosity();
            }
            Message::TogglePane => self.toggle_selected_pane(),
            Message::Quit => {
                self.set_done();
            }
            Message::BoxInput(key_event) => {
                if self.is_error_open() {
                    if let Some(ebox) = &mut self.error_box
                        && ebox.is_focused()
                    {
                        let resp = ebox.handle_key_event(key_event);
                        if resp.as_ref().is_ok_and(|r| r.is_some()) {
                            // If it's some the user chose an option so we can
                            // discard/close the error box
                            self.error_box = None;
                        }
                        if let Ok(r) = resp {
                            return r;
                        }
                    }
                }
            }
            Message::ScrollToStart => self.scroll_to_start(),
            Message::ScrollToEnd => self.scroll_to_end(),
            Message::Navigate(nav) => match nav {
                Navigate::Down => self.next_row(),
                Navigate::Up => self.previous_row(),
                Navigate::Right => self.navigate_right(),
                Navigate::Left => self.navigate_left(),
                Navigate::PageUp => self.navigate_page_up(),
                Navigate::PageDown => self.navigate_page_down(),
                Navigate::Select => self.navigate_select(),
            },
            Message::IncreaseLayoutFill => self.increase_layout_fill(),
            Message::DecreaseLayoutFill => self.decrease_layout_fill(),
            Message::Refresh => self.refresh(),
            _ => (),
        };
        None
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Option<Message> {
        log::info!("1: {key:?}");
        let mut top_idx = self.components.len();
        while top_idx > 0 {
            top_idx -= 1;
            log::info!("top_idx={top_idx}");

            if let Some(comp) = self.components.get_mut(top_idx) {
                let msg_opt = comp.handle_key_event(key);
                match msg_opt {
                    Ok(msg) => {
                        if msg.is_some() {
                            return msg;
                        }
                    }
                    Err(e) => {
                        self.error_box = Some(e);
                        return None;
                    }
                }
            }
        }

        log::info!("2: {key:?}");
        let msg = match key.code {
            KeyCode::Char('v') => Some(Message::IncreaseVerbosity),
            KeyCode::Char('g') => Some(Message::DecreaseVerbosity),
            KeyCode::Tab => Some(Message::TogglePane),
            KeyCode::Char('h') | KeyCode::Left => Some(Message::Navigate(Navigate::Left)),
            KeyCode::Char('l') | KeyCode::Right => Some(Message::Navigate(Navigate::Right)),
            KeyCode::Char('j') | KeyCode::Down => Some(Message::Navigate(Navigate::Down)),
            KeyCode::Char('k') | KeyCode::Up => Some(Message::Navigate(Navigate::Up)),
            KeyCode::Home => Some(Message::ScrollToStart),
            KeyCode::End => Some(Message::ScrollToEnd),
            KeyCode::PageDown => Some(Message::Navigate(Navigate::PageDown)),
            KeyCode::PageUp => Some(Message::Navigate(Navigate::PageUp)),
            KeyCode::Char('q') | KeyCode::Char('Q') => Some(Message::Quit),
            KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(Message::Open(Open::Search))
            }
            KeyCode::Char('+') => Some(Message::IncreaseLayoutFill),
            KeyCode::Char('-') => Some(Message::DecreaseLayoutFill),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(Message::Open(Open::Config))
            }
            KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(Message::Refresh)
            }
            KeyCode::Char(' ') | KeyCode::Enter => Some(Message::Navigate(Navigate::Select)),
            _ => None,
        };
        log::info!("Handle key end: {},  msg: {msg:?}", key.code);
        msg
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
        self.table_pane
            .render(frame, area, self.selected_pane == TuiPane::IpInfo);
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

    pub(crate) fn is_error_open(&self) -> bool {
        self.error_box.is_some()
    }

    pub(crate) fn close_error(&mut self) {
        self.error_box = None;
    }

    pub(crate) fn refresh(&self) {
        log::info!("Refreshing!");
        self.refresher.signal();
    }

    pub(crate) fn compact_ui(&self) -> bool {
        self.cfg.read().compact()
    }

    pub(crate) fn render_footer(&self, frame: &mut Frame, area: Rect) {
        self.footer.render(frame, area);
    }

    pub fn passive_refresh_interval(&mut self) -> Duration {
        self.host_resources.passive_refresh_interval()
    }

    pub(crate) fn navigate_select(&mut self) {
        match self.selected_pane {
            TuiPane::Logs => (), // Does nothing
            TuiPane::IpInfo => {
                let selected_ip_info = self.table_pane.selected_ip_info();
                let ip_info_popup = IpInfoPopUp::new(selected_ip_info);
                self.components.push_back(Box::new(ip_info_popup));
            }
        }
    }
}
