use std::{
    iter, mem,
    net::IpAddr,
    num::NonZero,
    sync::mpsc::TryRecvError,
    time::{Duration, Instant},
};

use mds_ipinfo::{
    IanaPortCategory, IpForHost, IpInfo, PortRanges,
    port_scan_result::{MdnsProbeOutcome, PortScanOutcomeCounts, PortScanResult, PortScanSettings},
};
use mds_keybindings::{Action, KeyBindings};
use mds_netscan::port_scan::{
    self, PortList, PortListParseError, PortOutcome, PortScanJob, PortScanRequest, PortScanUpdate,
};
use ratatui::{
    Frame,
    crossterm::event::{KeyCode, KeyEvent},
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, BorderType, Clear, Gauge, HighlightSpacing, List, ListItem, ListState, Paragraph,
        Scrollbar, ScrollbarOrientation, ScrollbarState,
    },
};
use strum::{EnumCount as _, IntoEnumIterator as _};
use tui_textarea::TextArea;
use unicode_width::UnicodeWidthStr as _;

#[cfg(any(test, feature = "test-utils"))]
use std::sync::mpsc::Sender;

use crate::{format, table_pane::TableColors, util};

/// The host a scan is pinned to, together with the settings the next scan runs
/// under. The host, its name, and its scanned address are fixed when the dialog
/// opens. The connect timeout and worker count follow the live configuration and
/// are refreshed while the dialog stays open. A running scan holds the values it
/// started with in [RunningPortScan].
struct PortScanTarget {
    scanned_ip: IpAddr,
    host: IpForHost,
    name: Option<String>,
    connect_timeout: Duration,
    worker_count: NonZero<u16>,
}

impl PortScanTarget {
    fn title(&self) -> String {
        match &self.name {
            Some(name) => format!("Port scan {ip} ({name})", ip = self.scanned_ip),
            None => format!("Port scan {ip}", ip = self.scanned_ip),
        }
    }
}

struct RunningPortScan {
    job: PortScanJob,
    settings: PortScanSettings,
    scanned_categories: Vec<IanaPortCategory>,
    scanned: u32,
    open_ports: Vec<u16>,
    counts: PortScanOutcomeCounts,
    mdns_outcome: Option<MdnsProbeOutcome>,
    started: Instant,
    /// Body height captured from the dialog state this scan started in. Scanning
    /// keeps this height for the duration of the scan.
    body_height: u16,
}

impl RunningPortScan {
    fn record(&mut self, port: u16, outcome: PortOutcome) {
        self.scanned += 1;
        match outcome {
            PortOutcome::Open => {
                let position = self.open_ports.partition_point(|open| *open < port);
                self.open_ports.insert(position, port);
            }
            PortOutcome::Closed => self.counts.closed += 1,
            PortOutcome::Filtered => self.counts.filtered += 1,
        }
    }

    fn into_result(self, cancelled: bool) -> PortScanResult {
        PortScanResult::new(
            self.open_ports,
            self.counts,
            self.mdns_outcome,
            self.started.elapsed(),
            cancelled,
            self.settings,
            self.scanned_categories,
        )
    }
}

enum PortScanState {
    Setup,
    Scanning(RunningPortScan),
    Results(PortScanResult),
}

/// Modal port-scan dialog: pick the port sources, watch the scan, read the results.
///
/// The next scan starts from the same selection: the checked sources and the
/// custom text outlive each opening of the dialog.
pub(super) struct PortScanPopup {
    target: Option<PortScanTarget>,
    state: PortScanState,
    ranges_checked: [bool; IanaPortCategory::COUNT],
    custom_checked: bool,
    mdns_checked: bool,
    focus: usize,
    custom_input: TextArea<'static>,
    selected_ports: Result<PortList, PortListParseError>,
    results_state: ListState,
    /// Interior width frozen when the dialog opens, so the border does not move
    /// horizontally as the state and its content change.
    frozen_interior_width: Option<u16>,
    /// Body height of the most recently rendered frame. Scanning captures this
    /// at scan start to keep the size of the state it began in.
    last_body_line_count: u16,
}

impl Default for PortScanPopup {
    fn default() -> Self {
        let mut custom_input = TextArea::default();
        custom_input.set_cursor_line_style(Style::default());
        custom_input.set_cursor_style(Style::new().add_modifier(Modifier::REVERSED));
        Self {
            target: None,
            state: PortScanState::Setup,
            ranges_checked: [false; IanaPortCategory::COUNT],
            custom_checked: false,
            mdns_checked: false,
            focus: 0,
            custom_input,
            selected_ports: Ok(PortList::default()),
            results_state: ListState::default(),
            frozen_interior_width: None,
            last_body_line_count: Self::SETUP_BODY_LINE_COUNT,
        }
    }
}

const CUSTOM_ROW: usize = IanaPortCategory::COUNT;
const MDNS_ROW: usize = CUSTOM_ROW + 1;
const START_ROW: usize = MDNS_ROW + 1;
const SETUP_ROW_COUNT: usize = START_ROW + 1;

// Dialog state and input
impl PortScanPopup {
    const CUSTOM_PLACEHOLDER: &'static str = "22,80,443,8000-8100";
    const MDNS_PORT_LABEL: &'static str = "5353/udp";

    pub(super) fn is_open(&self) -> bool {
        self.target.is_some()
    }

    pub(super) fn is_scanning(&self) -> bool {
        matches!(self.state, PortScanState::Scanning(_))
    }

