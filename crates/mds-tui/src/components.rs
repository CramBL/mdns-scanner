use ratatui::{
    Frame,
    crossterm::event::{KeyCode, KeyEvent, KeyEventKind},
};

use crate::{error_box::ErrorBox, message::Message};

pub(crate) trait MdsKeyHandler {
    fn handle_key_event(&mut self, key: KeyEvent) -> Result<Option<Message>, ErrorBox> {
        if key.kind == KeyEventKind::Press {
            let msg_opt = handle_global_keys(key);
            if msg_opt.is_some() {
                Ok(msg_opt)
            } else {
                self.handle_local_key_event(key)
            }
        } else {
            Ok(None)
        }
    }

    fn handle_local_key_event(&mut self, key: KeyEvent) -> Result<Option<Message>, ErrorBox>;

    fn update(&mut self, msg: Message) -> Result<Option<Message>, ErrorBox>;

    fn is_focused(&self) -> bool;

    fn render(&mut self, frame: &mut Frame<'_>);
}

pub fn handle_global_keys(key: KeyEvent) -> Option<Message> {
    match key.code {
        KeyCode::Esc => Some(Message::GlobalClose),
        _ => None,
    }
}
