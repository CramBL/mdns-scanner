use std::{num::NonZeroU16, sync::Arc, time::Instant};

use Constraint::{Length, Min};
use color_eyre::eyre::Context;
use mds_config::{AppConfig, ConfigType};
use parking_lot::RwLock;
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    layout::{Constraint, Layout, Rect},
    style::{Color, Style, Stylize, palette::tailwind},
    symbols,
    text::Line,
    widgets::{
        Block, Borders, Clear, HighlightSpacing, List, ListItem, ListState, Padding, Paragraph,
        StatefulWidget, StatefulWidgetRef, Tabs, Widget, WidgetRef,
    },
};
use tui_textarea::TextArea;

use strum::{Display, EnumIter, FromRepr, IntoEnumIterator};

use crate::{
    error_box::ErrorBox,
    util::{self, text_edit_content_len},
};

type ArcLockCfg = Arc<RwLock<AppConfig>>;

pub struct ConfigWindow<'t> {
    cfg: ArcLockCfg,
    is_open: bool,
    last_saved: Option<Instant>,
    awaiting_confirmation: bool,
    selected_tab: SelectedTab<'t>,
    state: ListState,
}

impl<'t> ConfigWindow<'t> {
    pub(crate) fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let vertical = Layout::vertical([Length(1), Min(0), Length(1)]);
        let [header_area, inner_area, footer_area] = vertical.areas(area);

        let horizontal = Layout::horizontal([Min(0), Length(20)]);
        let [tabs_area, title_area] = horizontal.areas(header_area);

        render_title(title_area, buf);
        self.render_tabs(tabs_area, buf);
        self.selected_tab.render_ref(inner_area, buf);
        render_footer(footer_area, buf);
    }

    pub(crate) fn new(cfg: ArcLockCfg) -> Self {
        let cfg_clone = Arc::clone(&cfg);
        let mut state = ListState::default();
        state.select(Some(0));
        Self {
            cfg,
            is_open: false,
            last_saved: None,
            awaiting_confirmation: false,
            selected_tab: SelectedTab::Interfaces(CfgPickerState::new(cfg_clone)),
            state,
        }
    }

    fn render_tabs(&self, area: Rect, buf: &mut Buffer) {
        let highlight_style = (Color::default(), self.selected_tab.palette().c700);
        let selected_tab_index = self.selected_tab.discriminant();
        let titles = SelectedTab::title_lines();
        Tabs::new(titles)
            .highlight_style(highlight_style)
            .select(selected_tab_index)
            .padding("", "")
            .divider(" ")
            .render_ref(area, buf);
    }

    pub(super) fn input(&mut self, key: KeyEvent) -> Result<(), ErrorBox> {
        match key.code {
            KeyCode::Left | KeyCode::Char('h') if !self.selected_tab.txt_edit_open() => {
                self.previous_tab()
            }
            KeyCode::Right | KeyCode::Char('l') if !self.selected_tab.txt_edit_open() => {
                self.next_tab()
            }
            // KeyCode::Home | KeyCode::Char('g') => self.state.select_first(),
            // KeyCode::End | KeyCode::Char('G') => self.state.select_last(),
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // self.save_config()?;
            }
            _ => self.selected_tab.input(key)?,
        };
        Ok(())
    }

    fn clone_cfg(&self) -> ArcLockCfg {
        Arc::clone(&self.cfg)
    }

    pub fn next_tab(&mut self) {
        self.selected_tab = self.selected_tab.next(self.clone_cfg());
    }

    pub fn previous_tab(&mut self) {
        self.selected_tab = self.selected_tab.previous(self.clone_cfg());
    }

    pub(crate) fn open(&mut self) {
        self.is_open = true;
    }

    pub(crate) fn is_open(&self) -> bool {
        self.is_open
    }

    pub(crate) fn close(&mut self) {
        self.is_open = false;
    }
}

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

