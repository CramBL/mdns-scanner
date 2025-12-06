use crate::Message;
use crate::config_window::ConfigWindow;
use crate::error_box::{ErrorBox, PromptResponse};
use crate::message::Popup;
use crate::util::centered_80_percent;

use super::RunningState;
use super::log_pane::LogPane;
use super::search_box::SearchBox;
use super::table_pane::TablePane;
use mds_config::AppConfig;
use mds_config::shared_config::SharedConfig;
use mds_keybindings::popup::KeybindingsPopup;
use mds_keybindings::{Action, KeyBindings};
use mds_log::LogMessage;
use mds_log::prelude::Logger;
use mds_util::refresh::Refresher;
use mds_util::resource_scaling::HostResources;
use ratatui::crossterm::event::{self, KeyEvent};
use ratatui::prelude::*;
use ratatui::widgets::TableState;
use semver::Version;
use smallvec::{SmallVec, smallvec};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Receiver;
use std::time::Duration;
use strum::EnumCount;

#[derive(Debug, PartialEq)]
enum TuiPane {
    Logs,
    IpInfo,
    IpInfoWithSearch,
}

pub struct Model<'sb, 't, 'km> {
    _cfg: SharedConfig,
    keymap: &'km KeyBindings,
    error_box: Option<ErrorBox>,
    refresher: Refresher,
    host_resources: HostResources,
    stop_flag: Arc<AtomicBool>,
    selected_pane: TuiPane,
    popup: SmallVec<[Popup; 5]>,
    running_state: RunningState,
    search_box: Option<SearchBox<'sb, 'km>>,
    config_window: ConfigWindow<'t, 'km>,
    table_pane: TablePane,
    log_pane: LogPane,
    pane_constraints: [u16; 2],
    keybindings_table_state: TableState,
}

impl<'sb, 't, 'km> Model<'sb, 't, 'km> {
    pub fn new(
        cfg: AppConfig,
        keymap: &'km KeyBindings,
        version: &Version,
        (logger, log_rx): (Logger, Receiver<LogMessage>),
    ) -> Self {
        let cfg = SharedConfig::new(cfg);
        let stop_flag = Arc::new(AtomicBool::new(false));
        let refresher = Refresher::new();
        let log_pane = LogPane::new(refresher.listen(), cfg.read().log_limit(), (logger, log_rx));

        let table_pane = TablePane::new(
            Arc::clone(&stop_flag),
            cfg.clone(),
            refresher.listen(),
            version,
        );
        let config_window = ConfigWindow::new(cfg.clone(), &keymap);

        Self {
            _cfg: cfg,
            keymap,
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
            keybindings_table_state: TableState::default(),
        }
    }

    pub fn render(&mut self, frame: &mut Frame) {
        let constr = self.pane_constraints();
        let pane_constraints = vec![constr[0], constr[1]];
        let layout = Layout::default()
            .constraints(pane_constraints)
            .split(frame.area());
        let top = layout[0];
        let bottom = layout[1];

        self.set_current_frame_log_pane_area(bottom);
        self.set_current_frame_table_pane_area(top);
        self.render_log_pane(frame, bottom);
        self.render_table_pane(frame, top);
        self.render_search_box(frame, top);

        self.render_config_window(frame);
        self.render_keybindings_popup(frame);
        self.render_error_box(frame);
    }

