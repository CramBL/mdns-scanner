use ratatui::crossterm::event;

use crate::{error_box::ErrorBox, message::Message};

pub(crate) trait MdsKeyHandler {
    fn handle_key_event(&mut self, key: event::KeyEvent) -> Result<Option<Message>, ErrorBox>;

    fn is_focused(&self) -> bool;
}
