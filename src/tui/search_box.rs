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
    const WIDTH: u16 = 40;
    const HEIGHT: u16 = 3;

    pub(super) fn render(&self, frame: &mut Frame, table_area: Rect) {
        let Some(search_box_area) = self.area(frame, table_area) else {
            return;
        };

        frame.render_widget(Clear, search_box_area);
        frame.render_widget(&self.text_area, search_box_area);
    }

    pub(super) fn contents(&self) -> &str {
        self.text_area.lines().first().expect("Unsound condition")
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

    fn area(&self, frame: &Frame, table_area: Rect) -> Option<Rect> {
        let x_center = table_area.width / 2;
        let available_height_above = frame.area().height - table_area.height;
        let y = if available_height_above >= Self::HEIGHT {
            available_height_above - Self::HEIGHT
        } else {
            available_height_above
        };

        Some(Rect {
            width: Self::WIDTH,
            height: Self::HEIGHT,
            x: x_center - Self::WIDTH / 2,
            y,
        })
    }
}