#[derive(Clone)]
pub(crate) struct CfgPickerState<'t> {
    cfg: ArcLockCfg,
    txt_edit: Option<TextArea<'t>>,
    state: ListState,
}

impl<'t> CfgPickerState<'t> {
    pub fn new(cfg: ArcLockCfg) -> Self {
        let mut state = ListState::default();
        state.select(Some(0));
        Self {
            cfg,
            txt_edit: None,
            state,
        }
    }

    /// Enter/spacebar ...
    pub fn handle_confirm_action(
        txt_edit: &mut Option<TextArea<'_>>,
        item: &mut ConfigType<'_>,
    ) -> Result<(), ErrorBox> {
        match item {
            ConfigType::Toggle { key: _, val } => **val = !**val,
            ConfigType::NumberNonZeroU16 { key: _, val } => {
                if let Some(txt_edit) = txt_edit.as_mut() {
                    let txt = txt_edit
                        .lines()
                        .first()
                        .expect("unsound condition")
                        .trim_ascii();
                    let Ok(num) = txt.parse::<u16>() else {
                        return Err("Could not parse as u16".into());
                    };
                    let Some(new_val) = NonZeroU16::new(num) else {
                        return Err(format!("Expected Non-zero u16, got '{num}'").into());
                    };
                    **val = new_val;
                } else {
                    let mut text_area = build_text_edit_area();
                    text_area.insert_str(item.value_str());
                    *txt_edit = Some(text_area);
                }
            }
            ConfigType::Numberu32 { key: _, val } => {
                if let Some(txt_edit) = txt_edit.as_mut() {
                    let txt = txt_edit
                        .lines()
                        .first()
                        .expect("unsound condition")
                        .trim_ascii();
                    let Ok(new_val) = txt.parse::<u32>() else {
                        return Err(format!("Could not parse '{txt}' as u32").into());
                    };
                    **val = new_val;
                } else {
                    let mut text_area = build_text_edit_area();
                    text_area.insert_str(item.value_str());
                    *txt_edit = Some(text_area);
                }
            }
            ConfigType::NumberList { key: _, val } => {
                if let Some(txt_edit) = txt_edit.as_mut() {
                    let mut new_val = vec![];
                    for l in txt_edit.lines() {
                        for num in l.split_terminator(",") {
                            if let Ok(num) = num.trim_ascii().parse::<u16>() {
                                new_val.push(num);
                            }
                        }
                    }
                    new_val.dedup();
                    **val = Some(new_val);
                } else {
                    let mut text_area = build_text_edit_area();
                    text_area.insert_str(item.value_str());
                    *txt_edit = Some(text_area);
                }
            }
            ConfigType::StringList { key: _, val } => {
                if let Some(txt_edit) = txt_edit.as_mut() {
                    let mut new_val = vec![];
                    for l in txt_edit.lines() {
                        for pattern in l.split_terminator(",") {
                            new_val.push(pattern.trim_ascii().to_owned());
                        }
                    }
                    new_val.dedup();
                    **val = new_val;
                } else {
                    let mut text_area = build_text_edit_area();
                    text_area.insert_str(item.value_str());
                    *txt_edit = Some(text_area);
                }
            }
        }
        Ok(())
    }
}

fn build_text_edit_area<'a>() -> TextArea<'a> {
    let mut text_area = tui_textarea::TextArea::default();
    text_area.set_block(
        Block::default()
            .borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM)
            .border_style(Style::default().fg(Color::LightBlue)),
    );

    text_area.set_style(Style::default().fg(Color::Yellow));
    text_area.set_placeholder_style(Style::default());
    text_area
}

fn render_tab_content(
    block: Block<'_>,
    area: Rect,
    buf: &mut Buffer,
    picker: &mut CfgPickerState,
    items: Vec<ListItem<'static>>,
) {
    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().bg(Color::DarkGray))
        .highlight_symbol(">")
        .highlight_spacing(HighlightSpacing::Always);

    StatefulWidget::render(list, area, buf, &mut picker.state);
    SelectedTab::render_txt_edit(picker, &area, buf);
}

