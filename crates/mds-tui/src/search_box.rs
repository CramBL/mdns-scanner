use std::{cell::RefCell, rc::Rc};

use ratatui::{
    Frame,
    crossterm::event::{KeyCode, KeyEvent},
    layout::{Alignment, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear},
};
use tui_textarea::TextArea;

use crate::{
    components::MdsKeyHandler,
    error_box::ErrorBox,
    message::{Message, Navigate},
};

pub(super) struct SearchBox<'a> {
    text_area: TextArea<'a>,
    pattern_ref: Rc<RefCell<String>>,
}

impl<'a> SearchBox<'a> {
    const DEFAULT_WIDTH: u16 = 15;
    const HEIGHT: u16 = 3;

    pub(crate) fn new() -> Self {
        log::error!("NEW SEARCH BOX");
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
        Self {
            text_area,
            pattern_ref: Rc::new(RefCell::new(String::new())),
        }
    }

    pub fn contents(&self) -> &str {
        self.text_area.lines().first().expect("Unsound condition")
    }

    pub(super) fn pattern(&self) -> Rc<RefCell<String>> {
        Rc::clone(&self.pattern_ref)
    }

    fn content_width(&self) -> usize {
        self.text_area
            .lines()
            .first()
            .map(|l| l.len())
            .unwrap_or_default()
    }

    fn area(&self, table_area: Rect) -> Rect {
        let width = Self::DEFAULT_WIDTH.max(self.content_width() as u16 + 3); // +3 otherwise it'll start eating the text from the left
        let width = width.min(table_area.width - 1);
        let height = Self::HEIGHT.min(table_area.height - 1);
        let x: u16 = (table_area.width.saturating_sub(width)) / 2;

        Rect {
            width,
            height,
            x,
            y: 0,
        }
    }
}

impl<'a> MdsKeyHandler for SearchBox<'a> {
    fn render(&mut self, frame: &mut Frame<'_>) {
        let search_box_area = self.area(frame.area());
        frame.render_widget(Clear, search_box_area);
        frame.render_widget(&self.text_area, search_box_area);
    }

    fn handle_local_key_event(&mut self, key: KeyEvent) -> Result<Option<Message>, ErrorBox> {
        let msg = match key.code {
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
            | KeyCode::NumLock => Some(Message::BoxInput(key)),
            KeyCode::Down => Some(Message::Navigate(Navigate::Down)),
            KeyCode::Up => Some(Message::Navigate(Navigate::Up)),
            KeyCode::Enter => Some(Message::Navigate(Navigate::Select)),
            // TODO: Catch event to open search box to prevent opening multiple search boxes
            _ => None,
        };

        Ok(msg)
    }

    // Currently the search box is kept in an optional field so we can only call this
    // if it is `Some` and so it must be focused
    fn is_focused(&self) -> bool {
        true
    }

    fn update(&mut self, msg: Message) -> Result<Option<Message>, ErrorBox> {
        match msg {
            Message::BoxInput(key_event) => {
                _ = self.text_area.input(key_event);
                let contents = self.contents();
                if *self.pattern_ref.borrow() != contents {
                    *self.pattern_ref.borrow_mut() = contents.to_owned();
                }
                return Ok(None);
            }
            _ => (),
        };
        Ok(Some(msg))
    }
}
