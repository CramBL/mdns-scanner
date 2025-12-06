use mds_keybindings::Action;
use ratatui::crossterm::event::KeyEvent;

use crate::error_box::PromptResponse;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Message {
    Action(Action),
    BoxInput(KeyEvent),
    PromptResponse(PromptResponse),
    Open(Popup),
}

impl From<Action> for Message {
    fn from(a: Action) -> Self {
        Self::Action(a)
    }
}

impl From<PromptResponse> for Message {
    fn from(p: PromptResponse) -> Self {
        Self::PromptResponse(p)
    }
}

impl From<Popup> for Message {
    fn from(p: Popup) -> Self {
        Self::Open(p)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Popup {
    ConfigBox,
    SearchBox,
    ErrorBox,
    IpInfoPopUp,
}
