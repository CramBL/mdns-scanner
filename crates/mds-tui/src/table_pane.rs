pub(crate) mod colors;
pub(crate) mod util;

pub(crate) use colors::{TableColors, Theme};

use mds_collector::CollectorUpdate;
use mds_config::shared_config::SharedConfig;
use mds_ipinfo::IpInfo;
use mds_ipinfo::db::IpDb;
use mds_keybindings::Action;
use semver::Version;

use crate::{
    error_box::ErrorBox,
    message::{Message, Popup},
    table_pane::util::ColumnConstraints,
};
use mds_netscan::progress::ScannerProgress;
use mds_util::refresh::RefreshListener;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Modifier, Style, Stylize},
    symbols,
    text::{Line, Span, Text},
    widgets::{
        Block, Cell, Gauge, HighlightSpacing, Row, Scrollbar, ScrollbarOrientation, ScrollbarState,
        Table, TableState,
    },
};
use std::sync::mpsc::Receiver;

mod ipinfo_popup;
use ipinfo_popup::IpInfoPopUp;

mod clipboard;
use clipboard::{CopiedCell, MdsClipboard, SubLineSelector};

pub(crate) struct TablePane {
    longest_item_lens: ColumnConstraints,
    colors: TableColors,
    last_theme_gen: u32,
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
    sub_line_selector: Option<SubLineSelector>,
    scanner_progress: ScannerProgress,
    version: String,
}

// Public
impl TablePane {
    pub fn new(
        cfg: SharedConfig,
        refresh_listener: RefreshListener,
        version: &Version,
        rx_ip_info: Receiver<CollectorUpdate>,
        scanner_progress: ScannerProgress,
    ) -> Self {
        let theme_gen = cfg.theme_version();
        let initial_theme: Theme = cfg.read().ui.theme.parse().unwrap_or_default();
        Self {
            longest_item_lens: ColumnConstraints::default(),
            colors: TableColors::from(initial_theme),
            last_theme_gen: theme_gen,
            state: TableState::default().with_selected(0),
            scroll_state: ScrollbarState::new(0),
            ip_db: IpDb::default(),
            rx_ip_info,
            current_frame_area: Rect::ZERO,
            cfg,
            refresh_listener,
            refreshing: false,
            ip_info_popup: IpInfoPopUp::default(),
            clipboard: MdsClipboard::new(),
            copied_cell: None,
            sub_line_selector: None,
            scanner_progress,
            version: format!("v{version}"),
        }
    }

    pub(crate) fn theme(&self) -> &TableColors {
        &self.colors
    }

