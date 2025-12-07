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

pub struct FormattedBindings {
    pub data: Vec<(String, Vec<String>)>,
    pub max_action_width: u16,
    pub max_keys_width: u16,
}

impl FormattedBindings {
    pub fn from_keybindings(keybindings: &KeyBindings) -> Self {
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

        let mut max_action_width: u16 = 10;
        let mut max_keys_width: u16 = 10;
        let mut rows_data: Vec<(String, Vec<String>)> = grouped_bindings
            .into_iter()
            .map(|(action, mut keys)| {
                max_action_width = max_action_width.max(action.len() as u16);
                keys.sort_unstable();

                // Calculate width of concatenated keys (with ", " separators)
                let keys_width =
                    keys.iter().map(|k| k.len()).sum::<usize>() + keys.len().saturating_sub(1) * 2;
                max_keys_width = max_keys_width.max(keys_width as u16);
                (action, keys)
            })
            .collect();

        rows_data.sort_unstable_by(|(a, _), (b, _)| a.cmp(b));

        Self {
            data: rows_data,
            max_action_width,
            max_keys_width,
        }
    }
}

pub struct KeybindingsPopup<'km> {
    keymap: &'km KeyBindings,
    formatted: Option<FormattedBindings>,
}

impl<'km> KeybindingsPopup<'km> {
    pub fn new(keybindings: &'km KeyBindings) -> Self {
        Self {
            keymap: keybindings,
            formatted: None,
        }
    }

    fn adaptive_width_constraint(screen_width: u16, desired_width: u16) -> Constraint {
        if screen_width > 120 {
            Constraint::Length(desired_width.min(screen_width * 60 / 100))
        } else if screen_width > 80 {
            let percentage = 60 + ((120 - screen_width) * 20 / 40);
            Constraint::Percentage(percentage.into())
        } else {
            Constraint::Percentage(90)
        }
    }

    fn adaptive_height_constraint(screen_height: u16) -> Constraint {
        if screen_height > 40 {
            Constraint::Percentage(60)
        } else if screen_height > 24 {
            let percentage = 70 + ((40 - screen_height) * 15 / 16);
            Constraint::Percentage(percentage.into())
        } else {
            Constraint::Percentage(90)
        }
    }
}

impl<'a> StatefulWidget for KeybindingsPopup<'a> {
    type State = TableState;

    fn render(mut self, area: Rect, buf: &mut ratatui::buffer::Buffer, state: &mut Self::State) {
        let formatted = self
            .formatted
            .get_or_insert_with(|| FormattedBindings::from_keybindings(self.keymap));

        let total_items = formatted.data.len();
        let max_action_width = formatted.max_action_width;
        let max_keys_width = formatted.max_keys_width;

        let mut rows = Vec::with_capacity(total_items);
        for (action, keys) in &formatted.data {
            let mut key_spans = Vec::with_capacity(keys.len() * 2);

            for (i, key) in keys.iter().enumerate() {
                if i > 0 {
                    key_spans.push(Span::raw(", "));
                }
                key_spans.push(Span::styled(key.as_str(), Style::default().fg(Color::Blue)));
            }

            rows.push(Row::new(vec![
                Cell::from(Text::from(action.as_str()).style(Style::default().fg(Color::Green))),
                Cell::from(Line::from(key_spans)),
            ]));
        }

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

        let desired_width = max_action_width + max_keys_width + 10; // +10 for borders, padding, margin
        let width_constraint = Self::adaptive_width_constraint(area.width, desired_width);
        let height_constraint = Self::adaptive_height_constraint(area.height);

        let area = util::center(area, width_constraint, height_constraint);

        let area = util::center(area, width_constraint, height_constraint);

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
