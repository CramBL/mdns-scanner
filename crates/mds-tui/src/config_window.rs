use std::{
    num::NonZeroU16,
    sync::Arc,
    time::{Duration, Instant},
};

use mds_config::{AppConfig, ConfigType};
use parking_lot::RwLock;
use ratatui::layout::Constraint::{Length, Min};
use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    layout::{Layout, Rect},
    style::{Color, Style, Stylize, palette::tailwind},
    symbols,
    text::{Line, Span, Text},
    widgets::{
        Block, BorderType, Borders, Clear, HighlightSpacing, List, ListState, Padding, Paragraph,
        StatefulWidget, Tabs, Widget, WidgetRef, Wrap,
    },
};
use tui_textarea::TextArea;

use strum::Display;

use crate::{error_box::ErrorBox, util::text_edit_content_len};

type ArcLockCfg = Arc<RwLock<AppConfig>>;

pub struct ConfigWindow<'t> {
    cfg: ArcLockCfg,
    is_open: bool,
    last_saved: Option<Instant>,
    awaiting_confirmation: bool,
    selected_tab: SelectedTab<'t>,
}

impl<'t> ConfigWindow<'t> {
    pub(crate) fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let vertical = Layout::vertical([Length(1), Min(0), Length(3)]);
        let [header_area, inner_area, footer_area] = vertical.areas(area);

        let horizontal = Layout::horizontal([Min(0), Length(20)]);
        let [tabs_area, title_area] = horizontal.areas(header_area);