    /// While this is `true`, every key that is not up/down/select/close is
    /// routed to the custom port text field.
    pub(super) fn is_custom_ports_focused(&self) -> bool {
        self.focus == CUSTOM_ROW && matches!(self.state, PortScanState::Setup)
    }

    pub(super) fn open(
        &mut self,
        info: &IpInfo,
        connect_timeout: Duration,
        port_scan_io_threads: NonZero<u16>,
    ) {
        self.state = match info.port_scan_result() {
            Some(previous) => PortScanState::Results(previous.clone()),
            None => PortScanState::Setup,
        };
        self.focus = 0;
        self.results_state.select(Some(0));
        self.target = Some(PortScanTarget {
            scanned_ip: info.ip().primary_address(),
            host: info.ip(),
            name: info.names().first().map(|n| n.display_name().to_owned()),
            connect_timeout,
            worker_count: port_scan_io_threads,
        });
        self.frozen_interior_width = None;
        self.refresh_selected_ports();
    }

    pub(super) fn close(&mut self) {
        self.target = None;
        self.state = PortScanState::Setup;
        self.frozen_interior_width = None;
    }

    /// Updates the open dialog's connect timeout and worker count from the current
    /// configuration.
    pub(super) fn refresh_live_settings(
        &mut self,
        connect_timeout: Duration,
        worker_count: NonZero<u16>,
    ) {
        if let Some(target) = &mut self.target {
            target.connect_timeout = connect_timeout;
            target.worker_count = worker_count;
        }
    }

    pub(super) fn navigate_up(&mut self) {
        match self.state {
            PortScanState::Setup => self.focus = self.focus.saturating_sub(1),
            PortScanState::Scanning(_) => (),
            PortScanState::Results(_) => self.results_state.select_previous(),
        }
    }

    pub(super) fn navigate_down(&mut self) {
        match self.state {
            PortScanState::Setup => self.focus = (self.focus + 1).min(SETUP_ROW_COUNT - 1),
            PortScanState::Scanning(_) => (),
            PortScanState::Results(_) => self.results_state.select_next(),
        }
    }

    /// Enter: toggle the focused checkbox or launch the scan on the setup screen,
    /// or leave the results behind to change the selection.
    pub(super) fn select(&mut self) {
        match self.state {
            PortScanState::Setup => self.setup_select(),
            PortScanState::Scanning(_) => (),
            PortScanState::Results(_) => self.state = PortScanState::Setup,
        }
    }

    pub(super) fn input(&mut self, key: KeyEvent) {
        if !self.is_custom_ports_focused() {
            return;
        }
        let edited = match key.code {
            KeyCode::Char(_)
            | KeyCode::Backspace
            | KeyCode::Delete
            | KeyCode::Left
            | KeyCode::Right
            | KeyCode::Home
            | KeyCode::End => self.custom_input.input(key),
            _ => false,
        };
        if edited {
            self.refresh_selected_ports();
        }
    }

    /// Re-runs the finished scan against the same host and selection.
    pub(super) fn rescan(&mut self) {
        if matches!(self.state, PortScanState::Results(_)) {
            self.start_scan();
        }
    }

    /// Stops a running scan and keeps what it found so far.
    pub(super) fn cancel_scan(&mut self) -> Option<(IpForHost, PortScanResult)> {
        let scanned_ip = self.target.as_ref()?.scanned_ip;
        let PortScanState::Scanning(running) = &self.state else {
            return None;
        };
        running.job.cancel();
        log::info!(
            "Cancelled port scan of {scanned_ip} after {scanned}/{total} ports",
            scanned = running.scanned,
            total = running.settings.port_count
        );
        self.finish_scan(true)
    }

    /// Drains the scan updates that arrived since the last call. Returns the
    /// result of a scan that just ended, for the caller to store on the host.
    pub(super) fn poll_updates(&mut self) -> Option<(IpForHost, PortScanResult)> {
        let PortScanState::Scanning(running) = &mut self.state else {
            return None;
        };

        loop {
            match running.job.try_recv() {
                Ok(PortScanUpdate::PortScanned { port, outcome }) => running.record(port, outcome),
                Ok(PortScanUpdate::MdnsProbed(outcome)) => running.mdns_outcome = Some(outcome),
                Ok(PortScanUpdate::Finished) => return self.finish_scan(false),
                Err(TryRecvError::Empty) => return None,
                Err(TryRecvError::Disconnected) => {
                    log::warn!("Port scan ended without reporting completion");
                    return self.finish_scan(false);
                }
            }
        }
    }

    fn setup_select(&mut self) {
        match self.focus {
            index if index < IanaPortCategory::COUNT => {
                self.ranges_checked[index] = !self.ranges_checked[index];
                self.refresh_selected_ports();
            }
            CUSTOM_ROW => {
                self.custom_checked = !self.custom_checked;
                self.refresh_selected_ports();
            }
            MDNS_ROW => self.mdns_checked = !self.mdns_checked,
            _ => self.start_scan(),
        }
    }

