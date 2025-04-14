use crate::ip_info::IpInfo;
use ratatui::prelude::*;
use style::palette::tailwind;
use unicode_width::UnicodeWidthStr;

pub(super) const ITEM_HEIGHT: usize = 1;

const PALETTES: [tailwind::Palette; 4] = [
    tailwind::BLUE,
    tailwind::EMERALD,
    tailwind::INDIGO,
    tailwind::RED,
];

pub(super) struct TableColors {
    pub(super) buffer_bg: Color,
    pub(super) header_bg: Color,
    pub(super) header_fg: Color,
    pub(super) row_fg: Color,
    pub(super) selected_row_style_fg: Color,
    pub(super) selected_column_style_fg: Color,
    pub(super) selected_cell_style_fg: Color,
    pub(super) normal_row_color: Color,
    pub(super) alt_row_color: Color,
    pub(super) footer_border_color: Color,
}

impl TableColors {
    pub(super) const fn default() -> Self {
        Self::new(&PALETTES[0])
    }

    const fn new(color: &tailwind::Palette) -> Self {
        Self {
            buffer_bg: tailwind::SLATE.c950,
            header_bg: color.c900,
            header_fg: tailwind::SLATE.c200,
            row_fg: tailwind::SLATE.c200,
            selected_row_style_fg: color.c400,
            selected_column_style_fg: color.c400,
            selected_cell_style_fg: color.c600,
            normal_row_color: tailwind::SLATE.c950,
            alt_row_color: tailwind::SLATE.c900,
            footer_border_color: color.c400,
        }
    }
}

pub(crate) fn constraint_len_calculator(items: &[&IpInfo]) -> (u16, u16, u16) {
    let ip_len = items
        .iter()
        .map(|m| m.ip().to_string().width())
        .max()
        .unwrap_or(0);

    let hostname_len = items
        .iter()
        .map(|m| m.max_name_unicode_width())
        .max()
        .unwrap_or(0);

    let packets_count_len = items
        .iter()
        .map(|m| m.seen_count().to_string().width())
        .max()
        .unwrap_or(0);

    #[allow(clippy::cast_possible_truncation)]
    (ip_len as u16, hostname_len as u16, packets_count_len as u16)
}
