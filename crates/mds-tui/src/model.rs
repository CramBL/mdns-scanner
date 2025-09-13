use crate::config_window::ConfigWindow;
use crate::error_box::{ErrorBox, PromptResponse};
use crate::help_footer::HelpFooter;
use crate::message::{Navigate, Popup};
use crate::util::centered_80_percent;
use crate::{Message, is_key_basic_navigation, is_key_copy_to_clipboard};

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
use ratatui::crossterm::event::{self, KeyEvent};
use ratatui::prelude::*;
use semver::Version;
use smallvec::{SmallVec, smallvec};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Receiver;
use std::time::Duration;

#[derive(Debug, PartialEq)]
enum TuiPane {
    Logs,
    IpInfo,
    IpInfoWithSearch,
}

pub struct Model<'sb, 't> {
    cfg: SharedConfig,
    error_box: Option<ErrorBox>,
    refresher: Refresher,
    host_resources: HostResources,
    stop_flag: Arc<AtomicBool>,
    selected_pane: TuiPane,
    popup: SmallVec<[Popup; 5]>,
    running_state: RunningState,
    search_box: Option<SearchBox<'sb>>,
    config_window: ConfigWindow<'t>,
    table_pane: TablePane,
    log_pane: LogPane,
    pane_constraints: [u16; 2],
    footer: HelpFooter,
}

impl<'sb, 't> Model<'sb, 't> {
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
        let config_window = ConfigWindow::new(cfg.clone());

        Self {
            cfg,
            error_box: None,
            refresher,
            stop_flag,
            selected_pane: TuiPane::IpInfo,
            popup: smallvec![],
            host_resources: HostResources::default(),
            running_state: Default::default(),
            search_box: None,
            config_window,
            table_pane,
            log_pane,
            pane_constraints: [70, 30],
            footer: HelpFooter::new(version),
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
        self.render_search_box(frame, top);
        self.render_config_window(frame);
        self.render_error_box(frame);
    }

    pub fn update(&mut self, msg: impl Into<Message>) -> Option<Message> {
        let msg = msg.into();
        match msg {
            Message::IncreaseVerbosity => {
                self.increase_verbosity();
            }
            Message::DecreaseVerbosity => {
                self.decrease_verbosity();
            }
            Message::ToggleWindow => match self.popup.last() {
                Some(p) => match p {
                    Popup::Config => _ = self.config_window.update(msg),
                    Popup::SearchBox => {
                        self.selected_pane = match self.selected_pane {
                            TuiPane::Logs => {
                                unreachable!("Cannot focus logs while search is active")
                            }
                            TuiPane::IpInfo => TuiPane::IpInfoWithSearch,
                            TuiPane::IpInfoWithSearch => TuiPane::IpInfo,
                        }
                    }
                    Popup::ErrorBox => {
                        if let Some(err) = &mut self.error_box {
                            err.navigate_toggle();
                        }
                    }
                    Popup::IpInfoPopUp => (),
                },
                None => self.toggle_selected_pane(),
            },
            Message::Quit => {
                self.set_done();
            }
            Message::BoxInput(key_event) => {
                if let Some(p) = self.popup.last() {
                    match p {
                        Popup::Config => match self.config_window.input(key_event) {
                            Ok(msg) => return msg,
                            Err(e) => {
                                self.error_box = Some(e);
                                return Some(Popup::ErrorBox.into());
                            }
                        },
                        Popup::SearchBox => {
                            if let Some(search) = &mut self.search_box {
                                return search.update(msg);
                            }
                        }
                        Popup::ErrorBox => {
                            unreachable!("error box only responds to navigate left/right/select")
                        }
                        Popup::IpInfoPopUp => (),
                    }
                }
            }
            Message::Navigate(nav) => match nav {
                Navigate::Select => match self.popup.last() {
                    Some(p) => match p {
                        Popup::Config => todo!("Propagate select to config window"),
                        Popup::SearchBox => {
                            unreachable!("Select should not happen if Search Box is focused")
                        }
                        Popup::ErrorBox => {
                            if let Some(err) = &mut self.error_box {
                                if let Some(resp) = err.select() {
                                    self.error_box = None;
                                    return Some(resp.into());
                                }
                            }
                        }
                        Popup::IpInfoPopUp => return Some(Message::CloseBox),
                    },
                    None => match self.selected_pane {
                        TuiPane::Logs => (), // Does nothing
                        TuiPane::IpInfo | TuiPane::IpInfoWithSearch => {
                            return self.table_pane.navigate_select();
                        }
                    },
                },
                Navigate::Right => self.navigate_right(),
                Navigate::Left => self.navigate_left(),
                Navigate::Down => match self.popup.last() {
                    Some(p) => match p {
                        Popup::Config => _ = self.config_window.update(msg),
                        Popup::ErrorBox => (),
                        Popup::SearchBox | Popup::IpInfoPopUp => self.next_row(),
                    },
                    None => self.next_row(),
                },
                Navigate::Up => match self.popup.last() {
                    Some(p) => match p {
                        Popup::Config => _ = self.config_window.update(msg),
                        Popup::ErrorBox => (),
                        Popup::SearchBox | Popup::IpInfoPopUp => self.previous_row(),
                    },
                    None => self.previous_row(),
                },
                Navigate::PageUp => self.navigate_page_up(),
                Navigate::PageDown => self.navigate_page_down(),
                Navigate::ScrollToEnd => self.scroll_to_end(),
                Navigate::ScrollToBeginning => self.scroll_to_start(),
            },
            Message::IncreaseLayoutFill => self.increase_layout_fill(),
            Message::DecreaseLayoutFill => self.decrease_layout_fill(),
            Message::PromptResponse(p) => {
                debug_assert_eq!(self.popup.last(), Some(&Popup::ErrorBox));
                self.popup.pop();
                match p {
                    PromptResponse::Ok => {
                        if self.is_config_open() {
                            if let Err(e) = self.config_window.confirm_action() {
                                self.error_box = Some(e);
                            }
                        }
                    }
                    PromptResponse::Cancel => {
                        if self.is_config_open() {
                            self.config_window.cancel_action();
                        }
                    }
                }
            }
            Message::Refresh => self.refresh(),
            Message::CopyToClipboard => {
                if let Err(e) = self
                    .table_pane
                    .copy_selected_cell_content(self.search_box.as_ref().map(|sb| sb.contents()))
                {
                    self.error_box = Some(e);
                }
            }
            Message::Open(p) => {
                debug_assert!(!self.popup.contains(&p));
                match p {
                    Popup::Config => {
                        self.open_config();
                        self.popup.push(Popup::Config);
                    }
                    Popup::SearchBox => {
                        self.selected_pane = TuiPane::IpInfoWithSearch;
                        self.search_box = Some(SearchBox::default());
                        self.popup.push(Popup::SearchBox);
                    }
                    Popup::ErrorBox => self.popup.push(Popup::ErrorBox),
                    Popup::IpInfoPopUp => self.popup.push(Popup::IpInfoPopUp),
                }
            }
            Message::CloseBox => {
                if let Some(p) = self.popup.pop() {
                    match p {
                        Popup::Config => self.config_window.close_action(),
                        Popup::SearchBox => {
                            self.selected_pane = TuiPane::IpInfo;
                            self.search_box = None;
                        }
                        Popup::ErrorBox => self.close_error(),
                        Popup::IpInfoPopUp => self.table_pane.close_action(),
                    }
                }
            }
        };
        None
    }

