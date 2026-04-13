use mds_config::{
    AppConfig,
    config_type::{ConfigType, KEY_STR_LEN},
    shared_config::SharedConfig,
};
use mds_keybindings::{Action, KeyBindings};
use ratatui::{
    buffer::Buffer,
    crossterm::event::KeyEvent,
    layout::Rect,
    style::{Stylize, palette::tailwind},
    symbols,
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, HighlightSpacing, List, Padding, Paragraph, StatefulWidget, Widget,
        Wrap,
    },
};

use crate::table_pane::TableColors;

use strum::Display;

/// Columns occupied before the key text in each row: inner padding plus the highlight symbol.
const ITEM_PREFIX_WIDTH: usize = 3;
/// Minimum horizontal gap between the end of a value and the left edge of the doc popup.
const DOC_GAP: usize = 3;
/// Rows between the selected item's value row and the doc popup for simple (non-list) items.
const DOC_SIMPLE_ROW_GAP: u16 = 0;
/// Rows between the selected item's value row and the doc popup for list-value items.
/// List items show an edit box directly below, so the doc is pushed further down.
const DOC_LIST_ROW_GAP: u16 = 2;
/// Rows between the bottom of the selected item and the edit/selector overlay.
const OVERLAY_GAP_BELOW_ITEM: u16 = 1;
/// Minimum width for the doc popup.
const DOC_MIN_WIDTH: u16 = 10;
/// Minimum height for the doc popup.
const DOC_MIN_HEIGHT: u16 = 1;
/// Descriptions shorter than this many lines get extra popup height (very short text looks cramped).
const DOC_SHORT_LINE_THRESHOLD: u16 = 3;
/// Descriptions shorter than this many lines get a moderate height increase.
const DOC_MEDIUM_LINE_THRESHOLD: u16 = 6;
/// Extra rows added to the popup height for very short descriptions.
const DOC_EXTRA_HEIGHT_SHORT: u16 = 1;
/// Extra rows added to the popup height for medium-length descriptions.
const DOC_EXTRA_HEIGHT_MEDIUM: u16 = 2;
/// Extra rows added to the popup height for long descriptions.
const DOC_EXTRA_HEIGHT_TALL: u16 = 3;

use super::cfg_picker_state::{CfgPickerState, OVERLAY_X_OFFSET};
use crate::{error_box::ErrorBox, message::Message, util::text_edit_content_len};

