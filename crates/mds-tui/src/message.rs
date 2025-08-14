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
    Navigate(Navigate),
    IncreaseLayoutFill,
    DecreaseLayoutFill,
    Refresh,
}

impl From<Navigate> for Message {
    fn from(nav: Navigate) -> Self {
        match nav {
            Navigate::Select => Message::Navigate(Navigate::Select),
            Navigate::Right => Message::Navigate(Navigate::Right),
            Navigate::Left => Message::Navigate(Navigate::Left),
            Navigate::Down => Message::Navigate(Navigate::Down),
            Navigate::Up => Message::Navigate(Navigate::Up),
            Navigate::PageUp => Message::Navigate(Navigate::PageUp),
            Navigate::PageDown => Message::Navigate(Navigate::PageDown),
            Navigate::ScrollToEnd => Message::Navigate(Navigate::ScrollToEnd),
            Navigate::ScrollToBeginning => Message::Navigate(Navigate::ScrollToBeginning),
        }
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