    fn start_scan(&mut self) {
        let Some(target) = &self.target else {
            return;
        };
        if !self.can_start() {
            return;
        }
        let Ok(ports) = &self.selected_ports else {
            return;
        };

        let ports = ports.clone();
        let mdns_probe_timeout = self.mdns_checked.then_some(port_scan::MDNS_PROBE_TIMEOUT);
        let settings = PortScanSettings {
            port_count: ports.len(),
            port_ranges: PortRanges::merged_from(ports.ports()),
            worker_count: target.worker_count,
            connect_timeout: target.connect_timeout,
            mdns_probe_timeout,
        };
        let scanned_categories = IanaPortCategory::categories_covering(ports.ports());
        let job = port_scan::spawn_port_scan(PortScanRequest::new(
            target.scanned_ip,
            ports,
            target.connect_timeout,
            target.worker_count,
            mdns_probe_timeout,
        ));
        self.results_state.select(Some(0));
        self.state = PortScanState::Scanning(RunningPortScan {
            job,
            settings,
            scanned_categories,
            scanned: 0,
            open_ports: Vec::new(),
            counts: PortScanOutcomeCounts {
                closed: 0,
                filtered: 0,
            },
            mdns_outcome: None,
            started: Instant::now(),
            body_height: self.last_body_line_count,
        });
    }

    fn finish_scan(&mut self, cancelled: bool) -> Option<(IpForHost, PortScanResult)> {
        let PortScanState::Scanning(running) = mem::replace(&mut self.state, PortScanState::Setup)
        else {
            return None;
        };
        let result = running.into_result(cancelled);
        self.state = PortScanState::Results(result.clone());
        self.results_state.select(Some(0));
        Some((self.target.as_ref()?.host, result))
    }

    fn refresh_selected_ports(&mut self) {
        if self.target.is_none() {
            return;
        }
        self.selected_ports = self.compute_selected_ports();
    }

    /// Union of every checked source, deduplicated and sorted by [PortList].
    ///
    /// The box can be checked before the text is typed: an empty custom field
    /// contributes nothing even when its box is checked.
    fn compute_selected_ports(&self) -> Result<PortList, PortListParseError> {
        let mut ports: Vec<u16> = Vec::new();
        for (category, checked) in IanaPortCategory::iter().zip(self.ranges_checked) {
            if checked {
                ports.extend(category.range());
            }
        }
        if self.custom_checked && !self.custom_input_text().trim().is_empty() {
            let custom: PortList = self.custom_input_text().parse()?;
            ports.extend(custom.ports());
        }
        Ok(ports.into_iter().collect())
    }

    fn can_start(&self) -> bool {
        match &self.selected_ports {
            Ok(ports) => !ports.is_empty() || self.mdns_checked,
            Err(_) => false,
        }
    }

    fn custom_input_text(&self) -> &str {
        self.custom_input.lines().first().map_or("", String::as_str)
    }
}

// Rendering
impl PortScanPopup {
    const FOCUS_MARKER_WIDTH: usize = 2;
    const CHECKBOX_COLUMN_WIDTH: usize = 4;
    const LABEL_COLUMN_WIDTH: usize = 17;
    const VALUE_COLUMN_X: usize =
        Self::FOCUS_MARKER_WIDTH + Self::CHECKBOX_COLUMN_WIDTH + Self::LABEL_COLUMN_WIDTH;
    /// Floor for the frozen interior width. Wide enough for the widest checkbox
    /// row and footer, so the body never overflows even when a state's content is
    /// narrow.
    const MIN_INTERIOR_WIDTH: u16 = 46;
    /// Fixed-width fields of the header's metrics line, so a shorter value never
    /// shifts the field after it. Each field is a value followed by
    /// [`Self::METRICS_FIELD_GAP`] spaces of separation.
    const HEADER_WORKERS_FIELD: usize = 13;
    const HEADER_TIMEOUT_FIELD: usize = 8;
    /// Spaces separating the metrics line's fields, baked into the field widths
    /// above and reused verbatim by the mDNS-only line.
    const METRICS_FIELD_GAP: usize = 3;
    /// The header's ranges and metrics lines plus its status line.
    const SETTINGS_HEADER_HEIGHT: u16 = 3;
    /// The setup screen's content rows: the range checkboxes, the custom and mDNS
    /// rows each preceded by a blank line, and the start button.
    const SETUP_CONTENT_LINE_COUNT: u16 = IanaPortCategory::COUNT as u16 + 5;
    /// The setup body height. Scanning that starts from setup reuses it, and it is
    /// the fallback scanning height before any frame has been rendered.
    const SETUP_BODY_LINE_COUNT: u16 =
        Self::SETTINGS_HEADER_HEIGHT + 1 + Self::SETUP_CONTENT_LINE_COUNT;
    /// Body lines above and below a scanning state's own content: the settings
    /// header, one blank separator, one blank separator, and the footer.
    const SCANNING_BODY_FRAME_LINE_COUNT: u16 = Self::SETTINGS_HEADER_HEIGHT + 3;
    /// First interior row of a state's own content: below the settings header and
    /// its one blank separator.
    const CONTENT_TOP: u16 = Self::SETTINGS_HEADER_HEIGHT + 1;
    /// Interior row of the setup custom-port field, whose text is drawn over the
    /// custom checkbox row.
    const CHECKBOX_BLOCK_TOP_OFFSET: usize = Self::CONTENT_TOP as usize;
    /// The elapsed-time ticker sits at the top of the content, the progress bar
    /// directly below it.
    const PROGRESS_TICKER_ROW: u16 = Self::CONTENT_TOP;
    const PROGRESS_BAR_ROW: u16 = Self::CONTENT_TOP + 1;
    /// The scanning open-port list sits below the ticker, the progress bar, and
    /// one blank separator.
    const SCANNING_LIST_ROW: u16 = Self::CONTENT_TOP + 3;
    /// Content rows above the scanning open-port list: the ticker, the progress
    /// bar, and one blank separator.
    const SCANNING_LIST_TOP_LINE_COUNT: u16 = Self::SCANNING_LIST_ROW - Self::CONTENT_TOP;
    /// Rows the open-port view may occupy before it starts scrolling.
    const MAX_VISIBLE_RESULT_ROWS: usize = 8;
    /// The ticker, the progress bar, the open-count line, and one blank separator,
    /// above the grouped open-port view.
    const RESULTS_FIXED_LINE_COUNT: u16 = 4;
    /// Every results body line except the grouped open-port rows and the optional
    /// mDNS line: the settings header and its blank separator, the fixed lines
    /// above the rows, and the blank separator plus footer below them.
    const RESULTS_NON_LIST_BODY_LINES: u16 =
        Self::SETTINGS_HEADER_HEIGHT + 1 + Self::RESULTS_FIXED_LINE_COUNT + 2;
    /// One blank row above and below the body, plus the two block borders.
    const FRAME_HEIGHT: u16 = 4;
    /// One padding column on each side, plus the two block borders.
    const FRAME_WIDTH: u16 = 4;

