pub(crate) mod colors;
pub(crate) mod util;

use mds_collector::CollectorUpdate;
use mds_config::shared_config::SharedConfig;
use mds_ipinfo::IpInfo;
use mds_ipinfo::db::IpDb;

use colors::TableColors;
use mds_netscan::NetworkScanner;
use mds_util::refresh::RefreshListener;
use ratatui::{
    Frame,
    layout::{Constraint, Margin, Rect},
    style::{Modifier, Style, Stylize},
    symbols,
    text::Text,
    widgets::{
        Block, Cell, HighlightSpacing, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table,
        TableState,
    },
};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{self, Receiver};

mod ipinfo_popup;
use ipinfo_popup::IpInfoPopUp;

pub(crate) struct TablePane {
    pub(crate) longest_item_lens: (u16, u16, u16, u16), // order is (IP, name, seen count, services)
    colors: TableColors,
    state: TableState,
    scroll_state: ScrollbarState,
    ip_db: IpDb,
    rx_ip_info: Receiver<CollectorUpdate>,
    current_frame_area: Rect,
    cfg: SharedConfig,
    refresh_listener: RefreshListener,
    refreshing: bool,
    ip_info_popup: IpInfoPopUp,
}

// Public
impl TablePane {
    pub fn new(
        stop_flag: Arc<AtomicBool>,
        cfg: SharedConfig,
        refresh_listener: RefreshListener,
    ) -> Self {
        let (tx_to_table_pane, rx_from_collector) = mpsc::channel();
        let (tx_to_collector, rx_from_scanners) = mpsc::channel();

        mds_collector::spawn_collector(
            Arc::clone(&stop_flag),
            rx_from_scanners,
            tx_to_table_pane,
            cfg.clone(),
            refresh_listener.clone(),
        );
        let scanner = NetworkScanner::new(
            stop_flag,
            tx_to_collector,
            cfg.clone(),
            refresh_listener.clone(),
        );
        scanner.spawn();

        Self {
            longest_item_lens: (10, 10, 10, 10),
            colors: TableColors::default(),
            state: TableState::default().with_selected(0),
            scroll_state: ScrollbarState::new(0),
            ip_db: IpDb::default(),
            rx_ip_info: rx_from_collector,
            current_frame_area: Rect::ZERO,
            cfg,
            refresh_listener,
            refreshing: false,
            ip_info_popup: IpInfoPopUp::default(),
        }
    }

    pub(crate) fn recv_new_ip_info(&mut self) {
        while let Ok(update) = self.rx_ip_info.try_recv() {
            if self.refreshing && !matches!(update, CollectorUpdate::Refresh) {
                continue; // ignore stale updates during a refresh
            }
            match update {
                CollectorUpdate::IpInfo(ip_info) => self.ip_db.insert(ip_info),
                CollectorUpdate::PacketSeen { ip, rtt } => self.ip_db.update_packets_seen(ip, rtt),
                CollectorUpdate::Status((ip, status)) => {
                    self.ip_db.update_last_known_status(ip, status)
                }
                CollectorUpdate::Refresh => {
                    if self.refreshing || self.refresh_listener.do_refresh() {
                        self.refreshing = false;
                        self.reset();
                    }
                }
            }
        }
        if self.refresh_listener.do_refresh() {
            self.reset();
            self.refreshing = true;
        }
    }

    fn reset(&mut self) {
        self.ip_db.clear();
        self.scroll_to_start();
    }

    pub fn next_column(&mut self) {
        self.state.select_next_column();
    }

    pub fn previous_column(&mut self) {
        self.state.select_previous_column();
    }