    pub fn handle_key(&self, key: KeyEvent) -> Option<Message> {
        if key.kind == event::KeyEventKind::Press {
            match self.popup.last() {
                None | Some(Popup::ErrorBox) | Some(Popup::IpInfoPopUp) => crate::handle_key(key),
                Some(pop_up) => match pop_up {
                    Popup::Config => {
                        if !self.config_window.is_txt_editing() && is_key_basic_navigation(key) {
                            crate::handle_key(key)
                        } else {
                            Some(Message::BoxInput(key))
                        }
                    }
                    Popup::SearchBox => match self.selected_pane {
                        TuiPane::Logs => unreachable!("Log pane active when search is open"),
                        TuiPane::IpInfo => {
                            if is_key_basic_navigation(key) || is_key_copy_to_clipboard(key) {
                                crate::handle_key(key)
                            } else {
                                Some(Message::BoxInput(key))
                            }
                        }
                        TuiPane::IpInfoWithSearch => Some(Message::BoxInput(key)),
                    },
                    Popup::IpInfoPopUp | Popup::ErrorBox => {
                        unreachable!("Handled in outer branch: Same as `None`")
                    }
                },
            }
        } else {
            None
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
            TuiPane::IpInfoWithSearch | TuiPane::IpInfo => TuiPane::Logs,
        };
    }

    pub fn render_log_pane(&mut self, frame: &mut Frame, area: Rect) {
        self.log_pane
            .render(frame, area, self.selected_pane == TuiPane::Logs);
    }

    pub(crate) fn render_search_box(&mut self, frame: &mut Frame<'_>, table_area: Rect) {
        if let Some(search) = &mut self.search_box {
            search.render(frame, table_area);
        }
    }

    pub(crate) fn open_config(&mut self) {
        self.config_window.open()
    }

    pub(crate) fn is_config_open(&self) -> bool {
        self.config_window.is_open()
    }

