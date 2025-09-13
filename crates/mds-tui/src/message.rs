use ratatui::crossterm::event::KeyEvent;

use crate::error_box::PromptResponse;

#[derive(Clone, Copy, PartialEq)]
pub enum Message {
    IncreaseVerbosity,
    DecreaseVerbosity,
    ToggleWindow,
    Quit,
    CloseBox,
    BoxInput(KeyEvent),
    Navigate(Navigate),
    PromptResponse(PromptResponse),
    IncreaseLayoutFill,
    DecreaseLayoutFill,
    Refresh,
    CopyToClipboard,
    Open(Popup),
}

impl From<Navigate> for Message {
    fn from(nav: Navigate) -> Self {
        Self::Navigate(nav)
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
    Config,
    SearchBox,
    ErrorBox,
    IpInfoPopUp,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Navigate {
    Select,
    Right,
    Left,
    Down,
    Up,
    PageUp,
    PageDown,
    ScrollToEnd,
    ScrollToBeginning,
}
