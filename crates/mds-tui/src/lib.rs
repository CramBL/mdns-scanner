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

const CLOSE_KEY: KeyCode = KeyCode::Esc;
const QUIT_KEY: &[KeyCode] = &[KeyCode::Char('q'), KeyCode::Char('Q')];
const TOGGLE_FOCUS_KEY: KeyCode = KeyCode::Tab;

fn is_key_copy_to_clipboard(key: event::KeyEvent) -> bool {
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
fn is_key_basic_navigation(key: event::KeyEvent) -> bool {
    let c = key.code;
    c == CLOSE_KEY
        || c == TOGGLE_FOCUS_KEY
        || NAVIGATE_LEFT_KEY.contains(&c)
        || NAVIGATE_RIGHT_KEY.contains(&c)
        || NAVIGATE_UP_KEY.contains(&c)
        || NAVIGATE_DOWN_KEY.contains(&c)
}

fn xhandle_key(key: event::KeyEvent) -> Option<Message> {
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

#[cfg(test)]
mod handle_key_compatibility_tests {
    use super::*;
    use mds_keybindings::KeyBindings;
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    // Helper function to create a KeyEvent
    fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, modifiers)
    }

    #[test]
    fn test_handle_key_compatibility() {
        let test_cases = vec![
            // Copy to clipboard (Ctrl+C)
            key(KeyCode::Char('c'), KeyModifiers::CONTROL),
            // Close (Esc)
            key(KeyCode::Esc, KeyModifiers::empty()),
            // Toggle focus (Tab)
            key(KeyCode::Tab, KeyModifiers::empty()),
            // Navigate select (Space and Enter)
            key(KeyCode::Char(' '), KeyModifiers::empty()),
            key(KeyCode::Enter, KeyModifiers::empty()),
            // Navigate left (h and Left arrow)
            key(KeyCode::Char('h'), KeyModifiers::empty()),
            key(KeyCode::Left, KeyModifiers::empty()),
            // Navigate right (l and Right arrow)
            key(KeyCode::Char('l'), KeyModifiers::empty()),
            key(KeyCode::Right, KeyModifiers::empty()),
            // Navigate up (k and Up arrow)
            key(KeyCode::Char('k'), KeyModifiers::empty()),
            key(KeyCode::Up, KeyModifiers::empty()),
            // Navigate down (j and Down arrow)
            key(KeyCode::Char('j'), KeyModifiers::empty()),
            key(KeyCode::Down, KeyModifiers::empty()),
            // Scroll to beginning (Home)
            key(KeyCode::Home, KeyModifiers::empty()),
            // Scroll to end (End)
            key(KeyCode::End, KeyModifiers::empty()),
            // Page down
            key(KeyCode::PageDown, KeyModifiers::empty()),
            // Page up
            key(KeyCode::PageUp, KeyModifiers::empty()),
            key(KeyCode::Char('q'), KeyModifiers::empty()),
            // Increase verbosity (v)
            key(KeyCode::Char('v'), KeyModifiers::empty()),
            // Decrease verbosity (g)
            key(KeyCode::Char('g'), KeyModifiers::empty()),
            // Search (Ctrl+F)
            key(KeyCode::Char('f'), KeyModifiers::CONTROL),
            // Increase layout fill (+)
            key(KeyCode::Char('+'), KeyModifiers::empty()),
            // Decrease layout fill (-)
            key(KeyCode::Char('-'), KeyModifiers::empty()),
            // Config (Shift+C)
            key(KeyCode::Char('c'), KeyModifiers::SHIFT),
            key(KeyCode::Char('C'), KeyModifiers::SHIFT),
            // Refresh (Ctrl+R)
            key(KeyCode::Char('r'), KeyModifiers::CONTROL),
            // Test with various modifiers that should not match
            key(KeyCode::Char('c'), KeyModifiers::empty()),
            key(KeyCode::Char('c'), KeyModifiers::ALT),
            key(KeyCode::Char('f'), KeyModifiers::empty()),
            key(KeyCode::Char('f'), KeyModifiers::SHIFT),
            key(KeyCode::Char('r'), KeyModifiers::empty()),
            key(KeyCode::Char('r'), KeyModifiers::SHIFT),
            // Unknown keys that should return None
            key(KeyCode::Char('a'), KeyModifiers::empty()),
            key(KeyCode::Char('z'), KeyModifiers::empty()),
            key(KeyCode::Char('1'), KeyModifiers::empty()),
            key(KeyCode::F(1), KeyModifiers::empty()),
            key(KeyCode::F(12), KeyModifiers::empty()),
            key(KeyCode::Backspace, KeyModifiers::empty()),
            key(KeyCode::Delete, KeyModifiers::empty()),
            key(KeyCode::Insert, KeyModifiers::empty()),
            // Edge cases with multiple modifiers
            key(
                KeyCode::Char('c'),
                KeyModifiers::CONTROL | KeyModifiers::SHIFT,
            ),
            key(KeyCode::Char('f'), KeyModifiers::CONTROL),
        ];

        let mut failures = Vec::new();

        let new_handle_key = KeyBindings::default();

        for (idx, test_key) in test_cases.iter().enumerate() {
            let old_result = xhandle_key(*test_key);
            let new_result = new_handle_key.handle_key(*test_key).map(|a| a.into());

            if old_result != new_result {
                failures.push((idx, *test_key, old_result, new_result));
            }
        }

        if !failures.is_empty() {
            let mut error_msg = String::from("handle_key compatibility test failed:\n");
            for (idx, key, old, new) in failures {
                error_msg.push_str(&format!(
                    "  Test case {}: {:?}\n    Old: {:?}\n    New: {:?}\n",
                    idx, key, old, new
                ));
            }
            panic!("{}", error_msg);
        }
    }

    #[test]
    fn test_all_printable_chars() {
        // Test all ASCII printable characters that aren't already covered
        let special_chars = vec![
            'h', 'j', 'k', 'l', 'q', 'Q', 'v', 'g', 'f', 'c', 'C', 'r', '+', '-', ' ',
        ];

        let new_handle_key = KeyBindings::default();

        for ch in ('!'..'~').filter(|c| !special_chars.contains(c)) {
            let test_key = key(KeyCode::Char(ch), KeyModifiers::empty());
            let old_result = xhandle_key(test_key);
            let new_result = new_handle_key.handle_key(test_key).map(|a| a.into());

            assert_eq!(
                old_result, new_result,
                "Mismatch for character '{}': old={:?}, new={:?}",
                ch, old_result, new_result
            );
        }
    }

    #[test]
    fn test_modifier_combinations() {
        // Test important keys with various modifier combinations
        let keys = vec![
            KeyCode::Char('a'),
            KeyCode::Enter,
            KeyCode::Tab,
            KeyCode::Left,
        ];

        let modifier_combos = vec![
            KeyModifiers::empty(),
            KeyModifiers::SHIFT,
            KeyModifiers::CONTROL,
            KeyModifiers::ALT,
            KeyModifiers::SHIFT | KeyModifiers::CONTROL,
            KeyModifiers::SHIFT | KeyModifiers::ALT,
            KeyModifiers::CONTROL | KeyModifiers::ALT,
            KeyModifiers::SHIFT | KeyModifiers::CONTROL | KeyModifiers::ALT,
        ];

        let mut failures = Vec::new();

        let keymap = KeyBindings::default();

        for code in &keys {
            for modifiers in &modifier_combos {
                let test_key = key(*code, *modifiers);
                let old_result = xhandle_key(test_key);
                let new_result = keymap.handle_key(test_key).map(|a| a.into());

                if old_result != new_result {
                    failures.push((*code, *modifiers, old_result, new_result));
                }
            }
        }

        if !failures.is_empty() {
            let mut error_msg = String::from("test_modifier_combinations failed:\n");
            for (code, modifiers, old, new) in failures {
                error_msg.push_str(&format!(
                    "  {:?} with {:?}:\n    Old: {:?}\n    New: {:?}\n",
                    code, modifiers, old, new
                ));
            }
            panic!("{}", error_msg);
        }
    }
}
