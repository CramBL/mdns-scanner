pub(crate) mod config_window;
pub(crate) mod error_box;
pub(crate) mod keybindings;
mod log_pane;
pub mod message;
pub mod model;
pub(crate) mod option_selector;
pub mod plumbing;
pub mod scan_backend;
pub(crate) mod search_box;
mod table_pane;
pub(crate) mod util;

pub use model::Model;
pub use scan_backend::ScanBackend;

use crate::message::Message;

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) enum RunningState {
    #[default]
    Running,
    Done,
}
