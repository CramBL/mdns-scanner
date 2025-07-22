use mds_config::{AppConfig, config_type::ConfigType, shared_config::SharedConfig};
use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent},
    layout::Rect,
    style::{Color, Style, Stylize, palette::tailwind},
    symbols,
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, HighlightSpacing, List, ListState, Padding, Paragraph,
        StatefulWidget, Widget, Wrap,
    },
};

use strum::Display;

use super::cfg_picker_state::CfgPickerState;
use crate::{error_box::ErrorBox, util::text_edit_content_len};

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
    pub(super) fn render_ref(&mut self, area: Rect, buf: &mut Buffer) {
        let block = self.block();
        match self {
            Self::Interfaces(cfg) => Self::render_interfaces_tab(block, area, buf, cfg),
            Self::Scan(cfg) => Self::render_scan_tab(block, area, buf, cfg),
            Self::Timeouts(cfg) => Self::render_timeouts_tab(block, area, buf, cfg),
            Self::Ui(cfg) => Self::render_ui_tab(block, area, buf, cfg),
        }
    }

    pub(super) fn input(&mut self, key: KeyEvent) -> Result<(), ErrorBox> {
        match key.code {
            KeyCode::Char(' ') | KeyCode::Enter => match self {
                SelectedTab::Interfaces(state) => {
                    state.handle_selected_item(|cfg| cfg.interfaces.items())?
                }
                SelectedTab::Scan(state) => state.handle_selected_item(|cfg| cfg.scan.items())?,
                SelectedTab::Timeouts(state) => {
                    state.handle_selected_item(|cfg| cfg.timeouts.items())?
                }
                SelectedTab::Ui(state) => state.handle_selected_item(|cfg| cfg.ui.items())?,
            },
            KeyCode::Char('k') | KeyCode::Up if !self.txt_edit_open() => {
                self.close_txt_edit();
                self.state().select_previous()
            }
            KeyCode::Char('j') | KeyCode::Down if !self.txt_edit_open() => {
                self.close_txt_edit();
                self.state().select_next()
            }
            KeyCode::Backspace
            | KeyCode::Left
            | KeyCode::Right
            | KeyCode::Home
            | KeyCode::End
            | KeyCode::BackTab
            | KeyCode::Delete
            | KeyCode::Insert
            | KeyCode::Char(_)
            | KeyCode::CapsLock
            | KeyCode::NumLock => match self {
                SelectedTab::Interfaces(picker)
                | SelectedTab::Scan(picker)
                | SelectedTab::Timeouts(picker)
                | SelectedTab::Ui(picker) => {
                    if let Some(txt_edit) = picker.txt_edit.as_mut() {
                        _ = txt_edit.input(key);
                    }
                }
            },
            _ => (),
        }
        Ok(())
    }

    pub(super) fn txt_edit_open(&self) -> bool {
        match self {
            SelectedTab::Interfaces(picker)
            | SelectedTab::Scan(picker)
            | SelectedTab::Timeouts(picker)
            | SelectedTab::Ui(picker) => picker.txt_edit.is_some(),
        }
    }

    pub(super) fn close_txt_edit(&mut self) {
        match self {
            SelectedTab::Interfaces(cfg_picker_state)
            | SelectedTab::Scan(cfg_picker_state)
            | SelectedTab::Timeouts(cfg_picker_state)
            | SelectedTab::Ui(cfg_picker_state) => cfg_picker_state.txt_edit = None,
        }
    }

    fn state(&mut self) -> &mut ListState {
        match self {
            SelectedTab::Interfaces(cfg)
            | SelectedTab::Scan(cfg)
            | SelectedTab::Timeouts(cfg)
            | SelectedTab::Ui(cfg) => &mut cfg.state,
        }
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

    fn from_repr(discriminant: usize, cfg: SharedConfig) -> Option<Self> {
        let cfg_picker_state = CfgPickerState::new(cfg);
        match discriminant {
            0 => Some(Self::Interfaces(cfg_picker_state)),
            1 => Some(Self::Scan(cfg_picker_state)),
            2 => Some(Self::Timeouts(cfg_picker_state)),
            3 => Some(Self::Ui(cfg_picker_state)),
            _ => None,
        }
    }

    pub(super) fn discriminant(&self) -> usize {
        match self {
            SelectedTab::Interfaces(_) => 0,
            SelectedTab::Scan(_) => 1,
            SelectedTab::Timeouts(_) => 2,
            SelectedTab::Ui(_) => 3,
        }
    }

    /// Get the previous tab, if there is no previous tab return the current tab.
    pub(super) fn previous(&self, cfg: SharedConfig) -> Self {
        let current_index: usize = self.discriminant();
        let previous_index = current_index.saturating_sub(1);
        Self::from_repr(previous_index, cfg).unwrap_or(self.clone())
    }

    /// Get the next tab, if there is no next tab return the current tab.
    pub(super) fn next(&self, cfg: SharedConfig) -> Self {
        let current_index = self.discriminant();
        let next_index = current_index.saturating_add(1);
        Self::from_repr(next_index, cfg).unwrap_or(self.clone())
    }

    fn render_txt_edit(cfg_picker: &CfgPickerState, area: &Rect, buf: &mut Buffer) {
        if let Some(txt_edit) = &cfg_picker.txt_edit {
            let selected = cfg_picker.state.selected().unwrap_or(0);
            let y_offset = (selected + 2) as u16;
            let x_offset = 28; // just ends up working out

            let content_width = text_edit_content_len(txt_edit) + 4;
            let available_width = area.width.saturating_sub(x_offset);
            const MIN_WIDTH: u16 = 10;
            if MIN_WIDTH > available_width {
                return;
            }
            let width = content_width.clamp(MIN_WIDTH, available_width);
            const HEIGHT: u16 = 2;
            let pos = area.as_position();
            let rect = Rect::new(pos.x + x_offset, pos.y + y_offset, width, HEIGHT);

            Clear.render(rect, buf);
            txt_edit.render(rect, buf);
        }
    }

    fn render_doc_paragraph(
        items: &[ConfigType<'_>],
        selected: usize,
        area: &Rect,
        buf: &mut Buffer,
    ) {
        if let Some(item) = items.get(selected) {
            match item {
                ConfigType::Toggle { description, .. }
                | ConfigType::NumberNonZeroU16 { description, .. }
                | ConfigType::Numberu32 { description, .. }
                | ConfigType::LogLevelString { description, .. }
                | ConfigType::ScanIoThreads { description, .. } => {
                    Self::render_doc_paragraph_inner(
                        description,
                        selected,
                        1,
                        area,
                        buf,
                        Borders::RIGHT,
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
                    );
                }
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
    ) {
        let mut max_line_width = 0;
        let num_lines = description.lines().count() as u16;
        let mut lines = vec![];
        for line in description.lines() {
            max_line_width = line.len().max(max_line_width);
            lines.push(Line::from(Span::styled(
                line,
                Style::new().fg(Color::LightGreen),
            )));
        }
        let doc_p = Paragraph::new(lines)
            .left_aligned()
            .wrap(Wrap { trim: true })
            .block(
                Block::default()
                    .borders(borders)
                    .border_style(Style::default().fg(Color::LightGreen)),
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
        // heuristic to make room for wrapping lines
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

    fn render_tab(
        block: Block<'_>,
        area: Rect,
        buf: &mut Buffer,
        picker: &mut CfgPickerState,
        get_items: impl Fn(&mut AppConfig) -> Vec<ConfigType<'_>>,
    ) {
        let list = picker.cfg.modify(|cfg| {
            let items = get_items(cfg);
            List::new(items)
                .block(block)
                .highlight_style(Style::default().bg(Color::DarkGray))
                .highlight_symbol("> ")
                .highlight_spacing(HighlightSpacing::Always)
        });

        StatefulWidget::render(list, area, buf, &mut picker.state);
        Self::render_txt_edit(picker, &area, buf);
        let Some(selected) = picker.state.selected() else {
            return;
        };

        picker.cfg.modify(|cfg| {
            let items = get_items(cfg);
            Self::render_doc_paragraph(&items, selected, &area, buf);
        });
    }

    fn render_interfaces_tab(
        block: Block<'_>,
        area: Rect,
        buf: &mut Buffer,
        picker: &mut CfgPickerState,
    ) {
        Self::render_tab(block, area, buf, picker, |cfg| cfg.interfaces.items())
    }

    fn render_scan_tab(
        block: Block<'_>,
        area: Rect,
        buf: &mut Buffer,
        picker: &mut CfgPickerState,
    ) {
        Self::render_tab(block, area, buf, picker, |cfg| cfg.scan.items())
    }

    fn render_timeouts_tab(
        block: Block<'_>,
        area: Rect,
        buf: &mut Buffer,
        picker: &mut CfgPickerState,
    ) {
        Self::render_tab(block, area, buf, picker, |cfg| cfg.timeouts.items())
    }

    fn render_ui_tab(block: Block<'_>, area: Rect, buf: &mut Buffer, picker: &mut CfgPickerState) {
        Self::render_tab(block, area, buf, picker, |cfg| cfg.ui.items())
    }

    /// A block surrounding the tab's content
    fn block(&self) -> Block<'static> {
        Block::bordered()
            .border_set(symbols::border::PROPORTIONAL_TALL)
            .padding(Padding::horizontal(1))
            .border_style(self.palette().c700)
    }

    pub(super) const fn palette(&self) -> tailwind::Palette {
        match self {
            Self::Interfaces(_) => tailwind::BLUE,
            Self::Scan(_) => tailwind::EMERALD,
            Self::Timeouts(_) => tailwind::CYAN,
            Self::Ui(_) => tailwind::YELLOW,
        }
    }
}