    pub(crate) fn recv_new_ip_info(&mut self) {
        let theme_gen = self.cfg.theme_version();
        if theme_gen != self.last_theme_gen {
            let theme: Theme = self.cfg.read().ui.theme.parse().unwrap_or_default();
            self.colors = TableColors::from(theme);
            self.last_theme_gen = theme_gen;
        }
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

    pub fn next_row(&mut self, search_pattern: Option<&str>) {
        let filtered_len = self.get_filtered_len(search_pattern);
        if filtered_len == 0 {
            return;
        }

        let last_row_idx = filtered_len - 1;
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
            Some(i) => i.saturating_sub(1),
            None => 0,
        };
        self.state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i * Self::ITEM_HEIGHT);
    }

    pub fn scroll_to_start(&mut self) {
        self.state.select(Some(0));
        self.scroll_state = self.scroll_state.position(0);
    }

    pub fn scroll_to_end(&mut self, search_pattern: Option<&str>) {
        let filtered_len = self.get_filtered_len(search_pattern);
        if filtered_len == 0 {
            return;
        }

        let last_index = filtered_len - 1;
        self.state.select(Some(last_index));
        self.scroll_state = self.scroll_state.position(last_index * Self::ITEM_HEIGHT);
    }

    /// Copies the content of the currently selected cell to the clipboard.
    ///
    /// If the cell contains multiple lines, enters sub-line selection mode instead of
    /// copying immediately. Call again (or use `sub_line_confirm`) to copy the selected line.
    pub fn copy_selected_cell_content(
        &mut self,
        search_pattern: Option<&str>,
    ) -> Result<(), ErrorBox> {
        if self.sub_line_selector.is_some() {
            return self.sub_line_confirm();
        }

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

        let lines: Vec<String> = content_to_copy
            .trim()
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(str::to_owned)
            .collect();

        if lines.len() <= 1 {
            let text = lines.into_iter().next().unwrap_or_default();
            self.clipboard.set_text(text)?;
            self.copied_cell = Some(CopiedCell::new(selected_row.ip(), col_idx));
        } else {
            // Fail early if clipboard is unavailable - better to error before the
            // user navigates the sub-line selector than after they confirm.
            self.clipboard.check_supported()?;
            // Snapshot the current content so the display stays stable for the
            // duration of the sub-line selection interaction.
            self.sub_line_selector = Some(SubLineSelector::new(selected_row.ip(), col_idx, lines));
        }

        Ok(())
    }

    pub(crate) fn is_in_sub_line_selection(&self) -> bool {
        self.sub_line_selector.is_some()
    }

    pub(crate) fn sub_line_next(&mut self) {
        if let Some(sel) = &mut self.sub_line_selector {
            sel.next();
        }
    }

    pub(crate) fn sub_line_prev(&mut self) {
        if let Some(sel) = &mut self.sub_line_selector {
            sel.prev();
        }
    }

    pub(crate) fn sub_line_cancel(&mut self) {
        self.sub_line_selector = None;
    }

    /// Copies the currently highlighted sub-line to the clipboard and exits selection mode.
    /// Derives the line content from the live data, so the index is clamped if entries
    /// have changed since the selector was opened.
    pub(crate) fn sub_line_confirm(&mut self) -> Result<(), ErrorBox> {
        let Some(sel) = self.sub_line_selector.take() else {
            return Ok(());
        };

        let (text, copied_cell) = if sel.is_copy_all() {
            let text = sel.lines().join("\n");
            let cell = CopiedCell::new(sel.ip, sel.col);
            (text, cell)
        } else {
            let Some(line) = sel.selected_line() else {
                return Ok(());
            };
            // Store the line text so the flash can find the correct line even
            // if new entries arrive and shift positional indices after this.
            let cell = CopiedCell::new_sub_line(sel.ip, sel.col, line.to_owned());
            (line.to_owned(), cell)
        };

        self.clipboard.set_text(text)?;
        self.copied_cell = Some(copied_cell);
        Ok(())
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

        // While sub-line selection is active the tracked row may have shifted
        // (e.g. a new IP that sorts before it was discovered).  Keep
        // `self.state` pointing at the actual current index so the row
        // highlight and ip-info popup always follow the right entry.
        if let Some(sel) = &self.sub_line_selector
            && let Some(idx) = ip_info.iter().position(|info| sel.matches_ip(info))
        {
            self.state.select(Some(idx));
            self.scroll_state = self.scroll_state.position(idx * Self::ITEM_HEIGHT);
        }

        self.longest_item_lens = util::ColumnConstraints::new(&ip_info);

        let header = Self::header(self.header_style());
        let rows = Self::rows(
            &self.colors,
            &ip_info,
            self.copied_cell.as_ref(),
            self.sub_line_selector.as_ref(),
        );

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
            .title(
                Line::from(self.pane_title(ip_info_filtered_len as u16)).style(self.colors.title()),
            )
            .title(
                Line::from(self.version.clone())
                    .style(self.colors.title())
                    .right_aligned(),
            )
            .border_style(self.colors.border())
            .border_set(block_border);
        let table: Table<'_> = table.block(table_block);

        frame.render_stateful_widget(table, table_area, &mut self.state);
        self.render_scrollbar(frame, table_area, ip_info_filtered_len);

        self.render_progress_gauge(frame, gauge_area);

        let selected_idx = self.state.selected().unwrap_or(0);
        let selected_ip_info = ip_info.get(selected_idx).copied();
        self.ip_info_popup
            .render(frame, selected_ip_info, &self.colors);
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

    #[cfg(any(test, feature = "test-utils"))]
    pub(crate) fn use_stub_clipboard(&mut self) {
        self.clipboard = MdsClipboard::stub();
    }
}

// Private
impl TablePane {
    const ITEM_HEIGHT: usize = 1;
    const HEADER: [&str; 4] = ["IP", "Name", "Hits", "Services"];

    const TITLE_SUFFIX: &str = " IPs discovered";

