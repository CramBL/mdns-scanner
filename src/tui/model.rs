use super::RunningState;
use super::log_pane::LogPane;
use super::search_box::SearchBox;
use super::table_pane::TablePane;
use ratatui::crossterm::event;
use ratatui::prelude::*;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

#[derive(Debug, PartialEq)]
enum TuiPane {
    Logs,
    IpInfo,
}

pub(crate) struct Model<'sb> {
    stop_flag: Arc<AtomicBool>,
    selected_pane: TuiPane,
    running_state: RunningState,
    search_box: Option<SearchBox<'sb>>,
    table_pane: TablePane,
    log_pane: LogPane,
    pane_constraints: [u16; 2],
}

impl Default for Model<'_> {
    fn default() -> Self {
        let stop_flag = Arc::new(AtomicBool::new(false));
        let log_pane = LogPane::default();
        let background_logger = log_pane.get_logger_clone();

        let table_pane = TablePane::new(Arc::clone(&stop_flag), background_logger);

        Self {
            stop_flag,
            selected_pane: TuiPane::IpInfo,
            running_state: Default::default(),
            search_box: None,
            table_pane,
            log_pane,
            pane_constraints: [30, 70],
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
        self.table_pane.recv_new_ip_info();
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
            TuiPane::Logs => (0, 1),
            TuiPane::IpInfo => (1, 0),
        };
        self.adjust_panes(grow, shrink);
    }

    pub(crate) fn decrease_layout_fill(&mut self) {
        let (grow, shrink) = match self.selected_pane {
            TuiPane::Logs => (1, 0),
            TuiPane::IpInfo => (0, 1),
        };
        self.adjust_panes(grow, shrink);
    }

    fn adjust_panes(&mut self, grow_idx: usize, shrink_idx: usize) {
        self.pane_constraints[grow_idx] = match self.pane_constraints[grow_idx] {
            v if v >= 95 => 100,
            v => v.saturating_add(5),
        };

        self.pane_constraints[shrink_idx] = match self.pane_constraints[shrink_idx] {
            v if v <= 5 => 1,
            v => v.saturating_sub(5),
        };
    }
}
