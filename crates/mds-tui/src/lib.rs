use mds_keybindings::Action;
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

use crate::message::Message;

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) enum RunningState {
    #[default]
    Running,
    Done,
}

pub const CLOSE_KEY: KeyCode = KeyCode::Esc;
const QUIT_KEY: &[KeyCode] = &[KeyCode::Char('q'), KeyCode::Char('Q')];
pub const TOGGLE_FOCUS_KEY: KeyCode = KeyCode::Tab;

pub(crate) fn is_key_copy_to_clipboard(key: event::KeyEvent) -> bool {
    key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL)
}

const NAVIGATE_SELECT_KEY: [KeyCode; 2] = [KeyCode::Char(' '), KeyCode::Enter];
const NAVIGATE_LEFT_KEY: [KeyCode; 2] = [KeyCode::Char('h'), KeyCode::Left];
const NAVIGATE_RIGHT_KEY: &[KeyCode] = &[KeyCode::Char('l'), KeyCode::Right];
const NAVIGATE_UP_KEY: &[KeyCode] = &[KeyCode::Char('k'), KeyCode::Up];
const NAVIGATE_DOWN_KEY: &[KeyCode] = &[KeyCode::Char('j'), KeyCode::Down];
const NAVIGATE_SCROLL_TO_BEGINNING_KEY: KeyCode = KeyCode::Home;
const NAVIGATE_SCROLL_TO_END_KEY: KeyCode = KeyCode::End;
const NAVIGATE_PAGE_UP_KEY: KeyCode = KeyCode::PageUp;
const NAVIGATE_PAGE_DOWN_KEY: KeyCode = KeyCode::PageDown;

// Is the key basic navigation, left/right/up/bottom/close/toggle
pub(crate) fn is_key_basic_navigation(key: event::KeyEvent) -> bool {
    let c = key.code;
    c == CLOSE_KEY
        || c == TOGGLE_FOCUS_KEY
        || NAVIGATE_LEFT_KEY.contains(&c)
        || NAVIGATE_RIGHT_KEY.contains(&c)
        || NAVIGATE_UP_KEY.contains(&c)
        || NAVIGATE_DOWN_KEY.contains(&c)
}

pub(crate) fn handle_key(key: event::KeyEvent) -> Option<Message> {
    if is_key_copy_to_clipboard(key) {
        return Some(Action::CopyToClipboard.into());
    }

    let msg = match key.code {
        CLOSE_KEY => Action::Close.into(),
        TOGGLE_FOCUS_KEY => Action::ToggleFocus.into(),
        k if NAVIGATE_SELECT_KEY.contains(&k) => Action::NavigateSelect.into(),
        k if NAVIGATE_LEFT_KEY.contains(&k) => Action::NavigateLeft.into(),
        k if NAVIGATE_RIGHT_KEY.contains(&k) => Action::NavigateRight.into(),
        k if NAVIGATE_DOWN_KEY.contains(&k) => Action::NavigateDown.into(),
        k if NAVIGATE_UP_KEY.contains(&k) => Action::NavigateUp.into(),
        NAVIGATE_SCROLL_TO_BEGINNING_KEY => Action::NavigateScrollToBeginning.into(),
        NAVIGATE_SCROLL_TO_END_KEY => Action::NavigateScrollToEnd.into(),
        NAVIGATE_PAGE_DOWN_KEY => Action::NavigatePagedown.into(),
        NAVIGATE_PAGE_UP_KEY => Action::NavigatePageup.into(),

        k if QUIT_KEY.contains(&k) => Action::Quit.into(),
        KeyCode::Char('v') => Action::IncreaseVerbosity.into(),
        KeyCode::Char('g') => Action::DecreaseVerbosity.into(),
        KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Action::Search.into()
        }
        KeyCode::Char('+') => Action::IncreaseLayoutFill.into(),
        KeyCode::Char('-') => Action::DecreaseLayoutFill.into(),
        KeyCode::Char('c') | KeyCode::Char('C') if key.modifiers.contains(KeyModifiers::SHIFT) => {
            Action::Config.into()
        }
        KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Action::Refresh.into()
        }
        _ => return None,
    };

    Some(msg)
}

#[cfg(test)]
mod tests {
    use ratatui::crossterm::event::KeyEvent;

    use super::*;

    #[test]
    fn test_key_is_basic_nav() {
        let k = KeyEvent::new(KeyCode::Left, KeyModifiers::empty());
        assert!(is_key_basic_navigation(k));
    }
}
