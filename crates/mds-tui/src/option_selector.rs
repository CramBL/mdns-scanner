use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::{Line, Span},
    widgets::{
        Block, BorderType, Clear, HighlightSpacing, List, ListItem, ListState, StatefulWidget,
        Widget,
    },
};

pub(crate) use mds_config::config_type::SelectorSideEffect;

use crate::table_pane::TableColors;

/// A generic inline option-picker overlay, rendered next to the selected list
/// item in the config window. Supports wrapping navigation and immediate
/// value-preview callbacks handled by the caller.
#[derive(Clone)]
pub(crate) struct OptionSelector {
    /// The config key that owns this selector, used as the popup title.
    pub(crate) config_key: &'static str,
    /// Side effect to run whenever this selector's value is applied.
    pub(crate) side_effect: SelectorSideEffect,
    options: Box<[String]>,
    pub(crate) selected_idx: usize,
    /// Index that was active when the selector was opened - used for cancel.
    original_idx: usize,
    list_state: ListState,
}

impl OptionSelector {
    pub(crate) fn new(
        config_key: &'static str,
        options: &'static [&'static str],
        current: &str,
        side_effect: SelectorSideEffect,
    ) -> Self {
        let idx = options.iter().position(|&o| o == current).unwrap_or(0);
        let mut list_state = ListState::default();
        list_state.select(Some(idx));
        Self {
            config_key,
            side_effect,
            options: options
                .iter()
                .map(|&s| s.to_owned())
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            selected_idx: idx,
            original_idx: idx,
            list_state,
        }
    }

    pub(crate) fn current_value(&self) -> &str {
        self.options
            .get(self.selected_idx)
            .map_or("", String::as_str)
    }

    pub(crate) fn original_value(&self) -> &str {
        self.options
            .get(self.original_idx)
            .map_or("", String::as_str)
    }

    pub(crate) fn navigate_up(&mut self) {
        let n = self.options.len();
        if n == 0 {
            return;
        }
        self.selected_idx = if self.selected_idx == 0 {
            n - 1
        } else {
            self.selected_idx - 1
        };
        self.list_state.select(Some(self.selected_idx));
    }

    pub(crate) fn navigate_down(&mut self) {
        let n = self.options.len();
        if n == 0 {
            return;
        }
        self.selected_idx = (self.selected_idx + 1) % n;
        self.list_state.select(Some(self.selected_idx));
    }

    /// Render the selector popup anchored to `list_item_idx` within `area`.
    /// `x_offset` and `y_offset_from_item` are supplied by the caller so that
    /// all overlay positioning is controlled from a single place.
    pub(crate) fn render(
        &mut self,
        list_item_idx: usize,
        x_offset: u16,
        y_offset_from_item: u16,
        area: &Rect,
        buf: &mut Buffer,
        theme: &TableColors,
    ) {
        // Border (left + right) + highlight prefix "> " = 4; padding adds 2 more.
        const WIDTH_OVERHEAD: u16 = 6;
        const MIN_WIDTH: u16 = 10;
        // Border top + border bottom.
        const HEIGHT_BORDER: u16 = 2;
        // Minimum viable popup: border + at least one visible row.
        const MIN_USABLE_WIDTH: u16 = 6;
        const MIN_USABLE_HEIGHT: u16 = 3;
        // Keep the popup away from the bottom edge of the containing area.
        const BOTTOM_MARGIN: u16 = 4;

        let n = self.options.len();
        if n == 0 {
            return;
        }

        let max_opt_len = self.options.iter().map(|o| o.len()).max().unwrap_or(4);
        let width = (max_opt_len as u16 + WIDTH_OVERHEAD).max(MIN_WIDTH);
        let height = (n as u16 + HEIGHT_BORDER).min(area.height.saturating_sub(BOTTOM_MARGIN));

        let y_offset = list_item_idx as u16 + y_offset_from_item;

        let available_width = area.width.saturating_sub(x_offset);
        let available_height = area.height.saturating_sub(y_offset);

        if available_width < MIN_USABLE_WIDTH || available_height < MIN_USABLE_HEIGHT {
            return;
        }

        let pos = area.as_position();
        let rect = Rect::new(
            pos.x + x_offset,
            pos.y + y_offset,
            width.min(available_width),
            height.min(available_height),
        );

        let items: Vec<ListItem> = self
            .options
            .iter()
            .map(|o| ListItem::new(Line::from(Span::raw(o.as_str()))))
            .collect();

        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .title(Span::styled(self.config_key, theme.config_doc()))
            .border_style(theme.config_doc())
            .style(theme.base());

        let list = List::new(items)
            .block(block)
            .style(theme.row())
            .highlight_style(theme.list_highlight())
            .highlight_symbol("> ")
            .highlight_spacing(HighlightSpacing::Always);

        Clear.render(rect, buf);
        StatefulWidget::render(list, rect, buf, &mut self.list_state);
    }
}