        render_title(title_area, buf);
        self.render_tabs(tabs_area, buf);
        self.selected_tab.render_ref(inner_area, buf);
        let footer_lines: Vec<Span<'_>> = if self
            .last_saved
            .is_some_and(|s| s.elapsed() < Duration::from_secs(2))
        {
            vec![Span::from("Config saved!").green()]
        } else {
            vec![
                Span::raw("<"),
                Span::styled("Ctrl+S", Style::new().fg(Color::Green)),
                Span::raw(">: save config"),
                Span::raw(" | <"),
                Span::styled("Spacebar, Enter", Style::new().fg(Color::Green)),
                Span::raw(">: modify"),
            ]
        };
        let footer = Paragraph::new(Text::from_iter(vec![footer_lines]))
            .style(Style::new())
            .centered()
            .block(
                Block::bordered()
                    .border_type(BorderType::Plain)
                    .border_style(Style::new())
                    .title(Line::from("").centered()),
            );
        footer.render(footer_area, buf);
    }

    pub(crate) fn new(cfg: ArcLockCfg) -> Self {
        let cfg_clone = Arc::clone(&cfg);
        Self {
            cfg,
            is_open: false,
            last_saved: None,
            awaiting_confirmation: false,
            selected_tab: SelectedTab::Interfaces(CfgPickerState::new(cfg_clone)),
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
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.save_config()?;
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

    pub(crate) fn close_action(&mut self) {
        if self.selected_tab.txt_edit_open() {
            self.selected_tab.close_txt_edit();
        } else {
            self.is_open = false;
        }
    }

    fn save_config(&mut self) -> Result<(), ErrorBox> {
        let Some(user_config) = AppConfig::user_config_path() else {
            return Err("Could not determine user config path".into());
        };

        match AppConfig::load_with_comments(&user_config) {
            Ok((_cfg, doc)) => {
                let current_cfg = self.cfg.read().clone();
                if let Err(e) = AppConfig::save_with_comments(user_config, &current_cfg, Some(doc))
                {
                    return Err(e.to_string().into());
                }
                self.last_saved = Some(Instant::now());
            }
            Err(e) => {
                let p = user_config.display();
                let prompt = vec![
                    (
                        "Failed retrieving the config from:".to_owned(),
                        Style::new().white().bold(),
                    ),
                    (format!("{p}"), Style::new().white().underlined()),
                    (String::new(), Style::new()),
                    (
                        "Would you like to create it?".to_owned(),
                        Style::new().yellow(),
                    ),
                ];
                self.awaiting_confirmation = true;
                return Err(ErrorBox::new(e.to_string()).with_prompt(prompt));
            }
        }
        Ok(())
    }

    fn write_new_current_config(&mut self) -> Result<(), ErrorBox> {
        if let Err(e) = AppConfig::write_default_config() {
            return Err(e.to_string().into());
        }
        self.save_config()?;
        Ok(())
    }

    pub(crate) fn confirm_action(&mut self) -> Result<(), ErrorBox> {
        if self.awaiting_confirmation {
            self.awaiting_confirmation = false;
            self.write_new_current_config()?;
        }
        Ok(())
    }

    pub(crate) fn cancel_action(&mut self) {
        self.awaiting_confirmation = false;
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
            ConfigType::Toggle { val, .. } => **val = !**val,
            ConfigType::NumberNonZeroU16 { val, .. } => {
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
            ConfigType::Numberu32 { val, .. } => {
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
            ConfigType::NumberList { val, .. } => {
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
            ConfigType::StringList { val, .. } => {
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

    fn txt_edit_open(&self) -> bool {
        match self {
            SelectedTab::Interfaces(picker)
            | SelectedTab::Scan(picker)
            | SelectedTab::Timeouts(picker)
            | SelectedTab::Ui(picker) => picker.txt_edit.is_some(),
        }
    }

    fn close_txt_edit(&mut self) {
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

    fn title_lines() -> [Line<'static>; 4] {
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
                .bg(tailwind::INDIGO.c900)
                .into(),
            "  UI  "
                .to_string()
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

    fn render_txt_edit(cfg_picker: &CfgPickerState, area: &Rect, buf: &mut Buffer) {
        if let Some(txt_edit) = &cfg_picker.txt_edit {
            let selected = cfg_picker.state.selected().unwrap_or(0);
            let y_offset = (selected + 2) as u16;
            let x_offset = 27; // just ends up working out

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
                | ConfigType::Numberu32 { description, .. } => {
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
                | ConfigType::StringList { description, .. } => {
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
        let height = num_lines.clamp(MIN_HEIGHT, available_height);
        let pos = area.as_position();
        let rect = Rect::new(pos.x + x_offset, pos.y + y_offset, width, height);
        Clear.render(rect, buf);
        doc_p.render(rect, buf);
    }

    fn render_interfaces_tab(
        block: Block<'_>,
        area: Rect,
        buf: &mut Buffer,
        picker: &mut CfgPickerState,
    ) {
        let list = {
            let mut cfg = picker.cfg.write();
            let items: Vec<_> = cfg.interfaces.items();

            List::new(items)
                .block(block)
                .highlight_style(Style::default().bg(Color::DarkGray))
                .highlight_symbol(">")
                .highlight_spacing(HighlightSpacing::Always)
        };

        StatefulWidget::render(list, area, buf, &mut picker.state);
        Self::render_txt_edit(picker, &area, buf);
        let Some(selected) = picker.state.selected() else {
            return;
        };
        let mut cfg = picker.cfg.write();
        let items = cfg.interfaces.items();

        Self::render_doc_paragraph(&items, selected, &area, buf);
    }

    fn render_scan_tab(
        block: Block<'_>,
        area: Rect,
        buf: &mut Buffer,
        picker: &mut CfgPickerState,
    ) {
        let list = {
            let mut cfg = picker.cfg.write();
            let items: Vec<_> = cfg.scan.items();

            List::new(items)
                .block(block)
                .highlight_style(Style::default().bg(Color::DarkGray))
                .highlight_symbol(">")
                .highlight_spacing(HighlightSpacing::Always)
        };

        StatefulWidget::render(list, area, buf, &mut picker.state);
        Self::render_txt_edit(picker, &area, buf);
        let Some(selected) = picker.state.selected() else {
            return;
        };
        let mut cfg = picker.cfg.write();
        let items = cfg.scan.items();

        Self::render_doc_paragraph(&items, selected, &area, buf);
    }

    fn render_timeouts_tab(
        block: Block<'_>,
        area: Rect,
        buf: &mut Buffer,
        picker: &mut CfgPickerState,
    ) {
        let list = {
            let mut cfg = picker.cfg.write();
            let items: Vec<_> = cfg.timeouts.items();

            List::new(items)
                .block(block)
                .highlight_style(Style::default().bg(Color::DarkGray))
                .highlight_symbol(">")
                .highlight_spacing(HighlightSpacing::Always)
        };

        StatefulWidget::render(list, area, buf, &mut picker.state);
        Self::render_txt_edit(picker, &area, buf);
        let Some(selected) = picker.state.selected() else {
            return;
        };
        let mut cfg = picker.cfg.write();
        let items = cfg.timeouts.items();

        Self::render_doc_paragraph(&items, selected, &area, buf);
    }

    fn render_ui_tab(block: Block<'_>, area: Rect, buf: &mut Buffer, picker: &mut CfgPickerState) {
        let list = {
            let mut cfg = picker.cfg.write();
            let items: Vec<_> = cfg.ui.items();

            List::new(items)
                .block(block)
                .highlight_style(Style::default().bg(Color::DarkGray))
                .highlight_symbol(">")
                .highlight_spacing(HighlightSpacing::Always)
        };
        StatefulWidget::render(list, area, buf, &mut picker.state);
        Self::render_txt_edit(picker, &area, buf);
        let Some(selected) = picker.state.selected() else {
            return;
        };
        let mut cfg = picker.cfg.write();
        let items = cfg.ui.items();
        Self::render_doc_paragraph(&items, selected, &area, buf);
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
