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

use crate::message::{Message, Navigate, Popup};

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) enum RunningState {
    #[default]
    Running,
    Done,
}

pub const CLOSE_KEY: KeyCode = KeyCode::Esc;
const QUIT_KEY: &[KeyCode] = &[KeyCode::Char('q'), KeyCode::Char('Q')];

pub(crate) fn handle_key(key: event::KeyEvent) -> Option<Message> {
    let msg = match key.code {
        CLOSE_KEY => Message::CloseBox,
        KeyCode::Char('v') => Message::IncreaseVerbosity,
        KeyCode::Char('g') => Message::DecreaseVerbosity,
        KeyCode::Tab => Message::ToggleWindow,
        KeyCode::Char('h') | KeyCode::Left => Navigate::Left.into(),
        KeyCode::Char('l') | KeyCode::Right => Navigate::Right.into(),
        KeyCode::Char('j') | KeyCode::Down => Navigate::Down.into(),
        KeyCode::Char('k') | KeyCode::Up => Navigate::Up.into(),
        KeyCode::Home => Navigate::ScrollToBeginning.into(),
        KeyCode::End => Navigate::ScrollToEnd.into(),
        KeyCode::PageDown => Navigate::PageDown.into(),
        KeyCode::PageUp => Navigate::PageUp.into(),
        k if QUIT_KEY.contains(&k) => Message::Quit,
        KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Popup::SearchBox.into()
        }
        KeyCode::Char('+') => Message::IncreaseLayoutFill,
        KeyCode::Char('-') => Message::DecreaseLayoutFill,
        KeyCode::Char('c') | KeyCode::Char('C') if key.modifiers.contains(KeyModifiers::SHIFT) => {
            Popup::Config.into()
        }
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Message::CopyToClipboard
        }
        KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => Message::Refresh,
        KeyCode::Char(' ') | KeyCode::Enter => Navigate::Select.into(),
        _ => return None,
    };
    Some(msg)
}
