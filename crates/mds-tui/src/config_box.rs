use ratatui::{
    Frame,
    crossterm::event::{KeyCode, KeyEvent},
    layout::{Alignment, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Clear, HighlightSpacing, List, ListItem, ListState, StatefulWidget},
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConfigToggle {
    HideIpsWithNoAssociation(bool),
    NotImplementedYetFeature(bool),
}

impl ConfigToggle {
    pub fn name(&self) -> &str {
        match self {
            ConfigToggle::HideIpsWithNoAssociation(_) => {
                "Hide IPs with no association (no resolved hostname/service information)"
            }
            ConfigToggle::NotImplementedYetFeature(_) => "Not implemented yet feature",
        }
    }

    pub fn enabled(&self) -> bool {
        match self {
            ConfigToggle::HideIpsWithNoAssociation(val)
            | ConfigToggle::NotImplementedYetFeature(val) => *val,
        }
    }

    pub fn toggle(&mut self) {
        *self = match self {
            ConfigToggle::HideIpsWithNoAssociation(val) => {
                ConfigToggle::HideIpsWithNoAssociation(!*val)
            }
            ConfigToggle::NotImplementedYetFeature(val) => {
                ConfigToggle::NotImplementedYetFeature(!*val)
            }
        };
    }
}

pub(super) struct ConfigBox {
    items: Vec<ConfigToggle>,
    state: ListState,
    is_open: bool,
}

impl Default for ConfigBox {
    fn default() -> Self {
        let items = vec![
            ConfigToggle::HideIpsWithNoAssociation(false),
            ConfigToggle::NotImplementedYetFeature(false),
        ];

        let mut state = ListState::default();
        state.select(Some(0));

        Self {
            items,
            state,
            is_open: false,
        }
    }
}

impl ConfigBox {
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

        let items: Vec<ListItem> = self.items.iter().map(ListItem::from).collect();

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
                item.toggle();
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

impl From<&ConfigToggle> for ListItem<'_> {
    fn from(item: &ConfigToggle) -> Self {
        let checkbox = if item.enabled() { "☑" } else { "☐" };
        let line = Line::styled(
            format!(" {} {}", checkbox, item.name()),
            if item.enabled() {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::White)
            },
        );
        ListItem::new(line)
    }
}
