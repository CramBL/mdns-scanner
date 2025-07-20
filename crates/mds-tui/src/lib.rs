use ratatui::{
    Frame,
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    layout::{Constraint, Layout},
};

pub(crate) mod config_window;
pub(crate) mod error_box;
pub(crate) mod help_footer;
mod log_pane;
pub mod model;
pub mod plumbing;
pub(crate) mod search_box;
mod table_pane;
pub(crate) mod util;

pub use model::Model;

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) enum RunningState {
    #[default]
    Running,
    Done,
}

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

/// Convert Event to Message
pub fn handle_event(m: &mut model::Model) -> color_eyre::Result<Option<Message>> {
    if event::poll(m.passive_refresh_interval())? {
        if let Event::Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Press {
                if key.code == KeyCode::Esc
                    && (m.is_search_active()
                        || m.is_config_open()
                        || m.is_error_open()
                        || m.is_ip_info_popup_open())
                {
                    return Ok(Some(Message::CloseBox));
                }
                if m.is_search_active() {
                    if key.code == KeyCode::Down
                        || key.code == KeyCode::Up
                        || key.code == KeyCode::Enter
                    {
                        return Ok(handle_key(key));
                    }
                    return Ok(Some(Message::BoxInput(key)));
                } else if m.is_config_open() {
                    return Ok(Some(Message::BoxInput(key)));
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
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(Message::PopupConfig)
        }
        KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(Message::Refresh)
        }
        KeyCode::Char(' ') | KeyCode::Enter => Some(Message::NavigateSelect),
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
        Message::CloseBox => {
            if model.is_error_open() {
                model.close_error();
            } else if model.is_search_active() {
                model.set_search_disabled();
            } else {
                model.close_action();
            }
        }
        Message::BoxInput(key_event) => {
            if model.is_error_open() {
                return model.error_box_input(key_event);
            } else if model.is_search_active() {
                model.search_box_input(key_event);
            } else if model.is_config_open() {
                model.config_window_input(key_event);
            }
        }
        Message::ScrollToStart => model.scroll_to_start(),
        Message::ScrollToEnd => model.scroll_to_end(),
        Message::NavigateDown => model.next_row(),
        Message::NavigateUp => model.previous_row(),
        Message::NavigateRight => model.navigate_right(),
        Message::NavigateLeft => model.navigate_left(),
        Message::NavigatePageUp => model.navigate_page_up(),
        Message::NavigatePageDown => model.navigate_page_down(),
        Message::NavigateSelect => model.navigate_select(),
        Message::IncreaseLayoutFill => model.increase_layout_fill(),
        Message::DecreaseLayoutFill => model.decrease_layout_fill(),
        Message::PopupConfig => model.open_config(),
        Message::Confirm => {
            model.confirm_action();
        }
        Message::Cancel => {
            model.cancel_action();
        }
        Message::Refresh => model.refresh(),
    };
    None
}

pub fn view(model: &mut model::Model, frame: &mut Frame) {
    let constr = model.pane_constraints();
    let pane_constraints = vec![constr[0], constr[1]];
    let layout = Layout::default()
        .constraints(pane_constraints)
        .split(frame.area());
    let top = layout[0];
    let mut bottom = layout[1];

    if !model.compact_ui() {
        let vertical = &Layout::vertical([Constraint::Min(5), Constraint::Length(4)]);
        let rects = vertical.split(bottom);
        model.render_footer(frame, rects[1]);
        bottom = rects[0];
    }

    model.set_current_frame_log_pane_area(bottom);
    model.set_current_frame_table_pane_area(top);
    model.render_log_pane(frame, bottom);
    model.render_table_pane(frame, top);
    model.render_search_box(frame, top);
    model.render_config_window(frame);
    model.render_error_box(frame);
}