    // Used to make the highlight symbol that appears to the left of the selected row
    const SELECTED_BAR: &str = " █ ";

    // Column indices
    const COL_IP: usize = 0;
    const COL_NAME: usize = 1;
    const COL_HITS: usize = 2;
    const COL_SERVICES: usize = 3;

    // Row height formula: each row is at least MIN_ROW_HEIGHT tall, plus ROW_PADDING blank
    // line at the top, and LINES_PER_SERVICE lines for each discovered service.
    const MIN_ROW_HEIGHT: u16 = 2;
    const ROW_PADDING: u16 = 1;
    const LINES_PER_SERVICE: u16 = 2;

    // When sub-line selection is open the row grows by: one "copy all" virtual option,
    // one blank line at the top, one blank line at the bottom.
    const SUB_LINE_SELECTOR_EXTRA_ROWS: u16 = 3;

    // Column width formula: live content width + CELL_PADDING; when the cursor is shown
    // the "▶ " prefix also adds CURSOR_PREFIX_WIDTH characters.
    const CURSOR_PREFIX_WIDTH: u16 = 2;
    const CELL_PADDING: u16 = 1;

    // Minimum rendered widths for each column (must fit the header label at minimum).
    const COL_IP_MIN_WIDTH: u16 = 4;
    const COL_NAME_MIN_WIDTH: u16 = 8;
    const COL_HITS_MIN_WIDTH: u16 = 5;

