use std::time::Duration;

use ratatui::{
    Frame,
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    layout::Layout,
};

mod log_pane;
pub mod model;
pub mod plumbing;
pub(crate) mod search_box;
mod table_pane;

pub use model::Model;

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) enum RunningState {
    #[default]
    Running,
    Done,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Message {
    IncreaseVerbosity,
    DecreaseVerbosity,
    ToggleWindow,
    Quit,
    PopupSearch,
    CloseSearch,
    SearchInput(KeyEvent),
    ScrollToStart,
    ScrollToEnd,
    NavigateRight,
    NavigateLeft,
    NavigateDown,
    NavigateUp,
    NavigatePageUp,
    NavigatePageDown,
    IncreaseLayoutFill,
    DecreaseLayoutFill,
}

/// Convert Event to Message
pub fn handle_event(m: &model::Model) -> color_eyre::Result<Option<Message>> {
    if event::poll(Duration::from_millis(100))? {
        if let Event::Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Press {
                if m.is_search_active() {
                    if key.code == KeyCode::Esc {
                        return Ok(Some(Message::CloseSearch));
                    } else if key.code == KeyCode::Down || key.code == KeyCode::Up {
                        return Ok(handle_key(key));
                    }
                    return Ok(Some(Message::SearchInput(key)));
                }
                return Ok(handle_key(key));
            }
        }
    }
    Ok(None)
}

pub(crate) fn handle_key(key: event::KeyEvent) -> Option<Message> {
    match key.code {
        KeyCode::Char('v') => Some(Message::IncreaseVerbosity),
        KeyCode::Char('g') => Some(Message::DecreaseVerbosity),
        KeyCode::Tab => Some(Message::ToggleWindow),
        KeyCode::Char('h') | KeyCode::Left => Some(Message::NavigateLeft),
        KeyCode::Char('l') | KeyCode::Right => Some(Message::NavigateRight),
        KeyCode::Char('j') | KeyCode::Down => Some(Message::NavigateDown),
        KeyCode::Char('k') | KeyCode::Up => Some(Message::NavigateUp),
        KeyCode::Home => Some(Message::ScrollToStart),
        KeyCode::End => Some(Message::ScrollToEnd),
        KeyCode::PageDown => Some(Message::NavigatePageDown),
        KeyCode::PageUp => Some(Message::NavigatePageUp),
        KeyCode::Char('q') | KeyCode::Char('Q') => Some(Message::Quit),
        KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(Message::PopupSearch)
        }
        KeyCode::Char('+') => Some(Message::IncreaseLayoutFill),
        KeyCode::Char('-') => Some(Message::DecreaseLayoutFill),
        _ => None,
    }
}

pub fn update(model: &mut model::Model, msg: Message) -> Option<Message> {
    match msg {
        Message::IncreaseVerbosity => {
            model.increase_verbosity();
        }
        Message::DecreaseVerbosity => {
            model.decrease_verbosity();
        }
        Message::ToggleWindow => model.toggle_selected_pane(),
        Message::Quit => {
            model.set_done();
        }
        Message::PopupSearch => model.set_search_active(),
        Message::CloseSearch => model.set_search_disabled(),
        Message::SearchInput(key_event) => model.search_box_input(key_event),
        Message::ScrollToStart => model.scroll_to_start(),
        Message::ScrollToEnd => model.scroll_to_end(),
        Message::NavigateDown => model.next_row(),
        Message::NavigateUp => model.previous_row(),
        Message::NavigateRight => model.navigate_right(),
        Message::NavigateLeft => model.navigate_left(),
        Message::NavigatePageUp => model.navigate_page_up(),
        Message::NavigatePageDown => model.navigate_page_down(),
        Message::IncreaseLayoutFill => model.increase_layout_fill(),
        Message::DecreaseLayoutFill => model.decrease_layout_fill(),
    };
    None
}

pub fn view(model: &mut model::Model, frame: &mut Frame) {
    let layout = Layout::default()
        .constraints(model.pane_constraints())
        .split(frame.area());
    let top = layout[0];
    let bottom = layout[1];

    model.set_current_frame_log_pane_area(top);
    model.set_current_frame_table_pane_area(bottom);
    model.render_log_pane(frame, top);
    model.render_table_pane(frame, bottom);
    model.render_search_box(frame, bottom);
}
