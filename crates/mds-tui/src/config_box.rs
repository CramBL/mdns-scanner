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

        let min_width = 40u16;
        let min_height = 8u16;
        let max_width_ratio = 0.8; // Maximum 80% of screen width
        let max_height_ratio = 0.8; // Maximum 80% of screen height

        let frame_width = frame_area.width as f32;
        let frame_height = frame_area.height as f32;

        enum Frame {
            Small,
            Medium,
            Large,
        }

        let width_size: Frame = if frame_area.width <= 80 {
            Frame::Small
        } else if frame_area.width <= 120 {
            Frame::Medium
        } else {
            Frame::Large
        };

        let scaled_width = match width_size {
            Frame::Small => (frame_width * 0.9).min(frame_width),
            Frame::Medium => frame_width * 0.7,
            Frame::Large => frame_width * 0.5,
        } as u16;
        let width = scaled_width
            .max(min_width)
            .min((frame_width * max_width_ratio) as u16);

        let height_size = if frame_area.height <= 20 {
            Frame::Small
        } else if frame_area.height <= 40 {
            Frame::Medium
        } else {
            Frame::Large
        };
        let scaled_height = match height_size {
            Frame::Small => frame_height * 0.9,
            Frame::Medium => frame_height * 0.8,
            Frame::Large => frame_height * 0.6,
        } as u16;

        let height = scaled_height
            .max(min_height)
            .min((frame_height * max_height_ratio) as u16);

        // Center the box
        let x = (frame_area.width.saturating_sub(width)) / 2;
        let y = (frame_area.height.saturating_sub(height)) / 2;

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