    pub(super) fn render(&mut self, frame: &mut Frame, theme: &TableColors, keymap: &KeyBindings) {
        let Some(target) = &self.target else {
            return;
        };
        let title = target.title();

        let results_content = match &self.state {
            PortScanState::Results(result) => Some(Self::results_content_lines(result, theme)),
            _ => None,
        };

        let title_width = title.width() as u16;
        let interior_width = *self.frozen_interior_width.get_or_insert_with(|| {
            let widest_footer = [
                Self::scanning_footer(keymap, theme),
                Self::results_footer(keymap, theme),
            ]
            .iter()
            .map(Line::width)
            .max()
            .unwrap_or(0) as u16;
            // The mDNS result line only appears in the results state, so account for its
            // widest form here to keep the width, frozen from the setup screen, wide enough.
            let widest_mdns_line =
                Self::mdns_result_line(MdnsProbeOutcome::UnicastAndMulticast, theme).width() as u16;
            widest_footer
                .max(title_width)
                .max(widest_mdns_line)
                .max(Self::MIN_INTERIOR_WIDTH)
        });

        let setup_settings;
        let (content, footer, settings, status) = match &self.state {
            PortScanState::Setup => {
                setup_settings = self.setup_settings(target);
                (
                    self.setup_content_lines(theme),
                    None,
                    &setup_settings,
                    self.setup_status_line(theme),
                )
            }
            PortScanState::Scanning(running) => (
                Self::scanning_content_lines(running.body_height),
                Some(Self::scanning_footer(keymap, theme)),
                &running.settings,
                Line::raw(""),
            ),
            PortScanState::Results(result) => (
                Self::results_placeholder_lines(
                    result,
                    results_content.as_ref().map_or(0, Vec::len),
                    Self::results_visible_rows(result, frame.area().height),
                    theme,
                ),
                Some(Self::results_footer(keymap, theme)),
                result.settings(),
                Line::raw(""),
            ),
        };

        let mut body =
            Self::settings_header_lines(settings, status, interior_width, theme).to_vec();
        body.push(Line::raw(""));
        body.extend(content);
        if let Some(footer) = footer {
            body.push(Line::raw(""));
            body.push(footer);
        }
        debug_assert!(
            match &self.state {
                PortScanState::Setup => body.len() == Self::SETUP_BODY_LINE_COUNT as usize,
                PortScanState::Scanning(running) => body.len() == running.body_height as usize,
                PortScanState::Results(_) => true,
            },
            "unexpected body height {len} for the current state",
            len = body.len()
        );
        self.last_body_line_count = body.len() as u16;

        let custom_is_empty = self.custom_input_text().is_empty();

        let width = (interior_width + Self::FRAME_WIDTH).min(frame.area().width);
        let height = (body.len() as u16 + Self::FRAME_HEIGHT).min(frame.area().height);
        let area = util::center(
            frame.area(),
            Constraint::Length(width),
            Constraint::Length(height),
        );

        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(theme.gauge_accent())
            .title(Span::styled(title, theme.gauge_accent()))
            .style(theme.base());
        let interior = block.inner(area);
        frame.render_widget(Clear, area);
        frame.render_widget(block, area);

        let [_, interior] = Layout::vertical([Constraint::Length(1), Constraint::Min(0)])
            .horizontal_margin(1)
            .areas(interior);
        frame.render_widget(
            Paragraph::new(Text::from(body)).style(theme.row()),
            interior,
        );

        match &mut self.state {
            PortScanState::Setup => {
                if self.focus == CUSTOM_ROW && !custom_is_empty {
                    Self::render_custom_input(
                        &mut self.custom_input,
                        self.custom_checked && self.selected_ports.is_err(),
                        interior,
                        frame,
                        theme,
                    );
                }
            }
            PortScanState::Scanning(running) => {
                Self::render_ticker(running.started.elapsed(), interior, frame, theme);
                Self::render_progress_bar(
                    running.scanned,
                    running.settings.port_count,
                    interior,
                    frame,
                    theme,
                );
                let content_line_count = running
                    .body_height
                    .saturating_sub(Self::SCANNING_BODY_FRAME_LINE_COUNT);
                let max_rows =
                    content_line_count.saturating_sub(Self::SCANNING_LIST_TOP_LINE_COUNT) as usize;
                let rows = running.open_ports.len().min(max_rows);
                if rows > 0 {
                    let area = Rect {
                        y: interior.y + Self::SCANNING_LIST_ROW,
                        height: rows as u16,
                        ..interior
                    };
                    Self::render_open_port_list(
                        &running.open_ports,
                        &mut ListState::default(),
                        area,
                        frame,
                        theme,
                    );
                }
            }
            PortScanState::Results(result) => {
                Self::render_progress_bar(
                    result.scanned(),
                    result.settings().port_count,
                    interior,
                    frame,
                    theme,
                );
                let content =
                    results_content.expect("results content computed for the results state");
                let max_rows = Self::results_visible_rows(result, frame.area().height);
                let area = Rect {
                    y: interior.y + Self::results_list_top(result),
                    height: content.len().clamp(1, max_rows) as u16,
                    ..interior
                };
                if area.bottom() > interior.bottom() {
                } else if content.is_empty() {
                    frame.render_widget(
                        Paragraph::new(Line::styled(
                            "No open ports",
                            theme.row().add_modifier(Modifier::DIM),
                        )),
                        area,
                    );
                } else {
                    Self::render_grouped_ports(
                        content,
                        &mut self.results_state,
                        area,
                        frame,
                        theme,
                    );
                }
            }
        }
    }