impl<'t> SelectedTab<'t> {
    fn render_ref(&mut self, area: Rect, buf: &mut Buffer) {
        let block = self.block();
        match self {
            Self::Interfaces(cfg) => Self::render_interfaces_tab(block, area, buf, cfg),
            Self::Scan(cfg) => Self::render_scan_tab(block, area, buf, cfg),
            Self::Timeouts(cfg) => Self::render_timeouts_tab(block, area, buf, cfg),
            Self::Ui(cfg) => Self::render_ui_tab(block, area, buf, cfg),
        }
    }

    fn input(&mut self, key: KeyEvent) -> Result<(), ErrorBox> {
        match key.code {
            KeyCode::Char(' ') | KeyCode::Enter => match self {
                SelectedTab::Interfaces(cfg_picker_state) => {
                    let Some(selected) = cfg_picker_state.state.selected() else {
                        return Ok(());
                    };
                    let mut cfg = cfg_picker_state.cfg.write();
                    let mut items = cfg.interfaces.items();
                    if let Some(item) = items.get_mut(selected) {
                        CfgPickerState::handle_confirm_action(
                            &mut cfg_picker_state.txt_edit,
                            item,
                        )?;
                    }
                }
                SelectedTab::Scan(cfg_picker_state) => {
                    let Some(selected) = cfg_picker_state.state.selected() else {
                        return Ok(());
                    };
                    let mut cfg = cfg_picker_state.cfg.write();
                    let mut items = cfg.scan.items();
                    if let Some(item) = items.get_mut(selected) {
                        CfgPickerState::handle_confirm_action(
                            &mut cfg_picker_state.txt_edit,
                            item,
                        )?;
                    }
                }
                SelectedTab::Timeouts(cfg_picker_state) => {
                    let Some(selected) = cfg_picker_state.state.selected() else {
                        return Ok(());
                    };
                    let mut cfg = cfg_picker_state.cfg.write();
                    let mut items = cfg.timeouts.items();
                    if let Some(item) = items.get_mut(selected) {
                        CfgPickerState::handle_confirm_action(
                            &mut cfg_picker_state.txt_edit,
                            item,
                        )?;
                    }
                }
                SelectedTab::Ui(cfg_picker_state) => {
                    let Some(selected) = cfg_picker_state.state.selected() else {
                        return Ok(());
                    };
                    let mut cfg = cfg_picker_state.cfg.write();
                    let mut items = cfg.ui.items();
                    if let Some(item) = items.get_mut(selected) {
                        CfgPickerState::handle_confirm_action(
                            &mut cfg_picker_state.txt_edit,
                            item,
                        )?;
                    }
                }
            },
            KeyCode::Up => {
                self.reset_txt_edit();
                self.state().select_previous()
            }
            KeyCode::Down => {
                self.reset_txt_edit();
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
                SelectedTab::Interfaces(picker) => {
                    if let Some(txt_edit) = picker.txt_edit.as_mut() {
                        _ = txt_edit.input(key);
                    }
                    let Some(selected) = picker.state.selected() else {
                        return Ok(());
                    };
                }
                SelectedTab::Scan(picker) => {
                    if let Some(txt_edit) = picker.txt_edit.as_mut() {
                        _ = txt_edit.input(key);
                    }
                }
                SelectedTab::Timeouts(picker) => {
                    if let Some(txt_edit) = picker.txt_edit.as_mut() {
                        _ = txt_edit.input(key);
                    }
                }
                SelectedTab::Ui(picker) => {
                    if let Some(txt_edit) = picker.txt_edit.as_mut() {
                        _ = txt_edit.input(key);
                    }
                }
            },
            _ => (),
        }
        Ok(())
    }

