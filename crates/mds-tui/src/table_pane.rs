pub(crate) mod colors;
pub(crate) mod util;

use mds_collector::CollectorUpdate;
use mds_config::shared_config::SharedConfig;
use mds_ipinfo::IpInfo;
use mds_ipinfo::db::IpDb;
use mds_keybindings::Action;

use crate::{
    error_box::ErrorBox,
    message::{Message, Popup},
    table_pane::util::ColumnConstraints,
};
use colors::TableColors;
use mds_netscan::{NetworkScanner, progress::ScannerProgress};
use mds_util::refresh::RefreshListener;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Modifier, Style, Stylize, palette::tailwind},
    symbols,
    text::{Span, Text},
    widgets::{
        Block, Cell, Gauge, HighlightSpacing, Row, Scrollbar, ScrollbarOrientation, ScrollbarState,
        Table, TableState,
    },
};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{self, Receiver};

mod ipinfo_popup;
use ipinfo_popup::IpInfoPopUp;

mod clipboard;
use clipboard::{CopiedCell, MdsClipboard};

pub(crate) struct TablePane {
    longest_item_lens: ColumnConstraints,
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
    clipboard: MdsClipboard,
    copied_cell: Option<CopiedCell>,
    scanner_progress: ScannerProgress,
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
        let scanner_progress = scanner.spawn();