    /// The header's three fixed-height lines: the scanned ranges, the
    /// worker/timeout/max-duration metrics, and a status line. The height is fixed
    /// across every state so nothing below it shifts.
    fn settings_header_lines(
        settings: &PortScanSettings,
        status: Line<'static>,
        width: u16,
        theme: &TableColors,
    ) -> [Line<'static>; 3] {
        let width = usize::from(width);
        let (ranges, metrics) = match settings.mdns_probe_timeout {
            Some(timeout) if settings.port_count == 0 => (
                "mDNS probe only".to_owned(),
                format!(
                    "{label}{gap}max {duration}",
                    label = Self::MDNS_PORT_LABEL,
                    gap = " ".repeat(Self::METRICS_FIELD_GAP),
                    duration = format::format_duration(timeout),
                ),
            ),
            _ => {
                let max = if settings.port_count == 0 {
                    "—".to_owned()
                } else {
                    format::format_duration(settings.all_filtered_duration())
                };
                (
                    Self::ranges_line(&settings.port_ranges, width),
                    format!(
                        "{workers:<ww$}{timeout:<tw$}max {max}",
                        workers = format!("{n} workers", n = settings.worker_count.get()),
                        timeout = format::format_duration(settings.connect_timeout),
                        ww = Self::HEADER_WORKERS_FIELD,
                        tw = Self::HEADER_TIMEOUT_FIELD,
                    ),
                )
            }
        };
        [
            Line::styled(ranges, theme.row()),
            Line::styled(metrics, theme.row().add_modifier(Modifier::DIM)),
            status,
        ]
    }

    /// The scanned ranges, truncated with `…` when they overflow `width`.
    fn ranges_line(port_ranges: &PortRanges, width: usize) -> String {
        Self::truncate_with_ellipsis(&port_ranges.to_string(), width)
    }

    fn truncate_with_ellipsis(text: &str, max_chars: usize) -> String {
        if text.chars().count() <= max_chars {
            return text.to_owned();
        }
        let kept: String = text.chars().take(max_chars.saturating_sub(1)).collect();
        format!("{kept}…")
    }

    fn setup_settings(&self, target: &PortScanTarget) -> PortScanSettings {
        let port_ranges = self.setup_display_ranges();
        PortScanSettings {
            port_count: port_ranges.port_count(),
            port_ranges,
            worker_count: target.worker_count,
            connect_timeout: target.connect_timeout,
            mdns_probe_timeout: self.mdns_checked.then_some(port_scan::MDNS_PROBE_TIMEOUT),
        }
    }

    /// The ranges shown in the setup header: the merged selection when it parses,
    /// or just the checked range checkboxes while the custom text is invalid.
    fn setup_display_ranges(&self) -> PortRanges {
        match &self.selected_ports {
            Ok(ports) => PortRanges::merged_from(ports.ports()),
            Err(_) => {
                let checked: PortList = IanaPortCategory::iter()
                    .zip(self.ranges_checked)
                    .filter(|(_, checked)| *checked)
                    .flat_map(|(category, _)| category.range())
                    .collect();
                PortRanges::merged_from(checked.ports())
            }
        }
    }