    // Rows updated more recently than this threshold are highlighted as "newly updated".
    const RECENTLY_UPDATED_SECS: u16 = 5;

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
            .style(self.colors.gauge_bg())
            .gauge_style(self.colors.gauge_fill())
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
        Self::MIN_ROW_HEIGHT
            .max(hostname_count + Self::ROW_PADDING)
            .max(service_count * Self::LINES_PER_SERVICE + Self::ROW_PADDING)
    }

    fn rows<'a>(
        colors: &TableColors,
        ip_info: &[&IpInfo],
        copied_cell: Option<&CopiedCell>,
        sub_line_selector: Option<&SubLineSelector>,
    ) -> impl Iterator<Item = Row<'a>> {
        ip_info.iter().enumerate().map(move |(row_idx, ip_info)| {
            let base_color = if ip_info.is_offline() {
                colors.offline_row_color(row_idx)
            } else if ip_info.updated_within_secs(Self::RECENTLY_UPDATED_SECS) {
                colors.newly_updated_row_color(row_idx)
            } else {
                colors.normal_row_color(row_idx)
            };

            let height = if let Some(sel) = sub_line_selector
                && sel.matches_ip(ip_info)
            {
                Self::sub_line_row_height(sel, ip_info)
            } else {
                Self::calc_row_height(ip_info)
            };
            let row_style = colors.row_with_bg(base_color);
            let item = ip_info.ref_array();

            item.into_iter()
                .enumerate()
                .map(|(col_idx, content)| {
                    // Sub-line selection mode: render the snapshotted lines with
                    // a ▶ cursor and a "copy all" option at the bottom.
                    if let Some(sel) = sub_line_selector
                        && sel.matches_ip_and_col(ip_info, col_idx)
                    {
                        let cursor = sel.selected_index();
                        let is_copy_all = sel.is_copy_all();
                        // Use explicit fg so the cursor line stays readable on
                        // any background color without relying on REVERSED.
                        let cursor_style = colors.row().add_modifier(Modifier::BOLD);
                        let dim_style = Style::new().add_modifier(Modifier::DIM);

                        let mut lines: Vec<Line<'_>> = vec![Line::raw("")];
                        for (i, line_text) in sel.lines().iter().enumerate() {
                            if i == cursor && !is_copy_all {
                                lines.push(Line::raw(format!("▶ {line_text}")).style(cursor_style));
                            } else {
                                lines.push(Line::raw(format!("  {line_text}")).style(dim_style));
                            }
                        }
                        let copy_all_text = "[ copy all ]";
                        lines.push(if is_copy_all {
                            Line::raw(format!("▶ {copy_all_text}")).style(cursor_style)
                        } else {
                            Line::raw(format!("  {copy_all_text}")).style(dim_style)
                        });
                        lines.push(Line::raw(""));
                        return Cell::from(Text::from(lines));
                    }

                    // Recently-copied flash: only highlight the specific line if it was a
                    // sub-line copy, or the whole cell if it was a full-cell copy.
                    if let Some(copied) = copied_cell
                        && copied.copied_recently()
                        && copied.matches_ip_and_col(ip_info, col_idx)
                    {
                        let copy_style = colors
                            .row()
                            .add_modifier(Modifier::BOLD)
                            .bg(colors.recently_copied_cell_color());
                        if let Some(line_text) = copied.line_text() {
                            // Identify the flashed line by text so the correct
                            // line is highlighted even if new lines were
                            // inserted after the copy was confirmed.
                            let current_lines: Vec<&str> = content
                                .trim()
                                .lines()
                                .map(str::trim)
                                .filter(|l| !l.is_empty())
                                .collect();
                            let mut lines: Vec<Line<'_>> = vec![Line::raw("")];
                            for line in &current_lines {
                                let style = if *line == line_text {
                                    copy_style
                                } else {
                                    Style::new()
                                };
                                lines.push(Line::raw(line.to_string()).style(style));
                            }
                            lines.push(Line::raw(""));
                            return Cell::from(Text::from(lines));
                        } else {
                            return Cell::from(Text::from(format!("\n{content}\n")))
                                .style(copy_style);
                        }
                    }

                    Cell::from(Text::from(format!("\n{content}\n")))
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
        self.colors.selected_row()
    }
    fn selected_col_style(&self) -> Style {
        self.colors.selected_col()
    }
    fn selected_cell_style(&self) -> Style {
        self.colors.selected_cell()
    }
    fn header_style(&self) -> Style {
        self.colors.header()
    }

    fn table_width(&self) -> [Constraint; 4] {
        // When sub-line selection is active the selected column's width is
        // frozen to the snapshot content so the layout cannot shift if longer
        // names arrive for the same IP during an open selection.
        let col_width = |col: usize, live_width: u16, min: u16| -> u16 {
            if let Some(sel) = &self.sub_line_selector
                && sel.col == col
            {
                (sel.max_line_width() + Self::CURSOR_PREFIX_WIDTH + Self::CELL_PADDING).max(min)
            } else {
                (live_width + Self::CELL_PADDING).max(min)
            }
        };
        [
            Constraint::Length(col_width(
                Self::COL_IP,
                self.longest_item_lens.max_ip_len,
                Self::COL_IP_MIN_WIDTH,
            )),
            Constraint::Length(col_width(
                Self::COL_NAME,
                self.longest_item_lens.max_hostname_len,
                Self::COL_NAME_MIN_WIDTH,
            )),
            Constraint::Length(col_width(
                Self::COL_HITS,
                self.longest_item_lens.max_packets_count_len,
                Self::COL_HITS_MIN_WIDTH,
            )),
            Constraint::Fill(1),
        ]
    }

    /// Row height when sub-line selection is active for this row.
    ///
    /// The selector column uses the snapshot size (lines + "copy all" + 2 padding)
    /// so the row height is frozen and does not shift when new entries for the same
    /// IP arrive.  Other variable-height columns (names ↔ services) still use live
    /// data so their content is never clipped.
    fn sub_line_row_height(sel: &SubLineSelector, ip_info: &IpInfo) -> u16 {
        // snapshot lines + "copy all" option + top/bottom padding
        let selector_height = sel.lines().len() as u16 + Self::SUB_LINE_SELECTOR_EXTRA_ROWS;
        let other_height = if sel.col == Self::COL_NAME {
            // name column is frozen - let service height drive the row
            let service_count = ip_info.services().map_or(0, |s| s.len()) as u16;
            Self::MIN_ROW_HEIGHT.max(service_count * Self::LINES_PER_SERVICE + Self::ROW_PADDING)
        } else if sel.col == Self::COL_SERVICES {
            // services column is frozen - let name height drive the row
            let hostname_count = ip_info.names().len() as u16;
            Self::MIN_ROW_HEIGHT.max(hostname_count + Self::ROW_PADDING)
        } else {
            Self::MIN_ROW_HEIGHT
        };
        selector_height.max(other_height)
    }

    /// Gets the length of the filtered IP list based on current configuration and search pattern
    fn get_filtered_len(&self, search_pattern: Option<&str>) -> usize {
        Self::filtered_ip_info(&self.ip_db, &self.cfg, search_pattern).len()
    }
}