    fn txt_edit_open(&self) -> bool {
        match self {
            SelectedTab::Interfaces(picker) => picker.txt_edit.is_some(),
            SelectedTab::Scan(picker) => picker.txt_edit.is_some(),
            SelectedTab::Timeouts(picker) => picker.txt_edit.is_some(),
            SelectedTab::Ui(picker) => picker.txt_edit.is_some(),
        }
    }

    fn reset_txt_edit(&mut self) {
        match self {
            SelectedTab::Interfaces(cfg_picker_state) => cfg_picker_state.txt_edit = None,
            SelectedTab::Scan(cfg_picker_state) => cfg_picker_state.txt_edit = None,
            SelectedTab::Timeouts(cfg_picker_state) => cfg_picker_state.txt_edit = None,
            SelectedTab::Ui(cfg_picker_state) => cfg_picker_state.txt_edit = None,
        }
    }

    fn selected_state(&self) -> Option<usize> {
        match self {
            SelectedTab::Interfaces(cfg) => cfg.state.selected(),
            SelectedTab::Scan(cfg) => cfg.state.selected(),
            SelectedTab::Timeouts(cfg) => cfg.state.selected(),
            SelectedTab::Ui(cfg) => cfg.state.selected(),
        }
    }

    fn state(&mut self) -> &mut ListState {
        match self {
            SelectedTab::Interfaces(cfg) => &mut cfg.state,
            SelectedTab::Scan(cfg) => &mut cfg.state,
            SelectedTab::Timeouts(cfg) => &mut cfg.state,
            SelectedTab::Ui(cfg) => &mut cfg.state,
        }
    }

    fn title_lines() -> [Line<'static>; 4] {
        [
            format!("  Interfaces  ")
                .fg(tailwind::SLATE.c200)
                .bg(tailwind::BLUE.c900)
                .into(),
            format!("  Scan  ")
                .fg(tailwind::SLATE.c200)
                .bg(tailwind::EMERALD.c900)
                .into(),
            format!("  Timeouts  ")
                .fg(tailwind::SLATE.c200)
                .bg(tailwind::INDIGO.c900)
                .into(),
            format!("  UI  ")
                .fg(tailwind::SLATE.c200)
                .bg(tailwind::RED.c900)
                .into(),
        ]
    }

    fn from_repr(discriminant: usize, cfg: ArcLockCfg) -> Option<Self> {
        let cfg_picker_state = CfgPickerState::new(cfg);
        match discriminant {
            0 => Some(Self::Interfaces(cfg_picker_state)),
            1 => Some(Self::Scan(cfg_picker_state)),
            2 => Some(Self::Timeouts(cfg_picker_state)),
            3 => Some(Self::Ui(cfg_picker_state)),
            _ => None,
        }
    }

    fn discriminant(&self) -> usize {
        match self {
            SelectedTab::Interfaces(_) => 0,
            SelectedTab::Scan(_) => 1,
            SelectedTab::Timeouts(_) => 2,
            SelectedTab::Ui(_) => 3,
        }
    }

    /// Get the previous tab, if there is no previous tab return the current tab.
    fn previous(&self, cfg: ArcLockCfg) -> Self {
        let current_index: usize = self.discriminant();
        let previous_index = current_index.saturating_sub(1);
        Self::from_repr(previous_index, cfg).unwrap_or(self.clone())
    }

    /// Get the next tab, if there is no next tab return the current tab.
    fn next(&self, cfg: ArcLockCfg) -> Self {
        let current_index = self.discriminant();
        let next_index = current_index.saturating_add(1);
        Self::from_repr(next_index, cfg).unwrap_or(self.clone())
    }

