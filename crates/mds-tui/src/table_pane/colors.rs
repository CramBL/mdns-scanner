use ratatui::prelude::*;
use style::palette::tailwind;

const PALETTES: [tailwind::Palette; 4] = [
    tailwind::BLUE,
    tailwind::EMERALD,
    tailwind::INDIGO,
    tailwind::RED,
];

pub(crate) struct TableColors {
    pub(crate) buffer_bg: Color,
    pub(crate) header_bg: Color,
    pub(crate) header_fg: Color,
    pub(crate) row_fg: Color,
    pub(crate) selected_row_style_fg: Color,
    pub(crate) selected_column_style_fg: Color,
    pub(crate) selected_cell_style_fg: Color,
    pub(crate) normal_row_color: Color,
    pub(crate) normal_row_color_alt: Color,
    pub(crate) offline_row_color: Color,
    pub(crate) offline_row_color_alt: Color,
    pub(crate) newly_updated_row_color: Color,
    pub(crate) newly_updated_row_color_alt: Color,
    pub(crate) footer_border_color: Color,
    recently_copied_cell_color: Color,
}

impl TableColors {
    pub(crate) const fn default() -> Self {
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
            normal_row_color_alt: tailwind::SLATE.c900,
            offline_row_color: tailwind::RED.c900,
            offline_row_color_alt: tailwind::RED.c950,
            newly_updated_row_color: tailwind::GREEN.c900,
            newly_updated_row_color_alt: tailwind::GREEN.c950,
            recently_copied_cell_color: tailwind::WHITE,
            footer_border_color: color.c400,
        }
    }

    pub(crate) const fn normal_row_color(&self, idx: usize) -> Color {
        match idx % 2 {
            0 => self.normal_row_color,
            _ => self.normal_row_color_alt,
        }
    }
    pub(crate) const fn offline_row_color(&self, idx: usize) -> Color {
        match idx % 2 {
            0 => self.offline_row_color,
            _ => self.offline_row_color_alt,
        }
    }
    pub(crate) const fn newly_updated_row_color(&self, idx: usize) -> Color {
        match idx % 2 {
            0 => self.newly_updated_row_color,
            _ => self.newly_updated_row_color_alt,
        }
    }

    pub(crate) const fn recently_copied_cell_color(&self) -> Color {
        self.recently_copied_cell_color
    }
}