    fn setup_status_line(&self, theme: &TableColors) -> Line<'static> {
        match &self.selected_ports {
            Err(e) => Line::styled(e.to_string(), theme.log_err()),
            Ok(_) => Line::raw(""),
        }
    }

    fn setup_content_lines(&self, theme: &TableColors) -> Vec<Line<'static>> {
        let mut lines: Vec<Line<'static>> = IanaPortCategory::iter()
            .enumerate()
            .map(|(index, category)| {
                let range = category.range();
                self.checkbox_row(
                    index,
                    self.ranges_checked[index],
                    category,
                    format!("{start}-{end}", start = range.start(), end = range.end()),
                    theme.row(),
                    theme,
                )
            })
            .collect();
        lines.push(self.custom_row(theme));
        lines.push(Line::raw(""));
        lines.push(self.checkbox_row(
            MDNS_ROW,
            self.mdns_checked,
            "probe mDNS",
            format!("({label})", label = Self::MDNS_PORT_LABEL),
            theme.row().add_modifier(Modifier::DIM),
            theme,
        ));
        lines.push(Line::raw(""));
        lines.push(self.start_row(theme));
        lines
    }

    /// The checkbox, focus marker, and left-aligned label that begin every setup
    /// row. Unfocused labels sit at normal row prominence, above the dimmer value
    /// hints. The focused label is brightened and bold.
    fn checkbox_prefix(
        &self,
        row: usize,
        checked: bool,
        label: impl std::fmt::Display,
        theme: &TableColors,
    ) -> Span<'static> {
        let focused = self.focus == row;
        let label_style = if focused {
            theme.title().add_modifier(Modifier::BOLD)
        } else {
            theme.row()
        };
        Span::styled(
            format!(
                "{marker}{checkbox} {label:<label_width$}",
                marker = if focused { "▶ " } else { "  " },
                checkbox = if checked { "[*]" } else { "[ ]" },
                label_width = Self::LABEL_COLUMN_WIDTH
            ),
            label_style,
        )
    }

    fn checkbox_row(
        &self,
        row: usize,
        checked: bool,
        label: impl std::fmt::Display,
        value: String,
        value_style: Style,
        theme: &TableColors,
    ) -> Line<'static> {
        Line::from(vec![
            self.checkbox_prefix(row, checked, label, theme),
            Span::styled(value, value_style),
        ])
    }

    fn custom_row(&self, theme: &TableColors) -> Line<'static> {
        let prefix = self.checkbox_prefix(CUSTOM_ROW, self.custom_checked, "Custom", theme);
        let text = self.custom_input_text();
        let mut spans = vec![prefix];
        if text.is_empty() {
            spans.extend(self.placeholder_spans(theme));
        } else {
            let style = if self.custom_checked && self.selected_ports.is_err() {
                theme.log_err()
            } else {
                theme.text_input_text()
            };
            spans.push(Span::styled(text.to_owned(), style));
        }
        Line::from(spans)
    }

    /// The dimmed custom-port example. When the row is focused, its first
    /// character is rendered with the text-cursor style, overlaying the example
    /// in place.
    fn placeholder_spans(&self, theme: &TableColors) -> Vec<Span<'static>> {
        let example = theme.row().add_modifier(Modifier::DIM);
        if self.focus != CUSTOM_ROW {
            return vec![Span::styled(Self::CUSTOM_PLACEHOLDER, example)];
        }
        let cursor = theme.row().add_modifier(Modifier::REVERSED);
        match Self::CUSTOM_PLACEHOLDER.split_at_checked(1) {
            Some((first, rest)) => vec![Span::styled(first, cursor), Span::styled(rest, example)],
            None => vec![Span::styled(" ", cursor)],
        }
    }

    fn start_row(&self, theme: &TableColors) -> Line<'static> {
        let focused = self.focus == START_ROW;
        let style = if !self.can_start() {
            theme.row().add_modifier(Modifier::DIM)
        } else if focused {
            theme.title().add_modifier(Modifier::BOLD)
        } else {
            theme.row()
        };
        Line::styled(
            format!(
                "{marker}[ Start scan ]",
                marker = if focused { "▶ " } else { "  " }
            ),
            style,
        )
    }

    /// Fixed-height placeholder rows the gauge and open-port list are drawn over,
    /// sized so the scanning body matches the height the scan started from.
    fn scanning_content_lines(body_height: u16) -> Vec<Line<'static>> {
        let count = body_height.saturating_sub(Self::SCANNING_BODY_FRAME_LINE_COUNT) as usize;
        iter::repeat_n(Line::raw(""), count).collect()
    }

    /// The frozen elapsed ticker, one blank row for the frozen progress bar drawn
    /// over it, the open-count and optional mDNS lines, one blank separator, then
    /// placeholder rows for the grouped open-port view rendered over them.
    fn results_placeholder_lines(
        result: &PortScanResult,
        content_len: usize,
        max_rows: usize,
        theme: &TableColors,
    ) -> Vec<Line<'static>> {
        let mut lines = vec![
            Self::ticker_line(result.duration(), result.was_cancelled(), theme),
            Line::raw(""),
            Self::open_count_line(result, theme),
        ];
        if let Some(mdns) = result.mdns() {
            lines.push(Self::mdns_result_line(mdns, theme));
        }
        lines.push(Line::raw(""));
        let content_rows = content_len.clamp(1, max_rows);
        lines.extend(iter::repeat_n(Line::raw(""), content_rows));
        lines
    }

    /// The open-port rows the results view shows at once: the configured maximum,
    /// reduced so the dialog and its footer still fit `frame_height`.
    fn results_visible_rows(result: &PortScanResult, frame_height: u16) -> usize {
        // Only FRAME_HEIGHT - 1 rows are required around the body: one of
        // FRAME_HEIGHT's rows is spare bottom padding that render()'s height clamp
        // drops before it cuts any body line.
        let chrome = Self::RESULTS_NON_LIST_BODY_LINES
            + u16::from(result.mdns().is_some())
            + (Self::FRAME_HEIGHT - 1);
        let available = frame_height.saturating_sub(chrome) as usize;
        Self::MAX_VISIBLE_RESULT_ROWS.min(available).max(1)
    }

    /// The elapsed-time ticker shown above the progress bar in both the scanning
    /// and results states, with a dim ` cancelled` suffix when the scan was
    /// stopped early.
    fn ticker_line(elapsed: Duration, cancelled: bool, theme: &TableColors) -> Line<'static> {
        let mut spans = vec![Span::styled(format::format_duration(elapsed), theme.row())];
        if cancelled {
            spans.push(Span::styled(
                "  cancelled",
                theme.row().add_modifier(Modifier::DIM),
            ));
        }
        Line::from(spans)
    }

    /// The open-port count, plus the filtered count only when some ports were
    /// filtered.
    fn open_count_line(result: &PortScanResult, theme: &TableColors) -> Line<'static> {
        let open = result.open_count();
        let filtered = result.filtered();
        let filtered_suffix = if filtered > 0 {
            format!(" ({filtered} filtered)")
        } else {
            String::new()
        };
        let summary = format!(
            " open port{plural}{filtered_suffix}",
            plural = if open == 1 { "" } else { "s" }
        );
        Line::from(vec![
            Span::styled(open.to_string(), theme.success()),
            Span::styled(summary, theme.row()),
        ])
    }

    /// The grouped, two-column open-port view: a table per [`IanaPortCategory`]
    /// that has open ports, each headed by its category name when more than one
    /// category was scanned. Empty when the scan found no open ports.
    fn results_content_lines(result: &PortScanResult, theme: &TableColors) -> Vec<Line<'static>> {
        let open_ports = result.open_ports();
        if open_ports.is_empty() {
            return Vec::new();
        }
        let show_headers = result.scanned_categories().len() > 1;
        let mut lines: Vec<Line<'static>> = Vec::new();
        for (category, ports) in IanaPortCategory::group_ports_by_category(open_ports) {
            if show_headers {
                lines.push(Line::styled(
                    category.to_string(),
                    theme.row().add_modifier(Modifier::DIM),
                ));
            }
            lines.extend(Self::two_column_port_rows(&ports, theme));
        }
        lines
    }

    /// Lays `ports` out down two columns, filling the left column first, so the
    /// port and service-label columns of each half line up.
    fn two_column_port_rows(ports: &[u16], theme: &TableColors) -> Vec<Line<'static>> {
        let dim = theme.row().add_modifier(Modifier::DIM);
        let left_count = ports.len().div_ceil(2);
        let (left, right) = ports.split_at(left_count);
        let port_width = ports
            .iter()
            .map(|port| port.to_string().len())
            .max()
            .unwrap_or(1);
        let label_width = left
            .iter()
            .map(|port| Self::service_label(*port).len())
            .max()
            .unwrap_or(0);

        (0..left_count)
            .map(|row| {
                let mut spans = vec![
                    Span::styled(
                        format!("  {port:<port_width$}  ", port = left[row]),
                        theme.success(),
                    ),
                    Span::styled(
                        format!(
                            "{label:<label_width$}",
                            label = Self::service_label(left[row])
                        ),
                        dim,
                    ),
                ];
                if let Some(port) = right.get(row) {
                    spans.push(Span::styled(
                        format!("  {port:<port_width$}  "),
                        theme.success(),
                    ));
                    spans.push(Span::styled(Self::service_label(*port), dim));
                }
                Line::from(spans)
            })
            .collect()
    }

    fn service_label(port: u16) -> &'static str {
        port_scan::service_label(port).unwrap_or("-")
    }

    fn mdns_result_line(mdns: MdnsProbeOutcome, theme: &TableColors) -> Line<'static> {
        let dim = theme.row().add_modifier(Modifier::DIM);
        let (status, style) = match mdns {
            MdnsProbeOutcome::Silent => ("no response", dim),
            MdnsProbeOutcome::Unicast => ("responding (unicast)", theme.success()),
            MdnsProbeOutcome::Multicast => ("responding (multicast)", theme.success()),
            MdnsProbeOutcome::UnicastAndMulticast => {
                ("responding (unicast & multicast)", theme.success())
            }
            MdnsProbeOutcome::NotApplicable => ("n/a (IPv6 host)", dim),
        };
        Line::styled(
            format!("mDNS {label}: {status}", label = Self::MDNS_PORT_LABEL),
            style,
        )
    }

    /// Interior row of the grouped open-port view, below the summary lines, the
    /// optional mDNS result line, and one blank separator.
    fn results_list_top(result: &PortScanResult) -> u16 {
        Self::CONTENT_TOP + Self::RESULTS_FIXED_LINE_COUNT + u16::from(result.mdns().is_some())
    }

    fn scanning_footer(keymap: &KeyBindings, theme: &TableColors) -> Line<'static> {
        util::key_hint_footer(
            theme,
            &[(keymap.get_key_display_for_action(Action::Close), "cancel")],
        )
    }

    fn results_footer(keymap: &KeyBindings, theme: &TableColors) -> Line<'static> {
        util::key_hint_footer(
            theme,
            &[
                (
                    keymap.get_key_display_for_action(Action::NavigateSelect),
                    "back",
                ),
                (keymap.get_key_display_for_action(Action::Refresh), "rescan"),
            ],
        )
    }

    fn render_custom_input(
        custom_input: &mut TextArea<'static>,
        is_invalid: bool,
        interior: Rect,
        frame: &mut Frame,
        theme: &TableColors,
    ) {
        let x_offset = Self::VALUE_COLUMN_X as u16;
        let custom_row_y = interior.y + (Self::CHECKBOX_BLOCK_TOP_OFFSET + CUSTOM_ROW) as u16;
        if x_offset >= interior.width || custom_row_y >= interior.bottom() {
            return;
        }
        let area = Rect::new(
            interior.x + x_offset,
            custom_row_y,
            interior.width - x_offset,
            1,
        );

        custom_input.set_style(if is_invalid {
            theme.log_err()
        } else {
            theme.text_input_text()
        });
        frame.render_widget(Clear, area);
        frame.render_widget(&*custom_input, area);
    }

    fn render_ticker(elapsed: Duration, interior: Rect, frame: &mut Frame, theme: &TableColors) {
        frame.render_widget(
            Paragraph::new(Self::ticker_line(elapsed, false, theme)),
            Rect {
                y: interior.y + Self::PROGRESS_TICKER_ROW,
                height: 1,
                ..interior
            },
        );
    }

    fn render_progress_bar(
        scanned: u32,
        total: usize,
        interior: Rect,
        frame: &mut Frame,
        theme: &TableColors,
    ) {
        let ratio = if total == 0 {
            0.0
        } else {
            (f64::from(scanned) / total as f64).clamp(0.0, 1.0)
        };
        let label = Span::styled(
            format!("{scanned}/{total}"),
            theme
                .gauge_label()
                .add_modifier(Modifier::ITALIC | Modifier::BOLD),
        );
        let gauge = Gauge::default()
            .style(theme.gauge_bg())
            .gauge_style(theme.gauge_fill())
            .ratio(ratio)
            .label(label);
        frame.render_widget(
            gauge,
            Rect {
                y: interior.y + Self::PROGRESS_BAR_ROW,
                height: 1,
                ..interior
            },
        );
    }

    /// Renders the single-column port-and-service-label list the scanning view
    /// fills as ports are found.
    fn render_open_port_list(
        open_ports: &[u16],
        list_state: &mut ListState,
        area: Rect,
        frame: &mut Frame,
        theme: &TableColors,
    ) {
        let items: Vec<ListItem> = open_ports
            .iter()
            .map(|port| {
                ListItem::new(Line::from(vec![
                    Span::styled(format!("{port:<8}"), theme.success()),
                    Span::styled(
                        Self::service_label(*port),
                        theme.row().add_modifier(Modifier::DIM),
                    ),
                ]))
            })
            .collect();
        let list = List::new(items)
            .style(theme.row())
            .highlight_style(theme.list_highlight())
            .highlight_spacing(HighlightSpacing::Never);
        frame.render_stateful_widget(list, area, list_state);
    }

    /// Renders the grouped open-port lines as a scrollable list, adding a
    /// scrollbar when they do not all fit in `area`.
    fn render_grouped_ports(
        content: Vec<Line<'static>>,
        list_state: &mut ListState,
        area: Rect,
        frame: &mut Frame,
        theme: &TableColors,
    ) {
        let total = content.len();
        let items: Vec<ListItem> = content.into_iter().map(ListItem::new).collect();
        let list = List::new(items)
            .style(theme.row())
            .highlight_style(theme.list_highlight())
            .highlight_spacing(HighlightSpacing::Never);
        frame.render_stateful_widget(list, area, list_state);

        if total > area.height as usize {
            let scrollbar = Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None);
            let mut scrollbar_state = ScrollbarState::new(total)
                .position(list_state.selected().unwrap_or(0))
                .viewport_content_length(area.height as usize);
            frame.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
        }
    }
}

