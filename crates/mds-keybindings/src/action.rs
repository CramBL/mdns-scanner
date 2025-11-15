use serde::{Deserialize, Serialize};
use strum::Display;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Action {
    Quit,
    Close,
    IncreaseVerbosity,
    DecreaseVerbosity,
    ToggleWindow,
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
    Search,
}