    /// Return tab's name as a styled `Line`
    fn title(self) -> Line<'static> {
        format!("  {self}  ")
            .fg(tailwind::SLATE.c200)
            .bg(self.palette().c900)
            .into()
    }

    fn render_txt_edit(cfg_picker: &CfgPickerState, area: &Rect, buf: &mut Buffer) {
        if let Some(txt_edit) = &cfg_picker.txt_edit {
            let selected = cfg_picker.state.selected().unwrap_or(0);
            let y_offset = (selected + 2) as u16;
            let x_offset = 27; // just ends up working out

            let content_width = text_edit_content_len(txt_edit) + 4;
            let available_width = area.width.saturating_sub(x_offset);
            let width = content_width.clamp(10, available_width);

            let pos = area.as_position();
            let rect = Rect::new(pos.x + x_offset, pos.y + y_offset, width, 2);

            Clear.render(rect, buf);
            txt_edit.render(rect, buf);
        }
    }

    fn render_interfaces_tab(
        block: Block<'_>,
        area: Rect,
        buf: &mut Buffer,
        picker: &mut CfgPickerState,
    ) {
        let mut cfg = picker.cfg.write();
        let items: Vec<_> = cfg.interfaces.items();

        let list = List::new(items)
            .block(block)
            .highlight_style(Style::default().bg(Color::DarkGray))
            .highlight_symbol(">")
            .highlight_spacing(HighlightSpacing::Always);

        StatefulWidget::render(list, area, buf, &mut picker.state);
        Self::render_txt_edit(picker, &area, buf);
    }

    fn render_scan_tab(
        block: Block<'_>,
        area: Rect,
        buf: &mut Buffer,
        picker: &mut CfgPickerState,
    ) {
        let mut cfg = picker.cfg.write();
        let items: Vec<_> = cfg.scan.items();

        let list = List::new(items)
            .block(block)
            .highlight_style(Style::default().bg(Color::DarkGray))
            .highlight_symbol(">")
            .highlight_spacing(HighlightSpacing::Always);

        StatefulWidget::render(list, area, buf, &mut picker.state);
        Self::render_txt_edit(picker, &area, buf);
    }

    fn render_timeouts_tab(
        block: Block<'_>,
        area: Rect,
        buf: &mut Buffer,
        picker: &mut CfgPickerState,
    ) {
        let mut cfg = picker.cfg.write();
        let items: Vec<_> = cfg.timeouts.items();

        let list = List::new(items)
            .block(block)
            .highlight_style(Style::default().bg(Color::DarkGray))
            .highlight_symbol(">")
            .highlight_spacing(HighlightSpacing::Always);

        StatefulWidget::render(list, area, buf, &mut picker.state);
        Self::render_txt_edit(picker, &area, buf);
    }

    fn render_ui_tab(block: Block<'_>, area: Rect, buf: &mut Buffer, picker: &mut CfgPickerState) {
        let mut cfg = picker.cfg.write();
        let items: Vec<_> = cfg.ui.items();

        let list = List::new(items)
            .block(block)
            .highlight_style(Style::default().bg(Color::DarkGray))
            .highlight_symbol(">")
            .highlight_spacing(HighlightSpacing::Always);
        StatefulWidget::render(list, area, buf, &mut picker.state);
        Self::render_txt_edit(picker, &area, buf);
    }

    /// A block surrounding the tab's content
    fn block(&self) -> Block<'static> {
        Block::bordered()
            .border_set(symbols::border::PROPORTIONAL_TALL)
            .padding(Padding::horizontal(1))
            .border_style(self.palette().c700)
    }

    const fn palette(&self) -> tailwind::Palette {
        match self {
            Self::Interfaces(_) => tailwind::BLUE,
            Self::Scan(_) => tailwind::EMERALD,
            Self::Timeouts(_) => tailwind::INDIGO,
            Self::Ui(_) => tailwind::RED,
        }
    }
}

fn render_title(area: Rect, buf: &mut Buffer) {
    "Config".bold().render(area, buf);
}

fn render_footer(area: Rect, buf: &mut Buffer) {
    Line::raw("◄ ► to change tab | Press q to quit")
        .centered()
        .render(area, buf);
}