    pub(crate) fn render_config_window(&mut self, frame: &mut Frame<'_>) {
        if self.config_window.is_open() {
            let pop_up_area = centered_80_percent(frame);
            frame.render_widget(ratatui::widgets::Clear, pop_up_area);
            let buf = frame.buffer_mut();
            self.config_window.render(pop_up_area, buf);
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
            TuiPane::IpInfo | TuiPane::IpInfoWithSearch => self.table_pane.next_row(),
        }
    }
    pub fn previous_row(&mut self) {
        match self.selected_pane {
            TuiPane::Logs => self.log_pane.scroll_up(),
            TuiPane::IpInfo | TuiPane::IpInfoWithSearch => self.table_pane.previous_row(),
        }
    }

    pub(crate) fn scroll_to_start(&mut self) {
        match self.selected_pane {
            TuiPane::Logs => self.log_pane.scroll_to_start(),
            TuiPane::IpInfo | TuiPane::IpInfoWithSearch => self.table_pane.scroll_to_start(),
        }
    }

    pub(crate) fn scroll_to_end(&mut self) {
        match self.selected_pane {
            TuiPane::Logs => self.log_pane.scroll_to_end(),
            TuiPane::IpInfo | TuiPane::IpInfoWithSearch => self.table_pane.scroll_to_end(),
        }
    }

    pub(crate) fn navigate_right(&mut self) {
        match self.popup.last() {
            Some(p) => match p {
                Popup::Config => _ = self.config_window.update(Navigate::Right.into()),
                Popup::SearchBox => match self.selected_pane {
                    TuiPane::Logs => unreachable!("cannot focus logs pane search box is open"),
                    TuiPane::IpInfoWithSearch => {
                        if let Some(sb) = &mut self.search_box {
                            _ = sb.update(Navigate::Right.into());
                        }
                    }
                    TuiPane::IpInfo => self.table_pane.next_column(),
                },

                Popup::ErrorBox => {
                    if let Some(err) = &mut self.error_box {
                        err.navigate_right();
                    }
                }
                Popup::IpInfoPopUp => (),
            },
            None => match self.selected_pane {
                TuiPane::Logs => self.log_pane.scroll_right(),
                TuiPane::IpInfo | TuiPane::IpInfoWithSearch => self.table_pane.next_column(),
            },
        }
    }

    pub(crate) fn navigate_left(&mut self) {
        match self.popup.last() {
            Some(p) => match p {
                Popup::Config => _ = self.config_window.update(Navigate::Left.into()),
                Popup::SearchBox => match self.selected_pane {
                    TuiPane::Logs => unreachable!("cannot focus logs pane search box is open"),
                    TuiPane::IpInfoWithSearch => {
                        if let Some(sb) = &mut self.search_box {
                            _ = sb.update(Navigate::Left.into());
                        }
                    }
                    TuiPane::IpInfo => self.table_pane.previous_column(),
                },
                Popup::ErrorBox => {
                    if let Some(err) = &mut self.error_box {
                        err.navigate_left();
                    }
                }
                Popup::IpInfoPopUp => (),
            },
            None => match self.selected_pane {
                TuiPane::Logs => self.log_pane.scroll_left(),
                TuiPane::IpInfo | TuiPane::IpInfoWithSearch => self.table_pane.previous_column(),
            },
        }
    }

    pub(crate) fn navigate_page_up(&mut self) {
        match self.selected_pane {
            TuiPane::Logs => self.log_pane.scroll_page_up(),
            TuiPane::IpInfo | TuiPane::IpInfoWithSearch => (),
        }
    }

    pub(crate) fn navigate_page_down(&mut self) {
        match self.selected_pane {
            TuiPane::Logs => self.log_pane.scroll_page_down(),
            TuiPane::IpInfo | TuiPane::IpInfoWithSearch => (),
        }
    }

    pub(crate) fn pane_constraints(&self) -> Vec<Constraint> {
        Constraint::from_percentages(self.pane_constraints)
    }

    pub(crate) fn increase_layout_fill(&mut self) {
        let (grow, shrink) = match self.selected_pane {
            TuiPane::IpInfo | TuiPane::IpInfoWithSearch => (0, 1),
            TuiPane::Logs => (1, 0),
        };
        self.adjust_panes(grow, shrink);
    }

    pub(crate) fn decrease_layout_fill(&mut self) {
        let (grow, shrink) = match self.selected_pane {
            TuiPane::IpInfo | TuiPane::IpInfoWithSearch => (1, 0),
            TuiPane::Logs => (0, 1),
        };
        self.adjust_panes(grow, shrink);
    }

    fn adjust_panes(&mut self, grow_idx: usize, shrink_idx: usize) {
        self.pane_constraints[grow_idx] =
            self.pane_constraints[grow_idx].saturating_add(5).min(100);
        self.pane_constraints[shrink_idx] =
            self.pane_constraints[shrink_idx].saturating_sub(5).max(2);
    }

    pub(crate) fn render_error_box(&self, frame: &mut Frame<'_>) {
        if let Some(err) = &self.error_box {
            err.render(frame);
        }
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
}