    pub fn next_row(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.ip_db.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i * Self::ITEM_HEIGHT);
    }

    pub fn previous_row(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.ip_db.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i * Self::ITEM_HEIGHT);
    }

    pub fn scroll_to_start(&mut self) {
        self.state.select(Some(0));
        self.scroll_state = self.scroll_state.position(0);
    }

    pub fn scroll_to_end(&mut self) {
        let last_index = self.ip_db.len() - 1;
        self.state.select(Some(last_index));
        self.scroll_state = self.scroll_state.position(last_index * Self::ITEM_HEIGHT);
    }

    pub(super) fn render(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        search_pattern: Option<&str>,
        in_focus: bool,
    ) {
        let mut ip_info = self.ip_db.get_ip_info(search_pattern);
        self.longest_item_lens = util::constraint_len_calculator(&ip_info);

        if self.cfg.read().hide_bare_ips() {
            ip_info.retain(|i| !i.names().is_empty() || i.services().is_some());
        }

        let header = Self::header(self.header_style());
        let rows = Self::rows(&self.colors, &ip_info);
        let table = Table::new(rows, self.table_width())
            .header(header)
            .row_highlight_style(self.selected_row_style())
            .column_highlight_style(self.selected_col_style())
            .cell_highlight_style(self.selected_cell_style())
            .highlight_symbol(Self::highlight_symbol())
            .bg(self.colors.buffer_bg)
            .highlight_spacing(HighlightSpacing::Always);

        let block_border = if in_focus {
            symbols::border::PLAIN
        } else {
            symbols::border::EMPTY
        };

        let table_block = Block::bordered()
            .title(self.pane_title(ip_info.len() as u16))
            .border_set(block_border);
        let table: Table<'_> = table.block(table_block);

        frame.render_stateful_widget(table, area, &mut self.state);
        self.render_scollbar(frame, area, ip_info.len());
        let selected_idx = self.state.selected().unwrap_or(0);
        let selected_ip_info = ip_info.get(selected_idx).copied();
        self.ip_info_popup.render(frame, selected_ip_info);
    }

    pub(crate) fn set_current_frame_area(&mut self, area: Rect) {
        self.current_frame_area = area;
    }

    pub(crate) fn navigate_select(&mut self) {
        self.ip_info_popup.is_open = true;
    }

    pub(crate) fn close_action(&mut self) {
        self.ip_info_popup.is_open = false;
    }

    pub(crate) fn is_ip_info_popup_open(&self) -> bool {
        self.ip_info_popup.is_open
    }
}

// Private
impl TablePane {
    const ITEM_HEIGHT: usize = 1;
    const HEADER: [&str; 4] = ["IP", "Name", "Hits", "Services"];

    const TITLE_SUFFIX: &str = " IPs discovered";

    // Used to make the highlight symbol that appears to the left of the selected row
    const SELECTED_BAR: &str = " █ ";

    fn render_scollbar(&self, frame: &mut Frame, area: Rect, table_len: usize) {
        let mut state = self.scroll_state.content_length(table_len);
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

    fn pane_title(&self, item_count: u16) -> String {
        let num = item_count.to_string();
        let mut title = String::with_capacity(num.len() + Self::TITLE_SUFFIX.len());
        title.push_str(&num);
        title.push_str(Self::TITLE_SUFFIX);
        title
    }

    fn highlight_symbol<'a>() -> Text<'a> {
        Text::from(vec![
            "".into(),
            Self::SELECTED_BAR.into(),
            Self::SELECTED_BAR.into(),
            "".into(),
        ])
    }

    fn header<'a>(style: Style) -> Row<'a> {
        Self::HEADER
            .into_iter()
            .map(Cell::from)
            .collect::<Row>()
            .style(style)
            .height(1)
    }

    fn calc_row_height(ip_info: &IpInfo) -> u16 {
        let hostname_count = ip_info.names().len() as u16;
        let service_count = ip_info.services().map_or(0, |s| s.len()) as u16;
        2.max(hostname_count + 1).max(service_count * 2 + 1)
    }

    fn rows<'a>(colors: &TableColors, ip_info: &[&IpInfo]) -> impl Iterator<Item = Row<'a>> {
        ip_info.iter().enumerate().map(|(i, ip_info)| {
            let color = if ip_info.is_offline() {
                colors.offline_row_color(i)
            } else if ip_info.updated_within_secs(5) {
                colors.newly_updated_row_color(i)
            } else {
                colors.normal_row_color(i)
            };

            let height = Self::calc_row_height(ip_info);
            let row_style = Style::new().fg(colors.row_fg).bg(color);
            let item = ip_info.ref_array();

            item.into_iter()
                .map(|content| Cell::from(Text::from(format!("\n{content}\n"))))
                .collect::<Row>()
                .style(row_style)
                .height(height)
        })
    }

    fn selected_row_style(&self) -> Style {
        Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(self.colors.selected_row_style_fg)
    }

    fn selected_col_style(&self) -> Style {
        Style::default().fg(self.colors.selected_column_style_fg)
    }

    fn selected_cell_style(&self) -> Style {
        Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(self.colors.selected_cell_style_fg)
    }

    fn header_style(&self) -> Style {
        Style::default()
            .fg(self.colors.header_fg)
            .bg(self.colors.header_bg)
    }

    fn table_width(&self) -> [Constraint; 4] {
        [
            Constraint::Length(self.longest_item_lens.0),
            Constraint::Length((self.longest_item_lens.1).max(8)),
            Constraint::Length(self.longest_item_lens.2.max(5)),
            Constraint::Length(self.longest_item_lens.3.max(8)),
        ]
    }
}