#[cfg(any(test, feature = "test-utils"))]
impl PortScanPopup {
    /// Elapsed time a stubbed scan reports, so that the duration in its results
    /// is the same on every run.
    const STUBBED_SCAN_DURATION: Duration = Duration::from_secs(100);
    const STUBBED_CONNECT_TIMEOUT: Duration = Duration::from_millis(100);
    const STUBBED_WORKER_COUNT: NonZero<u16> = NonZero::new(64).unwrap();

    /// Puts the dialog into its scanning state with no scanner thread behind it,
    /// so a test can feed the updates itself. The scan covers exactly the given
    /// [`IanaPortCategory`]s, which set both the port count and the header ranges.
    pub(super) fn start_stubbed_scan(
        &mut self,
        scanned_categories: Vec<IanaPortCategory>,
    ) -> Sender<PortScanUpdate> {
        let (job, updates_tx) = PortScanJob::stub();
        let (worker_count, connect_timeout) = self.target.as_ref().map_or(
            (Self::STUBBED_WORKER_COUNT, Self::STUBBED_CONNECT_TIMEOUT),
            |target| (target.worker_count, target.connect_timeout),
        );
        let scanned_ports: PortList = scanned_categories
            .iter()
            .flat_map(|category| category.range())
            .collect();
        let now = Instant::now();
        self.state = PortScanState::Scanning(RunningPortScan {
            job,
            settings: PortScanSettings {
                port_count: scanned_ports.len(),
                port_ranges: PortRanges::merged_from(scanned_ports.ports()),
                worker_count,
                connect_timeout,
                mdns_probe_timeout: None,
            },
            scanned_categories,
            scanned: 0,
            open_ports: Vec::new(),
            counts: PortScanOutcomeCounts {
                closed: 0,
                filtered: 0,
            },
            mdns_outcome: None,
            started: now.checked_sub(Self::STUBBED_SCAN_DURATION).unwrap_or(now),
            body_height: self.last_body_line_count,
        });
        updates_tx
    }
}
