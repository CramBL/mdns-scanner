use ratatui::crossterm::event::KeyEvent;

use crate::error_box::PromptResponse;

#[derive(Clone, Copy, PartialEq)]
pub enum Message {
    IncreaseVerbosity,
    DecreaseVerbosity,
    ToggleWindow,
    Quit,
    PopupConfig,
    PopupSearch,
    CloseBox,
    BoxInput(KeyEvent),
    Navigate(Navigate),
    PromptResponse(PromptResponse),
    IncreaseLayoutFill,
    DecreaseLayoutFill,
    Refresh,
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
