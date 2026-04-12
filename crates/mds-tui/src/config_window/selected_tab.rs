use mds_config::{AppConfig, config_type::ConfigType, shared_config::SharedConfig};
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

use super::cfg_picker_state::{CfgPickerState, OVERLAY_X_OFFSET, OVERLAY_Y_OFFSET_FROM_ITEM};
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

        let list = picker.cfg.modify(|cfg| {
            let items = (items_fn)(cfg);
            List::new(items)
                .block(block)
                .style(theme.row())
                .highlight_style(theme.list_highlight())
                .highlight_symbol("> ")
                .highlight_spacing(HighlightSpacing::Always)
        });
        StatefulWidget::render(list, area, buf, &mut picker.state);

        Self::render_txt_edit(picker, &area, buf, theme);
        Self::render_option_selector(picker, &area, buf, theme);

        let Some(selected) = picker.state.selected() else {
            return;
        };

        // Suppress the doc popup when the selector overlay already covers that area.
        if picker.option_selector.is_none() {
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
    ) {
        let Some(txt_edit) = picker.txt_edit.as_mut() else {
            return;
        };
        let selected = picker.state.selected().unwrap_or(0);
        let y_offset = selected as u16 + OVERLAY_Y_OFFSET_FROM_ITEM;

        let content_width = text_edit_content_len(txt_edit) + 4;
        let available_width = area.width.saturating_sub(OVERLAY_X_OFFSET);
        const MIN_WIDTH: u16 = 10;
        if MIN_WIDTH > available_width {
            return;
        }
        let width = content_width.clamp(MIN_WIDTH, available_width);
        const HEIGHT: u16 = 2;
        let pos = area.as_position();
        let rect = Rect::new(pos.x + OVERLAY_X_OFFSET, pos.y + y_offset, width, HEIGHT);

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
    ) {
        if let Some(sel) = picker.option_selector.as_mut() {
            let selected = picker.state.selected().unwrap_or(0);
            sel.render(
                selected,
                OVERLAY_X_OFFSET,
                OVERLAY_Y_OFFSET_FROM_ITEM,
                area,
                buf,
                theme,
            );
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
        match item {
            ConfigType::Toggle { description, .. }
            | ConfigType::NumberNonZeroU16 { description, .. }
            | ConfigType::Numberu32 { description, .. }
            | ConfigType::ScanIoThreads { description, .. }
            | ConfigType::StringSelect { description, .. } => {
                Self::render_doc_paragraph_inner(
                    description,
                    selected,
                    1,
                    area,
                    buf,
                    Borders::RIGHT,
                    theme,
                );
            }
            ConfigType::NumberList { description, .. }
            | ConfigType::RegexStringList { description, .. } => {
                Self::render_doc_paragraph_inner(
                    description,
                    selected,
                    3,
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
        selected: usize,
        y_offset: u16,
        area: &Rect,
        buf: &mut Buffer,
        borders: Borders,
        theme: &TableColors,
    ) {
        let mut max_line_width = 0;
        let num_lines = description.lines().count() as u16;
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
        let x_offset = (selected + 40) as u16;
        let y_offset = selected as u16 + y_offset;
        let available_width = area.width.saturating_sub(x_offset);
        let available_height = area.height.saturating_sub(y_offset);
        const MIN_WIDTH: u16 = 10;
        if MIN_WIDTH > available_width {
            return;
        }
        let width = (max_line_width as u16).clamp(MIN_WIDTH, available_width);
        const MIN_HEIGHT: u16 = 1;
        if MIN_HEIGHT > available_height {
            return;
        }
        let extra_height = if num_lines < 3 {
            1
        } else if num_lines < 6 {
            2
        } else {
            3
        };
        let height = (num_lines + extra_height).clamp(MIN_HEIGHT, available_height);
        let pos = area.as_position();
        let rect = Rect::new(pos.x + x_offset, pos.y + y_offset, width, height);
        Clear.render(rect, buf);
        doc_p.render(rect, buf);
    }
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
