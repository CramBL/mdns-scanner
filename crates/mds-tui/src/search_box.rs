use ratatui::{
    Frame,
    crossterm::event::{KeyCode, KeyEvent},
    layout::{Alignment, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear},
};
use tui_textarea::TextArea;

pub(super) struct SearchBox<'a> {
    text_area: TextArea<'a>,
}

impl Default for SearchBox<'_> {
    fn default() -> Self {
        let mut text_area = tui_textarea::TextArea::default();
        text_area.set_block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::LightBlue))
                .title("Search")
                .title_alignment(Alignment::Center),
        );

        text_area.set_style(Style::default().fg(Color::Yellow));
        text_area.set_placeholder_style(Style::default());
        Self { text_area }
    }
}

impl SearchBox<'_> {
    const DEFAULT_WIDTH: u16 = 15;
    const HEIGHT: u16 = 3;

    pub(super) fn render(&self, frame: &mut Frame, table_area: Rect) {
        let Some(search_box_area) = self.area(table_area) else {
            return;
        };

        frame.render_widget(Clear, search_box_area);
        frame.render_widget(&self.text_area, search_box_area);
    }

    pub(super) fn contents(&self) -> &str {
        self.text_area.lines().first().expect("Unsound condition")
    }

    fn content_width(&self) -> usize {
        self.text_area
            .lines()
            .first()
            .map(|l| l.len())
            .unwrap_or_default()
    }

    pub(super) fn input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Backspace
            | KeyCode::Left
            | KeyCode::Right
            | KeyCode::Home
            | KeyCode::End
            | KeyCode::Tab
            | KeyCode::BackTab
            | KeyCode::Delete
            | KeyCode::Insert
            | KeyCode::Char(_)
            | KeyCode::CapsLock
            | KeyCode::NumLock => _ = self.text_area.input(key),
            _ => (),
        };
    }

    fn area(&self, table_area: Rect) -> Option<Rect> {
        let width = Self::DEFAULT_WIDTH.max(self.content_width() as u16 + 3); // +3 otherwise it'll start eating the text from the left
        let width = width.min(table_area.width - 1);
        let height = Self::HEIGHT.min(table_area.height - 1);
        let x: u16 = (table_area.width.saturating_sub(width)) / 2;

        Some(Rect {
            width,
            height,
            x,
            y: 0,
        })
    }
}
