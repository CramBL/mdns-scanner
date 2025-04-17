pub(crate) mod colors;
pub(crate) mod util;

use std::cmp;

use colors::TableColors;
use ratatui::{
    Frame,
    layout::{Constraint, Margin, Rect},
    style::{Modifier, Style, Stylize},
    symbols,
    text::Text,
    widgets::{
        Block, Cell, HighlightSpacing, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table,
        TableState,
    },
};

use crate::ip_info::IpInfo;

pub(crate) struct TablePane {
    pub(crate) longest_item_lens: (u16, u16, u16), // order is (IP, name, seen count)
    colors: TableColors,
    state: TableState,
    scroll_state: ScrollbarState,
}

impl Default for TablePane {
    fn default() -> Self {
        Self {
            longest_item_lens: (10, 10, 10),
            colors: TableColors::default(),
            state: TableState::default().with_selected(0),
            scroll_state: ScrollbarState::new(0),
        }
    }
}

// Public
impl TablePane {
    pub fn next_column(&mut self) {
        self.state.select_next_column();
    }

    pub fn previous_column(&mut self) {
        self.state.select_previous_column();
    }

    pub fn next_row(&mut self, table_len: usize) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= table_len - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i * Self::ITEM_HEIGHT);
    }

    pub fn previous_row(&mut self, table_len: usize) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    table_len - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i * Self::ITEM_HEIGHT);
    }

    pub(super) fn render(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        ip_info: &[&IpInfo],
        in_focus: bool,
    ) {
        self.longest_item_lens = util::constraint_len_calculator(ip_info);

        let header = Self::header(self.header_style());
        let rows = Self::rows(&self.colors, ip_info);
        let table = Table::new(rows, self.table_width())
            .header(header)
            .row_highlight_style(self.selected_row_style())
            .column_highlight_style(self.selected_col_style())
            .cell_highlight_style(self.selected_cell_style())
            .highlight_symbol(Self::highlight_symbol())
            .bg(self.colors.buffer_bg)
            .highlight_spacing(HighlightSpacing::Always);

        let block_border = if in_focus {
            symbols::border::PLAIN
        } else {
            symbols::border::EMPTY
        };

        let table_block = Block::bordered()
            .title(self.pane_title(ip_info.len() as u16))
            .border_set(block_border);
        let table: Table<'_> = table.block(table_block);

        frame.render_stateful_widget(table, area, &mut self.state);
        self.render_scollbar(frame, area, ip_info.len());
    }
}

// Private
impl TablePane {
    const ITEM_HEIGHT: usize = 1;
    const HEADER: [&str; 3] = ["IP", "Name(s)", "Packets"];

    const TITLE_SUFFIX: &str = " IPs discovered";

    // Used to make the highlight symbol that appears to the left of the selected row
    const SELECTED_BAR: &str = " █ ";

    fn render_scollbar(&mut self, frame: &mut Frame, area: Rect, table_len: usize) {
        let mut state = self.scroll_state.content_length(table_len);
        frame.render_stateful_widget(
            Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None),
            area.inner(Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut state,
        );
    }

    fn pane_title(&self, item_count: u16) -> String {
        let num = item_count.to_string();
        let mut title = String::with_capacity(num.len() + Self::TITLE_SUFFIX.len());
        title.push_str(&num);
        title.push_str(Self::TITLE_SUFFIX);
        title
    }

    fn highlight_symbol<'a>() -> Text<'a> {
        Text::from(vec![
            "".into(),
            Self::SELECTED_BAR.into(),
            Self::SELECTED_BAR.into(),
            "".into(),
        ])
    }

    fn header<'a>(style: Style) -> Row<'a> {
        Self::HEADER
            .into_iter()
            .map(Cell::from)
            .collect::<Row>()
            .style(style)
            .height(1)
    }

    fn rows<'a>(colors: &TableColors, ip_info: &[&IpInfo]) -> impl Iterator<Item = Row<'a>> {
        let rows = ip_info.iter().enumerate().map(|(i, ip_info)| {
            let color = match i % 2 {
                0 => colors.normal_row_color,
                _ => colors.alt_row_color,
            };
            let hostname_count = ip_info.names().len() as u16;
            let height = cmp::max(2, hostname_count + 1);
            let row_style = Style::new().fg(colors.row_fg).bg(color);
            let item = ip_info.ref_array();

            item.into_iter()
                .map(|content| Cell::from(Text::from(format!("\n{content}\n"))))
                .collect::<Row>()
                .style(row_style)
                .height(height)
        });
        rows
    }

    fn selected_row_style(&self) -> Style {
        Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(self.colors.selected_row_style_fg)
    }

    fn selected_col_style(&self) -> Style {
        Style::default().fg(self.colors.selected_column_style_fg)
    }

    fn selected_cell_style(&self) -> Style {
        Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(self.colors.selected_cell_style_fg)
    }

    fn header_style(&self) -> Style {
        Style::default()
            .fg(self.colors.header_fg)
            .bg(self.colors.header_bg)
    }

    fn table_width(&self) -> [Constraint; 3] {
        [
            // + 1 is for padding.
            Constraint::Length(self.longest_item_lens.0 + 1),
            Constraint::Min(self.longest_item_lens.1 + 1),
            Constraint::Min(self.longest_item_lens.2),
        ]
    }
}