    pub fn update(&mut self, msg: impl Into<Message>) -> Option<Message> {
        let msg = msg.into();
        match msg {
            Message::Action(a) => match a {
                Action::Quit => self.set_done(),
                Action::Close => {
                    if let Some(p) = self.popup.pop() {
                        match p {
                            Popup::ConfigBox => self.config_window.close_action(),
                            Popup::SearchBox => {
                                self.selected_pane = TuiPane::IpInfo;
                                self.search_box = None;
                            }
                            Popup::ErrorBox => self.close_error(),
                            Popup::IpInfoPopUp => self.table_pane.close_action(),
                            Popup::Keybindings => (),
                        }
                    }
                }
                Action::Keybindings => {
                    // Prevent pushing it if it's already the top popup
                    if self.popup.last() != Some(&Popup::Keybindings) {
                        self.popup.push(Popup::Keybindings);
                        self.keybindings_table_state.select(Some(0));
                    }
                }
                Action::IncreaseVerbosity => self.increase_verbosity(),
                Action::DecreaseVerbosity => self.decrease_verbosity(),
                Action::ToggleFocus => match self.popup.last() {
                    Some(p) => match p {
                        Popup::ConfigBox => _ = self.config_window.update(msg),
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
                        Popup::IpInfoPopUp | Popup::Keybindings => (),
                    },
                    None => self.toggle_selected_pane(),
                },
                Action::NavigateSelect => match self.popup.last() {
                    Some(p) => match p {
                        Popup::ConfigBox => todo!("Propagate select to config window"),
                        Popup::SearchBox => {
                            unreachable!("Select should not happen if Search Box is focused")
                        }
                        Popup::ErrorBox => {
                            if let Some(err) = &mut self.error_box
                                && let Some(resp) = err.select()
                            {
                                self.error_box = None;
                                return Some(resp.into());
                            }
                        }
                        Popup::Keybindings | Popup::IpInfoPopUp => {
                            return Some(Action::Close.into());
                        }
                    },
                    None => match self.selected_pane {
                        TuiPane::Logs => (), // Does nothing
                        TuiPane::IpInfo | TuiPane::IpInfoWithSearch => {
                            return self.table_pane.navigate_select();
                        }
                    },
                },
                Action::NavigateRight => self.navigate_right(),
                Action::NavigateLeft => self.navigate_left(),
                Action::NavigateDown => match self.popup.last() {
                    Some(p) => match p {
                        Popup::ConfigBox => _ = self.config_window.update(msg),
                        Popup::ErrorBox => (),
                        Popup::SearchBox | Popup::IpInfoPopUp => self.next_row(),
                        Popup::Keybindings => self.next_keybinding_row(),
                    },
                    None => self.next_row(),
                },
                Action::NavigateUp => match self.popup.last() {
                    Some(p) => match p {
                        Popup::ConfigBox => _ = self.config_window.update(msg),
                        Popup::ErrorBox => (),
                        Popup::SearchBox | Popup::IpInfoPopUp => self.previous_row(),
                        Popup::Keybindings => self.previous_keybinding_row(),
                    },
                    None => self.previous_row(),
                },
                Action::NavigatePageup => self.navigate_page_up(),
                Action::NavigatePagedown => self.navigate_page_down(),
                Action::NavigateScrollToEnd => self.scroll_to_end(),
                Action::NavigateScrollToBeginning => self.scroll_to_start(),
                Action::IncreaseLayoutFill => self.increase_layout_fill(),
                Action::DecreaseLayoutFill => self.decrease_layout_fill(),
                Action::Refresh => self.refresh(),
                Action::CopyToClipboard => {
                    if let Err(e) = self.table_pane.copy_selected_cell_content(
                        self.search_box.as_ref().map(|sb| sb.contents()),
                    ) {
                        self.error_box = Some(e);
                    }
                }
                Action::Config => {
                    debug_assert!(!self.popup.contains(&Popup::ConfigBox));
                    return Some(Popup::ConfigBox.into());
                }
                Action::SaveConfig => {} // Should be handled in the popup
                Action::Search => {
                    debug_assert!(!self.popup.contains(&Popup::SearchBox));
                    return Some(Popup::SearchBox.into());
                }
            },
            Message::BoxInput(key_event) => {
                if let Some(p) = self.popup.last() {
                    match p {
                        Popup::ConfigBox => match self.config_window.input(key_event) {
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
                        Popup::Keybindings | Popup::IpInfoPopUp => (),
                    }
                }
            }
            Message::PromptResponse(p) => {
                debug_assert_eq!(self.popup.last(), Some(&Popup::ErrorBox));
                self.popup.pop();
                match p {
                    PromptResponse::Ok => {
                        if self.is_config_open()
                            && let Err(e) = self.config_window.confirm_action()
                        {
                            self.error_box = Some(e);
                        }
                    }
                    PromptResponse::Cancel => {
                        if self.is_config_open() {
                            self.config_window.cancel_action();
                        }
                    }
                }
            }
            Message::Open(p) => {
                debug_assert!(!self.popup.contains(&p));
                match p {
                    Popup::ConfigBox => {
                        self.open_config();
                        self.popup.push(Popup::ConfigBox);
                    }
                    Popup::SearchBox => {
                        self.selected_pane = TuiPane::IpInfoWithSearch;
                        self.search_box = Some(SearchBox::new(self.keymap));
                        self.popup.push(Popup::SearchBox);
                    }
                    Popup::ErrorBox => self.popup.push(Popup::ErrorBox),
                    Popup::IpInfoPopUp => self.popup.push(Popup::IpInfoPopUp),
                    Popup::Keybindings => self.popup.push(Popup::Keybindings),
                }
            }
        };
        None
    }

    fn keymap(&self, key: KeyEvent) -> Option<Message> {
        self.keymap.handle_key(key).map(|a| a.into())
    }

    pub fn handle_key(&self, key: KeyEvent) -> Option<Message> {
        if key.kind != event::KeyEventKind::Press {
            return None;
        }
        match self.popup.last() {
            None | Some(Popup::ErrorBox) | Some(Popup::IpInfoPopUp) | Some(Popup::Keybindings) => {
                self.keymap(key)
            }
            Some(pop_up) => match pop_up {
                Popup::ConfigBox => {
                    if !self.config_window.is_txt_editing()
                        && self.keymap.is_key_basic_navigation(key)
                    {
                        self.keymap(key)
                    } else {
                        Some(Message::BoxInput(key))
                    }
                }

                Popup::SearchBox => match self.selected_pane {
                    TuiPane::Logs => unreachable!("Log pane active when search is open"),
                    TuiPane::IpInfo => {
                        if self.keymap.is_key_basic_navigation(key)
                            || self.keymap.is_key_copy_to_clipboard(key)
                        {
                            self.keymap(key)
                        } else {
                            Some(Message::BoxInput(key))
                        }
                    }
                    TuiPane::IpInfoWithSearch => Some(Message::BoxInput(key)),
                },
                Popup::IpInfoPopUp | Popup::ErrorBox | Popup::Keybindings => {
                    unreachable!("Handled in outer branch: Same as `None`")
                }
            },
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

    pub(crate) fn render_keybindings_popup(&mut self, frame: &mut Frame<'_>) {
        if self.popup.last() == Some(&Popup::Keybindings) {
            let popup = KeybindingsPopup::new(self.keymap);
            frame.render_stateful_widget(popup, frame.area(), &mut self.keybindings_table_state);
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
            TuiPane::IpInfo | TuiPane::IpInfoWithSearch => self
                .table_pane
                .next_row(self.search_box.as_ref().map(|sb| sb.contents())),
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
            TuiPane::IpInfo | TuiPane::IpInfoWithSearch => self
                .table_pane
                .scroll_to_end(self.search_box.as_ref().map(|sb| sb.contents())),
        }
    }

    pub(crate) fn navigate_right(&mut self) {
        match self.popup.last() {
            Some(p) => match p {
                Popup::ConfigBox => _ = self.config_window.update(Action::NavigateRight.into()),
                Popup::SearchBox => match self.selected_pane {
                    TuiPane::Logs => unreachable!("cannot focus logs pane search box is open"),
                    TuiPane::IpInfoWithSearch => {
                        if let Some(sb) = &mut self.search_box {
                            _ = sb.update(Action::NavigateRight.into());
                        }
                    }
                    TuiPane::IpInfo => self.table_pane.next_column(),
                },

                Popup::ErrorBox => {
                    if let Some(err) = &mut self.error_box {
                        err.navigate_right();
                    }
                }
                Popup::Keybindings | Popup::IpInfoPopUp => (),
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
                Popup::ConfigBox => _ = self.config_window.update(Action::NavigateLeft.into()),
                Popup::SearchBox => match self.selected_pane {
                    TuiPane::Logs => unreachable!("cannot focus logs pane search box is open"),
                    TuiPane::IpInfoWithSearch => {
                        if let Some(sb) = &mut self.search_box {
                            _ = sb.update(Action::NavigateLeft.into());
                        }
                    }
                    TuiPane::IpInfo => self.table_pane.previous_column(),
                },
                Popup::ErrorBox => {
                    if let Some(err) = &mut self.error_box {
                        err.navigate_left();
                    }
                }
                Popup::Keybindings | Popup::IpInfoPopUp => (),
            },
            None => match self.selected_pane {
                TuiPane::Logs => self.log_pane.scroll_left(),
                TuiPane::IpInfo | TuiPane::IpInfoWithSearch => self.table_pane.previous_column(),
            },
        }
    }

    fn next_keybinding_row(&mut self) {
        let count = Action::COUNT;
        if count == 0 {
            return;
        }

        let i = match self.keybindings_table_state.selected() {
            Some(i) => {
                if i >= count - 1 {
                    i
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.keybindings_table_state.select(Some(i));
    }

    fn previous_keybinding_row(&mut self) {
        let count = Action::COUNT;
        if count == 0 {
            return;
        }

        let i = match self.keybindings_table_state.selected() {
            Some(i) => i.saturating_sub(1),
            None => 0,
        };
        self.keybindings_table_state.select(Some(i));
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

    pub fn passive_refresh_interval(&mut self) -> Duration {
        self.host_resources.passive_refresh_interval()
    }
}
