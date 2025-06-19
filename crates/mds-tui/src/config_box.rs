use std::sync::Arc;

use mds_config::{
    AppConfig,
    toggle::{ConfigFieldId, ConfigToggle},
};
use parking_lot::RwLock;
use ratatui::{
    Frame,
    crossterm::event::{KeyCode, KeyEvent},
    layout::{Alignment, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Clear, HighlightSpacing, List, ListItem, ListState, StatefulWidget},
};

pub struct AppConfigToggle<'a>(pub (&'a ConfigToggle, &'a AppConfig));

impl From<AppConfigToggle<'_>> for ListItem<'_> {
    fn from(app_config_toggle: AppConfigToggle) -> Self {
        let (item, cfg) = app_config_toggle.0;
        let checkbox = if item.enabled(cfg) { "☑" } else { "☐" };
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
}

impl ConfigBox {
    pub(crate) fn new(cfg: Arc<RwLock<AppConfig>>) -> Self {
        let items = vec![
            ConfigToggle::HideIpsWithNoAssociation(false),
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
        }
    }

    pub(super) fn render(&mut self, frame: &mut Frame, _area: Rect) {
        if !self.is_open {
            return;
        }
        let Some(config_box_area) = self.area(frame) else {
            return;
        };

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

        StatefulWidget::render(list, config_box_area, frame.buffer_mut(), &mut self.state);
    }

    pub(super) fn get_enabled_items(&self) -> Vec<ConfigToggle> {
        self.items.clone()
    }

    pub(super) fn input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => self.state.select_next(),
            KeyCode::Up | KeyCode::Char('k') => self.state.select_previous(),
            KeyCode::Home | KeyCode::Char('g') => self.state.select_first(),
            KeyCode::End | KeyCode::Char('G') => self.state.select_last(),
            KeyCode::Char(' ') | KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
                self.toggle_enabled();
            }
            KeyCode::Left | KeyCode::Char('h') => self.state.select(None),
            _ => (),
        };
    }

    fn toggle_enabled(&mut self) {
        if let Some(selected) = self.state.selected() {
            if let Some(item) = self.items.get_mut(selected) {
                let mut cfg = self.cfg.write();
                item.toggle(&mut cfg);
            }
        }
    }

    fn area(&self, frame: &Frame) -> Option<Rect> {
        let frame_area = frame.area();
        let width = (frame_area.width as f32 * 0.5) as u16;
        let height = (frame_area.height as f32 * 0.8) as u16;
        let x = (frame_area.width - width) / 2;
        let y = (frame_area.height - height) / 2;

        Some(Rect {
            width,
            height,
            x,
            y,
        })
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
}
