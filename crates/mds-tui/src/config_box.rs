use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use mds_config::{
    AppConfig,
    toggle::{ConfigFieldId, ConfigToggle},
};
use parking_lot::RwLock;
use ratatui::{
    Frame,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Style, Stylize as _},
    text::{Line, Span, Text},
    widgets::{
        Block, BorderType, Borders, Clear, HighlightSpacing, List, ListItem, ListState, Paragraph,
    },
};

use crate::{error_box::ErrorBox, util};

pub struct AppConfigToggle<'a>(pub (&'a ConfigToggle, &'a AppConfig));

impl From<AppConfigToggle<'_>> for ListItem<'_> {
    fn from(app_config_toggle: AppConfigToggle) -> Self {
        let (item, cfg) = app_config_toggle.0;
        let checkbox = if item.enabled(cfg) { "[*]" } else { "[ ]" };
        let line = Line::styled(
            format!(" {} {}", checkbox, item.name()),
            if item.enabled(cfg) {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::White)
            },
        );
        ListItem::new(line)
    }
}

pub(super) struct ConfigBox {
    cfg: Arc<RwLock<AppConfig>>,
    items: Vec<ConfigToggle>,
    state: ListState,
    is_open: bool,
    last_saved: Option<Instant>,
    awaiting_confirmation: bool,
}

impl ConfigBox {
    pub(crate) fn new(cfg: Arc<RwLock<AppConfig>>) -> Self {
        let items = vec![
            ConfigToggle::ConfigField {
                label: "Hide IPs with no association (no resolved hostname/service information)"
                    .into(),
                field_id: ConfigFieldId::HideBareIps,
            },
            ConfigToggle::ConfigField {
                label: "Enable Service Discovery".into(),
                field_id: ConfigFieldId::ServiceDiscovery,
            },
            ConfigToggle::ConfigField {
                label: "Include Docker Interfaces".into(),
                field_id: ConfigFieldId::IncludeDocker,
            },
            ConfigToggle::ConfigField {
                label: "Compact Output".into(),
                field_id: ConfigFieldId::Compact,
            },
        ];

        let mut state = ListState::default();
        state.select(Some(0));

        Self {
            cfg,
            items,
            state,
            is_open: false,
            last_saved: None,
            awaiting_confirmation: false,
        }
    }

    pub(super) fn render(&mut self, frame: &mut Frame) {
        if !self.is_open {
            return;
        }
        let config_box_area = self.centered(frame);
        let vertical = &Layout::vertical([Constraint::Min(5), Constraint::Length(5)]);
        let rects = vertical.split(config_box_area);
        let main_area = rects[0];
        let footer_area = rects[1];

        frame.render_widget(Clear, config_box_area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::LightBlue))
            .title("Configuration")
            .title_alignment(Alignment::Center);

        let cfg = self.cfg.read();
        let items: Vec<ListItem> = self
            .items
            .iter()
            .map(|i| ListItem::from(AppConfigToggle((i, &*cfg))))
            .collect();

        let list = List::new(items)
            .block(block)
            .highlight_style(Style::default().bg(Color::DarkGray))
            .highlight_symbol(">> ")
            .highlight_spacing(HighlightSpacing::Always);

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
                Span::styled("Spacebar", Style::new().fg(Color::Green)),
                Span::raw(">: toggle"),
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
        frame.render_widget(footer, footer_area);
        frame.render_stateful_widget(list, main_area, &mut self.state);
    }

    pub(super) fn input(&mut self, key: KeyEvent) -> Result<(), ErrorBox> {
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => self.state.select_next(),
            KeyCode::Up | KeyCode::Char('k') => self.state.select_previous(),
            KeyCode::Home | KeyCode::Char('g') => self.state.select_first(),
            KeyCode::End | KeyCode::Char('G') => self.state.select_last(),
            KeyCode::Char(' ') | KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
                self.toggle_enabled();
            }
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.save_config()?;
            }
            _ => (),
        };
        Ok(())
    }

    fn toggle_enabled(&mut self) {
        if let Some(selected) = self.state.selected() {
            if let Some(item) = self.items.get_mut(selected) {
                let mut cfg = self.cfg.write();
                item.toggle(&mut cfg);
            }
        }
    }

    fn centered(&self, frame: &Frame) -> Rect {
        let horizontal = Constraint::Percentage(80);
        let vertical = Constraint::Percentage(80);
        util::center(frame.area(), horizontal, vertical)
    }

    pub(crate) fn open(&mut self) {
        self.is_open = true;
    }

    pub(crate) fn close(&mut self) {
        self.is_open = false;
    }
    pub(crate) fn is_open(&self) -> bool {
        self.is_open
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