#[derive(Clone, Display)]
pub(crate) enum SelectedTab<'t> {
    #[strum(to_string = "Interfaces")]
    Interfaces(CfgPickerState<'t>),
    #[strum(to_string = "Scan")]
    Scan(CfgPickerState<'t>),
    #[strum(to_string = "Timeouts")]
    Timeouts(CfgPickerState<'t>),
    #[strum(to_string = "UI")]
    Ui(CfgPickerState<'t>),
}

impl<'t> SelectedTab<'t> {
    // These are the only places that match on the enum variant.  All logic
    // above this section goes through picker_ref / picker_mut.

    fn picker_ref(&self) -> &CfgPickerState<'t> {
        match self {
            Self::Interfaces(p) | Self::Scan(p) | Self::Timeouts(p) | Self::Ui(p) => p,
        }
    }

    fn picker_mut(&mut self) -> &mut CfgPickerState<'t> {
        match self {
            Self::Interfaces(p) | Self::Scan(p) | Self::Timeouts(p) | Self::Ui(p) => p,
        }
    }

    pub(super) fn discriminant(&self) -> usize {
        match self {
            Self::Interfaces(_) => 0,
            Self::Scan(_) => 1,
            Self::Timeouts(_) => 2,
            Self::Ui(_) => 3,
        }
    }

    pub(super) const fn palette(&self) -> tailwind::Palette {
        match self {
            Self::Interfaces(_) => tailwind::BLUE,
            Self::Scan(_) => tailwind::EMERALD,
            Self::Timeouts(_) => tailwind::CYAN,
            Self::Ui(_) => tailwind::YELLOW,
        }
    }

    fn from_repr(discriminant: usize, cfg: SharedConfig) -> Option<Self> {
        match discriminant {
            0 => Some(Self::Interfaces(CfgPickerState::new(cfg, interfaces_items))),
            1 => Some(Self::Scan(CfgPickerState::new(cfg, scan_items))),
            2 => Some(Self::Timeouts(CfgPickerState::new(cfg, timeouts_items))),
            3 => Some(Self::Ui(CfgPickerState::new(cfg, ui_items))),
            _ => None,
        }
    }

    /// Create the default starting tab (Interfaces).
    pub(super) fn initial(cfg: SharedConfig) -> Self {
        Self::Interfaces(CfgPickerState::new(cfg, interfaces_items))
    }

    pub(super) fn navigate_up(&mut self) {
        let p = self.picker_mut();
        if p.selector_open() {
            p.navigate_selector_up();
        } else {
            p.txt_edit = None;
            p.state.select_previous();
        }
    }

    pub(super) fn navigate_down(&mut self) {
        let p = self.picker_mut();
        if p.selector_open() {
            p.navigate_selector_down();
        } else {
            p.txt_edit = None;
            p.state.select_next();
        }
    }

    pub(super) fn navigate_select(&mut self) -> Result<Option<Message>, ErrorBox> {
        let p = self.picker_mut();
        if p.selector_open() {
            p.confirm_selector();
            return Ok(None);
        }
        if p.try_open_selector() {
            p.apply_selector_preview();
            return Ok(None);
        }
        p.handle_selected_item()?;
        Ok(None)
    }

    pub(super) fn txt_edit_open(&self) -> bool {
        self.picker_ref().txt_edit.is_some()
    }

    pub(super) fn selector_open(&self) -> bool {
        self.picker_ref().selector_open()
    }

    pub(super) fn close_txt_edit(&mut self) {
        self.picker_mut().txt_edit = None;
    }

    pub(super) fn input(
        &mut self,
        keymap: &KeyBindings,
        key: KeyEvent,
    ) -> Result<Option<Message>, ErrorBox> {
        // Selector captures all input while open.
        if self.selector_open() {
            let p = self.picker_mut();
            match keymap.handle_key(key) {
                Some(Action::NavigateUp) => p.navigate_selector_up(),
                Some(Action::NavigateDown) => p.navigate_selector_down(),
                Some(Action::NavigateSelect) => p.confirm_selector(),
                Some(Action::Close) => p.cancel_selector(),
                _ => {}
            }
            return Ok(None);
        }

        match keymap.handle_key(key) {
            Some(Action::NavigateSelect) => {
                self.navigate_select()?;
            }
            Some(Action::NavigateUp) if !self.txt_edit_open() => self.navigate_up(),
            Some(Action::NavigateDown) if !self.txt_edit_open() => self.navigate_down(),
            Some(Action::Close) => {
                if self.txt_edit_open() {
                    self.picker_mut().txt_edit = None;
                } else {
                    return Ok(Some(Action::Close.into()));
                }
            }
            _ => {
                if let Some(txt_edit) = self.picker_mut().txt_edit.as_mut() {
                    _ = txt_edit.input(key);
                }
            }
        }
        Ok(None)
    }

    pub(super) fn title_lines() -> [Line<'static>; 4] {
        [
            "  Interfaces  "
                .to_string()
                .fg(tailwind::SLATE.c200)
                .bg(tailwind::BLUE.c900)
                .into(),
            "  Scan  "
                .to_string()
                .fg(tailwind::SLATE.c200)
                .bg(tailwind::EMERALD.c900)
                .into(),
            "  Timeouts  "
                .to_string()
                .fg(tailwind::SLATE.c200)
                .bg(tailwind::CYAN.c900)
                .into(),
            "  UI  "
                .to_string()
                .fg(tailwind::SLATE.c200)
                .bg(tailwind::YELLOW.c900)
                .into(),
        ]
    }

    pub(super) fn previous(&self, cfg: SharedConfig) -> Self {
        let prev = self.discriminant().saturating_sub(1);
        Self::from_repr(prev, cfg).unwrap_or(self.clone())
    }

    pub(super) fn next(&self, cfg: SharedConfig) -> Self {
        let next = self.discriminant().saturating_add(1);
        Self::from_repr(next, cfg).unwrap_or(self.clone())
    }

    pub(super) fn toggle(&self, cfg: SharedConfig) -> Self {
        let next = self.discriminant().saturating_add(1);
        Self::from_repr(next, cfg.clone()).unwrap_or_else(|| Self::from_repr(0, cfg).unwrap())
    }

    pub(super) fn render_ref(&mut self, area: Rect, buf: &mut Buffer, theme: &TableColors) {
        let block = self.block(theme);
        Self::render_tab(block, area, buf, self.picker_mut(), theme);
    }

    fn block(&self, theme: &TableColors) -> Block<'static> {
        Block::bordered()
            .border_set(symbols::border::PROPORTIONAL_TALL)
            .padding(Padding::horizontal(1))
            .border_style(self.palette().c700)
            .style(theme.base())
    }

    fn render_tab(
        block: Block<'_>,
        area: Rect,
        buf: &mut Buffer,
        picker: &mut CfgPickerState,
        theme: &TableColors,
    ) {
        let items_fn = picker.items_fn;
        let selected = picker.state.selected().unwrap_or(0);

        let (list, selected_start_row, item_row_height) = picker.cfg.modify(|cfg| {
            let items = (items_fn)(cfg);
            let start_row: u16 = items[..selected].iter().map(|it| it.row_height()).sum();
            let row_height = items.get(selected).map_or(1, ConfigType::row_height);
            let list = List::new(items)
                .block(block)
                .style(theme.row())
                .highlight_style(theme.list_highlight())
                .highlight_symbol("> ")
                .highlight_spacing(HighlightSpacing::Always);
            (list, start_row, row_height)
        });
        StatefulWidget::render(list, area, buf, &mut picker.state);

        // Place overlays one row below the selected item's last row.
        let overlay_y = selected_start_row + item_row_height + OVERLAY_GAP_BELOW_ITEM;
        Self::render_txt_edit(picker, &area, buf, theme, overlay_y);
        Self::render_option_selector(picker, &area, buf, theme, overlay_y);

        let Some(selected) = picker.state.selected() else {
            return;
        };

        // Suppress the doc popup while an edit or selector overlay is active.
        if picker.option_selector.is_none() && picker.txt_edit.is_none() {
            picker.cfg.modify(|cfg| {
                let items = (items_fn)(cfg);
                Self::render_doc_paragraph(&items, selected, &area, buf, theme);
            });
        }
    }

    fn render_txt_edit(
        picker: &mut CfgPickerState,
        area: &Rect,
        buf: &mut Buffer,
        theme: &TableColors,
        overlay_y: u16,
    ) {
        let Some(txt_edit) = picker.txt_edit.as_mut() else {
            return;
        };
        let content_width = text_edit_content_len(txt_edit) + 4;
        // Place the left border one column before the value so the box content aligns with the value.
        let x = OVERLAY_X_OFFSET.saturating_sub(1);
        let available_width = area.width.saturating_sub(x);
        // Enough room for two borders, one character of content, a cursor, and one space of margin.
        const MIN_WIDTH: u16 = 5;
        if MIN_WIDTH > available_width {
            return;
        }
        let width = content_width.clamp(MIN_WIDTH, available_width);
        const HEIGHT: u16 = 2;
        let pos = area.as_position();
        let rect = Rect::new(pos.x + x, pos.y + overlay_y, width, HEIGHT);

        txt_edit.set_block(
            Block::default()
                .borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM)
                .style(theme.base())
                .border_style(theme.text_input_border()),
        );
        txt_edit.set_style(theme.text_input_text());
        Clear.render(rect, buf);
        txt_edit.render(rect, buf);
    }

    fn render_option_selector(
        picker: &mut CfgPickerState,
        area: &Rect,
        buf: &mut Buffer,
        theme: &TableColors,
        overlay_y: u16,
    ) {
        if let Some(sel) = picker.option_selector.as_mut() {
            sel.render(overlay_y, OVERLAY_X_OFFSET, area, buf, theme);
        }
    }

    fn render_doc_paragraph(
        items: &[ConfigType<'_>],
        selected: usize,
        area: &Rect,
        buf: &mut Buffer,
        theme: &TableColors,
    ) {
        let Some(item) = items.get(selected) else {
            return;
        };
        let x_offset = (ITEM_PREFIX_WIDTH + KEY_STR_LEN + item.value_str().len() + DOC_GAP) as u16;
        let item_start: u16 = items[..selected].iter().map(|it| it.row_height()).sum();
        let item_height = item.row_height();
        match item {
            ConfigType::Toggle { description, .. }
            | ConfigType::NumberNonZeroU16 { description, .. }
            | ConfigType::Numberu32 { description, .. }
            | ConfigType::ScanIoThreads { description, .. }
            | ConfigType::StringSelect { description, .. } => {
                let base_y = item_start + item_height + DOC_SIMPLE_ROW_GAP;
                let abs_y = compute_doc_y(
                    items,
                    selected,
                    x_offset,
                    base_y,
                    doc_popup_height(description),
                );
                Self::render_doc_paragraph_inner(
                    description,
                    x_offset,
                    abs_y,
                    area,
                    buf,
                    Borders::RIGHT,
                    theme,
                );
            }
            ConfigType::NumberList { description, .. }
            | ConfigType::RegexStringList { description, .. } => {
                let base_y = item_start + item_height + DOC_LIST_ROW_GAP;
                let abs_y = compute_doc_y(
                    items,
                    selected,
                    x_offset,
                    base_y,
                    doc_popup_height(description),
                );
                Self::render_doc_paragraph_inner(
                    description,
                    x_offset,
                    abs_y,
                    area,
                    buf,
                    Borders::RIGHT | Borders::BOTTOM,
                    theme,
                );
            }
        }
    }

    fn render_doc_paragraph_inner(
        description: &str,
        x_offset: u16,
        abs_y: u16,
        area: &Rect,
        buf: &mut Buffer,
        borders: Borders,
        theme: &TableColors,
    ) {
        let mut max_line_width = 0;
        let mut lines = vec![];
        for line in description.lines() {
            max_line_width = line.len().max(max_line_width);
            lines.push(Line::from(Span::styled(line, theme.config_doc())));
        }
        let doc_p = Paragraph::new(lines)
            .left_aligned()
            .wrap(Wrap { trim: true })
            .block(
                Block::default()
                    .borders(borders)
                    .border_style(theme.config_doc())
                    .style(theme.base()),
            );
        let available_width = area.width.saturating_sub(x_offset);
        let available_height = area.height.saturating_sub(abs_y);
        if DOC_MIN_WIDTH > available_width {
            return;
        }
        let width = (max_line_width as u16).clamp(DOC_MIN_WIDTH, available_width);
        if DOC_MIN_HEIGHT > available_height {
            return;
        }
        let height = doc_popup_height(description).clamp(DOC_MIN_HEIGHT, available_height);
        let pos = area.as_position();
        let rect = Rect::new(pos.x + x_offset, pos.y + abs_y, width, height);
        Clear.render(rect, buf);
        doc_p.render(rect, buf);
    }
}

