use ratatui::crossterm::event;

use crate::message::Message;

pub(crate) trait MdsKeyHandler {
    fn handle_key_event(&mut self, key: event::KeyEvent) -> color_eyre::Result<Option<Message>>;

    fn is_focused(&self) -> bool;
}
