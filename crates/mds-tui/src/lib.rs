use ratatui::crossterm::event::{self, KeyCode, KeyModifiers};

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

use crate::message::{Message, Navigate};

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
        KeyCode::Tab => Some(Message::ToggleWindow),
        KeyCode::Char('h') | KeyCode::Left => Some(Navigate::Left.into()),
        KeyCode::Char('l') | KeyCode::Right => Some(Navigate::Right.into()),
        KeyCode::Char('j') | KeyCode::Down => Some(Navigate::Down.into()),
        KeyCode::Char('k') | KeyCode::Up => Some(Navigate::Up.into()),
        KeyCode::Home => Some(Navigate::ScrollToBeginning.into()),
        KeyCode::End => Some(Navigate::ScrollToEnd.into()),
        KeyCode::PageDown => Some(Navigate::PageDown.into()),
        KeyCode::PageUp => Some(Navigate::PageUp.into()),
        KeyCode::Char('q') | KeyCode::Char('Q') => Some(Message::Quit),
        KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(Message::PopupSearch)
        }
        KeyCode::Char('+') => Some(Message::IncreaseLayoutFill),
        KeyCode::Char('-') => Some(Message::DecreaseLayoutFill),
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(Message::PopupConfig)
        }
        KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(Message::Refresh)
        }
        KeyCode::Char(' ') | KeyCode::Enter => Some(Navigate::Select.into()),
        _ => None,
    }
}
