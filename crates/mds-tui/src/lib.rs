use ratatui::crossterm::event::{self, KeyCode, KeyModifiers};

pub(crate) mod components;
pub(crate) mod config_window;
pub(crate) mod error_box;
pub(crate) mod help_footer;
mod log_pane;
pub mod message;
pub mod model;
pub mod plumbing;
pub(crate) mod search_box;
mod table_pane;
pub(crate) mod util;

pub use model::Model;

use crate::message::{Message, Navigate, Open};

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) enum RunningState {
    #[default]
    Running,
    Done,
}

pub(crate) fn handle_key(key: event::KeyEvent) -> Option<Message> {
    match key.code {
        KeyCode::Char('v') => Some(Message::IncreaseVerbosity),
        KeyCode::Char('g') => Some(Message::DecreaseVerbosity),
        KeyCode::Tab => Some(Message::TogglePane),
        KeyCode::Char('h') | KeyCode::Left => Some(Message::Navigate(Navigate::Left)),
        KeyCode::Char('l') | KeyCode::Right => Some(Message::Navigate(Navigate::Right)),
        KeyCode::Char('j') | KeyCode::Down => Some(Message::Navigate(Navigate::Down)),
        KeyCode::Char('k') | KeyCode::Up => Some(Message::Navigate(Navigate::Up)),
        KeyCode::Home => Some(Message::ScrollToStart),
        KeyCode::End => Some(Message::ScrollToEnd),
        KeyCode::PageDown => Some(Message::Navigate(Navigate::PageDown)),
        KeyCode::PageUp => Some(Message::Navigate(Navigate::PageUp)),
        KeyCode::Char('q') | KeyCode::Char('Q') => Some(Message::Quit),
        KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(Message::Open(Open::Search))
        }
        KeyCode::Char('+') => Some(Message::IncreaseLayoutFill),
        KeyCode::Char('-') => Some(Message::DecreaseLayoutFill),
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(Message::Open(Open::Config))
        }
        KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(Message::Refresh)
        }
        KeyCode::Char(' ') | KeyCode::Enter => Some(Message::Navigate(Navigate::Select)),
        _ => None,
    }
}
