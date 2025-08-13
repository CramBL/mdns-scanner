use ratatui::crossterm::event::KeyEvent;

use crate::error_box::{ErrorBox, PromptResponse};

#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    ConfirmAction,
    Cancel,
    IncreaseVerbosity,
    DecreaseVerbosity,
    TogglePane,
    Quit,
    CloseBox,
    BoxInput(KeyEvent),
    ScrollToStart,
    ScrollToEnd,
    Navigate(Navigate),
    IncreaseLayoutFill,
    DecreaseLayoutFill,
    Refresh,
    PromptResponse(PromptResponse),
    Error(ErrorBox),
    SaveConfig,
    GlobalClose,
    Open(Open),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Open {
    Config,
    Search,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Navigate {
    Select,
    Right,
    Left,
    Down,
    Up,
    PageUp,
    PageDown,
}
