use std::collections::HashMap;

use ratatui::{
    layout::{Alignment, Constraint, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, BorderType, Borders, Cell, Clear, Row, Scrollbar, ScrollbarOrientation,
        ScrollbarState, StatefulWidget, Table, TableState, Widget,
    },
};

use mds_keybindings::{KeyBindings, key_event_to_string};

use crate::util;

pub struct KeybindingsPopup<'km> {
    keymap: &'km KeyBindings,
}

impl<'km> KeybindingsPopup<'km> {
    pub fn new(keybindings: &'km KeyBindings) -> Self {
        Self {
            keymap: keybindings,
        }
    }

    pub fn get_formatted_bindings(keybindings: &KeyBindings) -> Vec<(String, Vec<String>)> {
        let mut grouped_bindings: HashMap<String, Vec<String>> = HashMap::new();

        for bindings in keybindings.values() {
            for (key_event, action) in bindings {
                let action_name = action.to_string();
                let key_string = key_event_to_string(key_event);

                grouped_bindings
                    .entry(action_name)
                    .or_default()
                    .push(key_string);
            }
        }

        let mut rows_data: Vec<(String, Vec<String>)> = grouped_bindings
            .into_iter()
            .map(|(action, mut keys)| {
                keys.sort();
                (action, keys)
            })
            .collect();

        rows_data.sort_by(|(a, _), (b, _)| a.cmp(b));
        rows_data
    }
}

impl<'a> StatefulWidget for KeybindingsPopup<'a> {
    type State = TableState;

    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer, state: &mut Self::State) {
        let data = Self::get_formatted_bindings(self.keymap);
        let total_items = data.len();
        let max_action_width = data.iter().map(|(a, _)| a.len()).max().unwrap_or(10) as u16;

        let rows: Vec<Row> = data
            .into_iter()
            .map(|(action, keys)| {
                let mut key_spans = Vec::new();
                for (i, key) in keys.iter().enumerate() {
                    if i > 0 {
                        key_spans.push(Span::raw(", "));
                    }
                    key_spans.push(Span::styled(key.clone(), Style::default().fg(Color::Blue)));
                }
                let keys_line = Line::from(key_spans);

                Row::new(vec![
                    Cell::from(Text::from(action).style(Style::default().fg(Color::Green))),
                    Cell::from(keys_line),
                ])
            })
            .collect();

        let block = Block::default()
            .title(" Keybindings ")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded);

        let viewport_height = area.height.saturating_sub(6) as usize;

        let constraints = [
            Constraint::Length(max_action_width + 4),
            Constraint::Fill(1),
        ];

        let area = util::center(area, Constraint::Percentage(50), Constraint::Percentage(70));

        Clear.render(area, buf);

        StatefulWidget::render(
            Table::new(rows, constraints)
                .block(block)
                .header(
                    Row::new(vec!["Action", "Keystroke"])
                        .style(Style::default().add_modifier(Modifier::BOLD))
                        .bottom_margin(1),
                )
                .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED)),
            area,
            buf,
            state,
        );

        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None);

        let mut scrollbar_state = ScrollbarState::new(total_items)
            .position(state.selected().unwrap_or(0))
            .viewport_content_length(viewport_height);

        scrollbar.render(area, buf, &mut scrollbar_state);
    }
}
