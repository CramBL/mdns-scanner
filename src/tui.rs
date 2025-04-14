use std::time::Duration;

use ratatui::{
    Frame,
    crossterm::event::{self, Event, KeyCode},
    layout::{Constraint, Layout},
    style::Stylize,
    widgets::{Block, List, ListItem},
};

use crate::log::LogMessage;

pub(crate) mod model;
pub(crate) mod plumbing;
mod table;
mod util;

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) enum RunningState {
    #[default]
    Running,
    Done,
}

#[derive(PartialEq)]
pub(crate) enum Message {
    IncreaseVerbosity,
    DecreaseVerbosity,
    NextRow,
    PreviousRow,
    Reset,
    Quit,
}

/// Convert Event to Message
///
/// We don't need to pass in a `model` to this function in this example
/// but you might need it as your project evolves
pub(crate) fn handle_event(_: &model::Model) -> color_eyre::Result<Option<Message>> {
    if event::poll(Duration::from_millis(100))? {
        if let Event::Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Press {
                return Ok(handle_key(key));
            }
        }
    }
    Ok(None)
}

pub(crate) fn handle_key(key: event::KeyEvent) -> Option<Message> {
    match key.code {
        KeyCode::Char('j') => Some(Message::IncreaseVerbosity),
        KeyCode::Char('k') => Some(Message::DecreaseVerbosity),
        KeyCode::Down => Some(Message::NextRow),
        KeyCode::Up => Some(Message::PreviousRow),
        KeyCode::Char('q') | KeyCode::Char('Q') => Some(Message::Quit),
        _ => None,
    }
}

pub(crate) fn update(model: &mut model::Model, msg: Message) -> Option<Message> {
    match msg {
        Message::IncreaseVerbosity => {
            model.increase_verbosity();
        }
        Message::DecreaseVerbosity => {
            model.decrease_verbosity();
        }
        Message::NextRow => model.next_row(),
        Message::PreviousRow => model.previous_row(),
        Message::Reset => (),
        Message::Quit => {
            model.set_done();
        }
    };
    None
}

pub(crate) fn view(model: &mut model::Model, frame: &mut Frame) {
    let [top, bottom] = Layout::vertical([Constraint::Fill(1); 2]).areas(frame.area());

    let logs = model.latest_logs();
    let mut list_items: Vec<ListItem> = vec![];
    for msg in logs {
        match msg {
            LogMessage::Error(s) => {
                list_items.push(ListItem::new(s.as_ref()).red());
            }
            LogMessage::Warn(s) => list_items.push(ListItem::new(s.as_ref()).yellow()),
            LogMessage::Info(s) => list_items.push(ListItem::new(s.as_ref())),
            LogMessage::Debug(s) => list_items.push(ListItem::new(s.as_ref()).cyan()),
            LogMessage::Trace(s) => list_items.push(ListItem::new(s.as_ref()).blue()),
        }
    }

    let list = List::new(list_items);

    frame.render_widget(
        list.block(Block::bordered().title(format!("Log Level: {}", model.log_level()))),
        top,
    );

    model.set_colors();
    model.render_table(frame, bottom);
    model.render_scrollbar(frame, bottom);
}
