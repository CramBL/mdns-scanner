use std::collections::HashMap;

use ratatui::{
    layout::{Constraint, Rect},
    style::Modifier,
    text::{Line, Span, Text},
    widgets::{
        Block, BorderType, Borders, Cell, Clear, Row, Scrollbar, ScrollbarOrientation,
        ScrollbarState, StatefulWidget, Table, TableState, Widget,
    },
};

use mds_keybindings::{KeyBindings, key_event_to_string};

use crate::table_pane::TableColors;
use crate::util;

pub struct FormattedBindings {
    pub data: Vec<(String, Vec<String>)>,
    pub max_action_width: u16,
    pub max_keys_width: u16,
}

impl FormattedBindings {
    pub fn from_keybindings(keybindings: &KeyBindings) -> Self {
        let mut grouped_bindings: HashMap<String, Vec<String>> = HashMap::new();

        for bindings in keybindings.0.values() {
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
                // Sort by length first, then alphabetically for same-length keys
                keys.sort_unstable_by(|a, b| a.len().cmp(&b.len()).then_with(|| a.cmp(b)));

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

pub struct KeybindingsPopup<'km, 't> {
    keymap: &'km KeyBindings,
    theme: &'t TableColors,
    formatted: Option<FormattedBindings>,
}

impl<'km, 't> KeybindingsPopup<'km, 't> {
    pub fn new(keybindings: &'km KeyBindings, theme: &'t TableColors) -> Self {
        Self {
            keymap: keybindings,
            theme,
            formatted: None,
        }
    }

    fn adaptive_width_constraint(screen_width: u16, desired_width: u16) -> Constraint {
        if screen_width > 120 {
            Constraint::Length(desired_width.min(screen_width * 60 / 100))
        } else if screen_width > 80 {
            let percentage = 60 + ((120 - screen_width) * 20 / 40);
            Constraint::Percentage(percentage)
        } else {
            Constraint::Percentage(90)
        }
    }
}

impl<'a> StatefulWidget for KeybindingsPopup<'a, 'a> {
    type State = TableState;

    fn render(mut self, area: Rect, buf: &mut ratatui::buffer::Buffer, state: &mut Self::State) {
        let formatted = self
            .formatted
            .get_or_insert_with(|| FormattedBindings::from_keybindings(self.keymap));

        let total_items = formatted.data.len();
        let max_action_width = formatted.max_action_width;
        let max_keys_width = formatted.max_keys_width;

        let theme = &self.theme;

        let mut rows = Vec::with_capacity(total_items);
        for (action, keys) in &formatted.data {
            let mut key_spans = Vec::with_capacity(keys.len() * 2);

            for (i, key) in keys.iter().enumerate() {
                if i > 0 {
                    key_spans.push(Span::styled(", ", theme.row()));
                }
                key_spans.push(Span::styled(key.as_str(), theme.config_doc()));
            }

            rows.push(
                Row::new(vec![
                    Cell::from(Text::from(action.as_str())),
                    Cell::from(Line::from(key_spans)),
                ])
                .style(theme.row()),
            );
        }

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(theme.border())
            .style(theme.base());

        let constraints = [
            Constraint::Length(max_action_width + 4),
            Constraint::Fill(1),
        ];

        let desired_width = max_action_width + max_keys_width + 10; // +10 for borders, padding, margin
        let width_constraint = Self::adaptive_width_constraint(area.width, desired_width);
        // Height: one row per item + header row + header bottom margin + 2 border rows.
        // Capped to the available height so it never overflows on small screens.
        let content_height = total_items as u16 + 4;
        let height_constraint =
            Constraint::Length(content_height.min(area.height.saturating_sub(2)));

        let area = util::center(area, width_constraint, height_constraint);

        Clear.render(area, buf);

        let inner_area = block.inner(area);

        // Clamp the scroll offset so the table always fills from the bottom when
        // the viewport grows (e.g. terminal resized after scrolling down).
        // Subtract 2 for the header row and its bottom margin.
        let visible_items = inner_area.height.saturating_sub(2) as usize;
        let max_offset = total_items.saturating_sub(visible_items);
        *state.offset_mut() = state.offset().min(max_offset);

        StatefulWidget::render(
            Table::new(rows, constraints)
                .block(block)
                .header(
                    Row::new(vec!["Action", "Keystroke"])
                        .style(
                            theme
                                .header()
                                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                        )
                        .bottom_margin(1),
                )
                .row_highlight_style(theme.list_highlight()),
            area,
            buf,
            state,
        );

        if total_items > visible_items {
            let scrollbar = Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None);

            let mut scrollbar_state = ScrollbarState::new(total_items)
                .position(state.selected().unwrap_or(0))
                .viewport_content_length(inner_area.height as usize);

            scrollbar.render(inner_area, buf, &mut scrollbar_state);
        }
    }
}