        Self {
            longest_item_lens: ColumnConstraints::default(),
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
            clipboard: MdsClipboard::new(),
            copied_cell: None,
            scanner_progress,
        }
    }

    pub(crate) fn recv_new_ip_info(&mut self) {
        while let Ok(update) = self.rx_ip_info.try_recv() {
            if self.refreshing && update != CollectorUpdate::Refresh {
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
        let last_row_idx = self.ip_db.len() - 1;
        let i = match self.state.selected() {
            Some(i) => {
                if i >= last_row_idx {
                    last_row_idx
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
                    0
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

    /// Copies the content of the currently selected cell to the clipboard.
    pub fn copy_selected_cell_content(
        &mut self,
        search_pattern: Option<&str>,
    ) -> Result<(), ErrorBox> {
        let clipboard = self.clipboard.get()?;

        // Re-generate the list of IPs being displayed, applying the same filters as in `render`
        let ip_info = Self::filtered_ip_info(&self.ip_db, &self.cfg, search_pattern);

        let Some((row_idx, col_idx)) = Self::selected_cell_coords(&self.state) else {
            return Ok(());
        };

        let Some(selected_row) = ip_info.get(row_idx) else {
            return Ok(());
        };

        let cell_contents = selected_row.ref_array();
        let Some(content_to_copy) = cell_contents.get(col_idx) else {
            return Ok(());
        };

        if let Err(e) = clipboard.set_text(content_to_copy.trim().to_owned()) {
            Err(format!("Failed setting clipboard content: {e}").into())
        } else {
            self.copied_cell = Some(CopiedCell::new(row_idx, col_idx));
            Ok(())
        }
    }

    fn filtered_ip_info<'d>(
        db: &'d IpDb,
        cfg: &SharedConfig,
        search_pattern: Option<&str>,
    ) -> Box<[&'d IpInfo]> {
        let mut ip_info = db.get_ip_info(search_pattern);
        if cfg.read().hide_bare_ips() {
            ip_info.retain(|i| !i.names().is_empty() || i.services().is_some());
        }
        ip_info.into_boxed_slice()
    }

    pub(super) fn render(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        search_pattern: Option<&str>,
        in_focus: bool,
    ) {
        let ip_info = Self::filtered_ip_info(&self.ip_db, &self.cfg, search_pattern);

        let ip_info_filtered_len = ip_info.len();
        // Check if the current selected index is out of bounds for the filtered list.
        // Reset the selection and and reset the scrollbar state as well, so it appears scrolled to the top
        if self
            .state
            .selected()
            .is_some_and(|i| i >= ip_info_filtered_len)
        {
            self.state.select(None);
            self.scroll_state = self.scroll_state.position(0);
        }

        self.longest_item_lens = util::ColumnConstraints::new(&ip_info);

        let header = Self::header(self.header_style());
        let rows = Self::rows(&self.colors, &ip_info, self.copied_cell.as_ref());

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(area);
        let table_area = layout[0];
        let gauge_area = layout[1];

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
            .title(self.pane_title(ip_info_filtered_len as u16))
            .border_set(block_border);
        let table: Table<'_> = table.block(table_block);

        frame.render_stateful_widget(table, table_area, &mut self.state);
        self.render_scrollbar(frame, table_area, ip_info_filtered_len);

        self.render_progress_gauge(frame, gauge_area);

        let selected_idx = self.state.selected().unwrap_or(0);
        let selected_ip_info = ip_info.get(selected_idx).copied();
        self.ip_info_popup.render(frame, selected_ip_info);
    }

    pub(crate) fn set_current_frame_area(&mut self, area: Rect) {
        self.current_frame_area = area;
    }

    pub(crate) fn navigate_select(&mut self) -> Option<Message> {
        self.ip_info_popup.is_open = !self.ip_info_popup.is_open;
        if self.ip_info_popup.is_open {
            Some(Popup::IpInfoPopUp.into())
        } else {
            Some(Action::Close.into())
        }
    }

    pub(crate) fn close_action(&mut self) {
        self.ip_info_popup.is_open = false;
    }
}

// Private
impl TablePane {
    const ITEM_HEIGHT: usize = 1;
    const HEADER: [&str; 4] = ["IP", "Name", "Hits", "Services"];

    const TITLE_SUFFIX: &str = " IPs discovered";

    // Used to make the highlight symbol that appears to the left of the selected row
    const SELECTED_BAR: &str = " █ ";

    fn render_scrollbar(&self, frame: &mut Frame, area: Rect, table_len: usize) {
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

    fn render_progress_gauge(&self, frame: &mut Frame, area: Rect) {
        let (scanned, total) = self.scanner_progress.progress_scanned_total();
        let ratio = scanned as f32 / total as f32;

        let ratio = if total == 0 {
            0.0
        } else {
            debug_assert!(
                ratio <= 1.0,
                "Invalid scanner progress ratio={ratio}, scanned: {scanned}, total: {total}"
            );
            (scanned as f32 / total as f32).clamp(0.0, 1.0)
        };

        let label = Span::styled(
            format!("Scanning potential hosts {scanned}/{total}"),
            Style::new().italic().bold(),
        );
        let gauge = Gauge::default()
            .gauge_style(tailwind::CYAN.c800)
            .ratio(ratio.into())
            .label(label);

        frame.render_widget(gauge, area);
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

    fn rows<'a>(
        colors: &TableColors,
        ip_info: &[&IpInfo],
        copied_cell: Option<&CopiedCell>,
    ) -> impl Iterator<Item = Row<'a>> {
        ip_info.iter().enumerate().map(move |(row_idx, ip_info)| {
            let base_color = if ip_info.is_offline() {
                colors.offline_row_color(row_idx)
            } else if ip_info.updated_within_secs(5) {
                colors.newly_updated_row_color(row_idx)
            } else {
                colors.normal_row_color(row_idx)
            };

            let height = Self::calc_row_height(ip_info);
            let row_style = Style::new().fg(colors.row_fg).bg(base_color);
            let item: [String; 4] = ip_info.ref_array();

            item.into_iter()
                .enumerate()
                .map(|(col_idx, content)| {
                    let cell = Cell::from(Text::from(format!("\n{content}\n")));
                    // If the cell was copied recently, highlight it with a special color.
                    if let Some(copied) = copied_cell
                        && copied.copied_recently()
                        && copied.matches_coord(row_idx, col_idx)
                    {
                        return cell.style(
                            Style::new()
                                .add_modifier(Modifier::BOLD)
                                .bg(colors.recently_copied_cell_color())
                                .fg(colors.row_fg),
                        );
                    }
                    cell
                })
                .collect::<Row>()
                .style(row_style)
                .height(height)
        })
    }

    /// Gets the coordinates of the selected cell, if any.
    ///
    /// Returns `Some((row, column))` if a cell is selected, otherwise `None`.
    fn selected_cell_coords(state: &TableState) -> Option<(usize, usize)> {
        Some((state.selected()?, state.selected_column()?))
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
            Constraint::Length((self.longest_item_lens.max_ip_len + 1).max(4)),
            Constraint::Length((self.longest_item_lens.max_hostname_len + 1).max(8)),
            Constraint::Length((self.longest_item_lens.max_packets_count_len + 1).max(5)),
            Constraint::Fill(1),
        ]
    }
}
