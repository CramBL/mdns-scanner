use ratatui::crossterm::event::KeyEvent;

#[derive(Clone, Copy, PartialEq)]
pub enum Message {
    Confirm,
    Cancel,
    IncreaseVerbosity,
    DecreaseVerbosity,
    ToggleWindow,
    Quit,
    PopupConfig,
    PopupSearch,
    CloseBox,
    BoxInput(KeyEvent),
    ScrollToStart,
    ScrollToEnd,
    NavigateSelect,
    NavigateRight,
    NavigateLeft,
    NavigateDown,
    NavigateUp,
    NavigatePageUp,
    NavigatePageDown,
    IncreaseLayoutFill,
    DecreaseLayoutFill,
    Refresh,
}
