use std::time::{Duration, Instant};

use mds_config::{AppConfig, shared_config::SharedConfig};
use ratatui::layout::Constraint::{Length, Min};
use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    layout::{Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Paragraph, Tabs, Widget, WidgetRef},
};

use crate::components;
use crate::error_box::ErrorBox;
use crate::message::Message;

mod selected_tab;
use selected_tab::SelectedTab;

pub(super) mod cfg_picker_state;
use cfg_picker_state::CfgPickerState;

pub struct ConfigWindow<'t> {
    cfg: SharedConfig,
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
                Span::styled("Spacebar/Enter", Style::new().fg(Color::Green)),
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

    pub(crate) fn new(cfg: SharedConfig) -> Self {
        Self {
            cfg: cfg.clone(),
            is_open: false,
            last_saved: None,
            awaiting_confirmation: false,
            selected_tab: SelectedTab::Interfaces(CfgPickerState::new(cfg)),
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

    pub fn next_tab(&mut self) {
        self.selected_tab = self.selected_tab.next(self.cfg.clone());
    }

    pub fn previous_tab(&mut self) {
        self.selected_tab = self.selected_tab.previous(self.cfg.clone());
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

fn render_title(area: Rect, buf: &mut Buffer) {
    "Config".bold().render(area, buf);
}

impl components::MdsKeyHandler for ConfigWindow<'_> {
    fn handle_key_event(&mut self, key: KeyEvent) -> color_eyre::Result<Option<Message>> {
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

    fn is_focused(&self) -> bool {
        self.is_open
    }
}