/// Compute the height (in rows) of the doc popup for the given description.
fn doc_popup_height(description: &str) -> u16 {
    let num_lines = description.lines().count() as u16;
    let extra = if num_lines < DOC_SHORT_LINE_THRESHOLD {
        DOC_EXTRA_HEIGHT_SHORT
    } else if num_lines < DOC_MEDIUM_LINE_THRESHOLD {
        DOC_EXTRA_HEIGHT_MEDIUM
    } else {
        DOC_EXTRA_HEIGHT_TALL
    };
    num_lines + extra
}

/// Compute the absolute row (within the list's inner area) at which the doc popup should start.
///
/// Starting from `base_y`, the popup is shifted down until it clears every subsequent item
/// whose content would extend into the popup's column range. Items that occupy two rows
/// (wrapped keys) are handled correctly.
fn compute_doc_y(
    items: &[ConfigType<'_>],
    selected: usize,
    x_offset: u16,
    base_y: u16,
    popup_height: u16,
) -> u16 {
    // Precompute the starting row of each item within the list's inner area.
    let item_rows: Vec<u16> = {
        let mut acc = 0u16;
        items
            .iter()
            .map(|it| {
                let start = acc;
                acc += it.row_height();
                start
            })
            .collect()
    };

    let mut y = base_y;
    loop {
        let popup_start = y.saturating_sub(1);
        let popup_end = y + popup_height - 2;

        let last_conflict = items[(selected + 1)..]
            .iter()
            .enumerate()
            .filter_map(|(i, item)| {
                let j = selected + 1 + i;
                let row_start = item_rows[j];
                let row_end = row_start + item.row_height() - 1;
                let overlaps = row_start <= popup_end && row_end >= popup_start;
                if !overlaps {
                    return None;
                }
                let item_width = (ITEM_PREFIX_WIDTH + KEY_STR_LEN + item.value_str().len()) as u16;
                if item_width > x_offset { Some(j) } else { None }
            })
            .next_back();

        match last_conflict {
            None => break,
            Some(j) => {
                let new_y = item_rows[j] + items[j].row_height() + 1;
                if new_y <= y {
                    break;
                }
                y = new_y;
            }
        }
    }
    y
}

// One function per config section, stored as ItemsFn inside CfgPickerState.
fn interfaces_items(cfg: &mut AppConfig) -> Vec<ConfigType<'_>> {
    cfg.interfaces.items()
}
fn scan_items(cfg: &mut AppConfig) -> Vec<ConfigType<'_>> {
    cfg.scan.items()
}
fn timeouts_items(cfg: &mut AppConfig) -> Vec<ConfigType<'_>> {
    cfg.timeouts.items()
}
fn ui_items(cfg: &mut AppConfig) -> Vec<ConfigType<'_>> {
    cfg.ui.items()
}
