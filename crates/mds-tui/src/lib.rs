use ratatui::crossterm::event::{self, Event, KeyCode, KeyModifiers};

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

/// Convert Event to Message
pub fn handle_event(m: &mut model::Model) -> color_eyre::Result<Option<Message>> {
    if event::poll(m.passive_refresh_interval())? {
        if let Event::Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Press {
                if key.code == KeyCode::Esc
                    && (m.is_search_active()
                        || m.is_config_open()
                        || m.is_error_open()
                        || m.is_ip_info_popup_open())
                {
                    return Ok(Some(Message::CloseBox));
                }
                if m.is_search_active() {
                    if key.code == KeyCode::Down
                        || key.code == KeyCode::Up
                        || key.code == KeyCode::Enter
                    {
                        return Ok(handle_key(key));
                    }
                    return Ok(Some(Message::BoxInput(key)));
                } else if m.is_config_open() {
                    return Ok(Some(Message::BoxInput(key)));
                }
                return Ok(handle_key(key));
            }
        }
    }
    Ok(None)
}

pub(crate) fn handle_key(key: event::KeyEvent) -> Option<Message> {
    match key.code {
        KeyCode::Char('v') => Some(Message::IncreaseVerbosity),
        KeyCode::Char('g') => Some(Message::DecreaseVerbosity),
        KeyCode::Tab => Some(Message::ToggleWindow),
        KeyCode::Char('h') | KeyCode::Left => Some(Message::NavigateLeft),
        KeyCode::Char('l') | KeyCode::Right => Some(Message::NavigateRight),
        KeyCode::Char('j') | KeyCode::Down => Some(Message::NavigateDown),
        KeyCode::Char('k') | KeyCode::Up => Some(Message::NavigateUp),
        KeyCode::Home => Some(Message::ScrollToStart),
        KeyCode::End => Some(Message::ScrollToEnd),
        KeyCode::PageDown => Some(Message::NavigatePageDown),
        KeyCode::PageUp => Some(Message::NavigatePageUp),
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
        KeyCode::Char(' ') | KeyCode::Enter => Some(Message::NavigateSelect),
        _ => None,
    }
}
