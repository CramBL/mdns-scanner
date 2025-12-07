use serde::{Deserialize, Serialize};
use strum::{Display, EnumCount};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Serialize, Deserialize, EnumCount, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum Action {
    Quit,
    Close,
    IncreaseVerbosity,
    DecreaseVerbosity,
    ToggleFocus,
    NavigateSelect,
    NavigateRight,
    NavigateLeft,
    NavigateDown,
    NavigateUp,
    NavigatePageup,
    NavigatePagedown,
    NavigateScrollToEnd,
    NavigateScrollToBeginning,
    IncreaseLayoutFill,
    DecreaseLayoutFill,
    Refresh,
    CopyToClipboard,
    Config,
    SaveConfig,
    Search,
    Keybindings,
}

impl Action {
    pub fn is_basic_navigation(&self) -> bool {
        match self {
            Action::NavigateRight
            | Action::NavigateLeft
            | Action::NavigateDown
            | Action::NavigateUp
            | Action::ToggleFocus
            | Action::Close => true,
            Action::IncreaseVerbosity
            | Action::DecreaseVerbosity
            | Action::NavigateSelect
            | Action::NavigatePageup
            | Action::NavigatePagedown
            | Action::NavigateScrollToEnd
            | Action::NavigateScrollToBeginning
            | Action::IncreaseLayoutFill
            | Action::DecreaseLayoutFill
            | Action::Refresh
            | Action::CopyToClipboard
            | Action::Config
            | Action::SaveConfig
            | Action::Search
            | Action::Keybindings
            | Action::Quit => false,
        }
    }
}
