use ratatui::prelude::*;
use ratatui::style::palette::tailwind;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Theme {
    #[default]
    Dark,
    Light,
    Gruvbox,
    Nord,
    Solarized,
    TokyoNight,
    Pitch,
}

impl std::str::FromStr for Theme {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "dark" => Ok(Self::Dark),
            "light" => Ok(Self::Light),
            "gruvbox dark" => Ok(Self::Gruvbox),
            "nord" => Ok(Self::Nord),
            "solarized" => Ok(Self::Solarized),
            "tokyo night" => Ok(Self::TokyoNight),
            "pitch" => Ok(Self::Pitch),
            _ => Err(format!("Unknown theme: {s}")),
        }
    }
}

// Named palette constants for themes that have their own well-known color systems.

mod gruvbox {
    use ratatui::style::Color;
    pub(crate) const BG0_HARD: Color = Color::Rgb(29, 32, 33); // #1d2021
    pub(crate) const BG1: Color = Color::Rgb(60, 56, 54); // #3c3836
    pub(crate) const BG3: Color = Color::Rgb(102, 92, 84); // #665c54
    pub(crate) const FG: Color = Color::Rgb(235, 219, 178); // #ebdbb2
    pub(crate) const RED: Color = Color::Rgb(204, 36, 29); // #cc241d
    pub(crate) const RED_DIM: Color = Color::Rgb(157, 0, 6);
    pub(crate) const AQUA: Color = Color::Rgb(104, 157, 106); // #689d6a
    pub(crate) const AQUA_DIM: Color = Color::Rgb(79, 121, 82);
    pub(crate) const BR_RED: Color = Color::Rgb(251, 73, 52); // #fb4934
    pub(crate) const BR_YELLOW: Color = Color::Rgb(250, 189, 47); // #fabd2f
    pub(crate) const BR_BLUE: Color = Color::Rgb(131, 165, 152); // #83a598
    pub(crate) const BR_AQUA: Color = Color::Rgb(142, 192, 124); // #8ec07c
}

mod nord {
    use ratatui::style::Color;
    // Polar Night
    pub(crate) const POLAR0: Color = Color::Rgb(46, 52, 64); // #2e3440
    pub(crate) const POLAR1: Color = Color::Rgb(59, 66, 82); // #3b4252
    pub(crate) const POLAR2: Color = Color::Rgb(67, 76, 94); // #434c5e
    pub(crate) const POLAR3: Color = Color::Rgb(76, 86, 106); // #4c566a
    // Snow Storm
    pub(crate) const SNOW0: Color = Color::Rgb(216, 222, 233); // #d8dee9
    pub(crate) const SNOW2: Color = Color::Rgb(236, 239, 244); // #eceff4
    // Frost
    pub(crate) const FROST0: Color = Color::Rgb(143, 188, 187); // #8fbcbb - teal
    pub(crate) const FROST1: Color = Color::Rgb(136, 192, 208); // #88c0d0 - light blue
    pub(crate) const FROST2: Color = Color::Rgb(129, 161, 193); // #81a1c1 - steel blue
    pub(crate) const FROST3: Color = Color::Rgb(94, 129, 172); // #5e81ac - deep blue
    // Aurora
    pub(crate) const RED: Color = Color::Rgb(191, 97, 106); // #bf616a
    pub(crate) const YELLOW: Color = Color::Rgb(235, 203, 139); // #ebcb8b
    pub(crate) const GREEN_DARK: Color = Color::Rgb(42, 62, 34);
    pub(crate) const GREEN_DARK_ALT: Color = Color::Rgb(30, 46, 24);
}

mod solarized {
    use ratatui::style::Color;
    pub(crate) const BASE03: Color = Color::Rgb(0, 43, 54); // #002b36
    pub(crate) const BASE02: Color = Color::Rgb(7, 54, 66); // #073642
    pub(crate) const BASE01: Color = Color::Rgb(88, 110, 117); // #586e75
    pub(crate) const BASE0: Color = Color::Rgb(131, 148, 150); // #839496
    pub(crate) const BASE1: Color = Color::Rgb(147, 161, 161); // #93a1a1
    pub(crate) const YELLOW: Color = Color::Rgb(181, 137, 0); // #b58900
    pub(crate) const RED: Color = Color::Rgb(220, 50, 47); // #dc322f
    pub(crate) const RED_DIM: Color = Color::Rgb(176, 40, 37);
    pub(crate) const BLUE: Color = Color::Rgb(38, 139, 210); // #268bd2
    pub(crate) const CYAN: Color = Color::Rgb(42, 161, 152); // #2aa198
    pub(crate) const GREEN: Color = Color::Rgb(133, 153, 0); // #859900
}

mod tokyo_night {
    use ratatui::style::Color;
    // Storm variant - the more visibly blue of the two main Tokyo Night variants
    pub(crate) const BG: Color = Color::Rgb(36, 40, 59); // #24283b  main bg
    pub(crate) const BG_DARK: Color = Color::Rgb(31, 35, 53); // #1f2335  log pane / darker areas
    pub(crate) const BG_HEADER: Color = Color::Rgb(41, 46, 66); // #292e42  header row
    pub(crate) const SELECTION: Color = Color::Rgb(40, 52, 87); // #283457  alt rows / selection bg
    pub(crate) const COMMENT: Color = Color::Rgb(86, 95, 137); // #565f89  borders / dim text
    pub(crate) const FG: Color = Color::Rgb(192, 202, 245); // #c0caf5  body text (blue-tinted)
    pub(crate) const RED: Color = Color::Rgb(247, 118, 142); // #f7768e
    pub(crate) const YELLOW: Color = Color::Rgb(224, 175, 104); // #e0af68
    pub(crate) const GREEN_DARK: Color = Color::Rgb(28, 52, 18);
    pub(crate) const GREEN_DARK_ALT: Color = Color::Rgb(20, 38, 13);
    pub(crate) const BLUE: Color = Color::Rgb(122, 162, 247); // #7aa2f7
    pub(crate) const CYAN: Color = Color::Rgb(125, 207, 255); // #7dcfff
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TableColors {
    // Table
    pub(crate) buffer_bg: Color,
    pub(crate) header_bg: Color,
    header_fg: Color,
    row_fg: Color,
    selected_row_fg: Color,
    selected_col_fg: Color,
    selected_cell_fg: Color,
    pub(crate) normal_row_color: Color,
    pub(crate) normal_row_color_alt: Color,
    pub(crate) offline_row_color: Color,
    pub(crate) offline_row_color_alt: Color,
    pub(crate) newly_updated_row_color: Color,
    pub(crate) newly_updated_row_color_alt: Color,
    recently_copied_cell_color: Color,
    // Shared UI
    border_fg: Color,
    title_fg: Color,
    gauge_fg: Color,
    /// Color for popup border/title/hostnames. Usually equals `gauge_fg`; may differ when the
    /// bar color is intentionally dim (e.g. Pitch) and needs a brighter accent for readability.
    gauge_accent_fg: Color,
    /// Explicit label fg override. `None` = let ratatui invert for contrast automatically.
    gauge_label_fg: Option<Color>,
    // Log pane
    log_pane_bg: Color,
    log_err_fg: Color,
    log_warn_fg: Color,
    log_info_fg: Color,
    log_debug_fg: Color,
    log_trace_fg: Color,
    // Config window
    config_highlight_bg: Color,
    config_doc_fg: Color,
    // Success / affirmative accent (always a green, used for e.g. error box border)
    success_fg: Color,
}

impl TableColors {
    /// Background for most pane areas.
    pub(crate) fn base(&self) -> Style {
        Style::new().bg(self.buffer_bg)
    }
    /// Border foreground.
    pub(crate) fn border(&self) -> Style {
        Style::new().fg(self.border_fg)
    }
    /// Text for titles and labels.
    pub(crate) fn title(&self) -> Style {
        Style::new().fg(self.title_fg)
    }
    /// Table / list header row.
    pub(crate) fn header(&self) -> Style {
        Style::new().fg(self.header_fg).bg(self.header_bg)
    }
    /// Default foreground for data rows and list items.
    pub(crate) fn row(&self) -> Style {
        Style::new().fg(self.row_fg)
    }
    /// Gauge filled portion.
    pub(crate) fn gauge_fill(&self) -> Style {
        Style::new().fg(self.gauge_fg).bg(self.buffer_bg)
    }
    /// Gauge background / label area.
    pub(crate) fn gauge_bg(&self) -> Style {
        Style::new().bg(self.buffer_bg).fg(self.title_fg)
    }
    /// Accent color for the stats popup (border/title/hostnames).
    pub(crate) fn gauge_accent(&self) -> Style {
        Style::new().fg(self.gauge_accent_fg)
    }
    /// Progress bar label style. No explicit fg for most themes (ratatui inverts automatically);
    /// explicit white for Pitch where the dark bar would otherwise hide the text.
    pub(crate) fn gauge_label(&self) -> Style {
        match self.gauge_label_fg {
            Some(c) => Style::new().fg(c),
            None => Style::new(),
        }
    }
    /// Highlighted / selected list item.
    pub(crate) fn list_highlight(&self) -> Style {
        Style::new().bg(self.config_highlight_bg).fg(self.row_fg)
    }
    /// Inline documentation text in the config window.
    pub(crate) fn config_doc(&self) -> Style {
        Style::new().fg(self.config_doc_fg)
    }
    /// Success / affirmative accent - always a green regardless of theme.
    pub(crate) fn success(&self) -> Style {
        Style::new().fg(self.success_fg)
    }
    /// Border for text input overlays (search box, inline config editor).
    pub(crate) fn text_input_border(&self) -> Style {
        Style::new().fg(self.config_doc_fg)
    }
    /// Text content inside text input overlays (includes background fill).
    /// Uses the theme's warm accent (amber/yellow) so edited values stand out
    /// from the surrounding list text.
    pub(crate) fn text_input_text(&self) -> Style {
        Style::new().fg(self.log_warn_fg).bg(self.buffer_bg)
    }
    /// Log pane background.
    pub(crate) fn log_bg(&self) -> Style {
        Style::new().bg(self.log_pane_bg)
    }
    pub(crate) fn log_err(&self) -> Style {
        Style::new().fg(self.log_err_fg)
    }
    pub(crate) fn log_warn(&self) -> Style {
        Style::new().fg(self.log_warn_fg)
    }
    pub(crate) fn log_info(&self) -> Style {
        Style::new().fg(self.log_info_fg)
    }
    pub(crate) fn log_debug(&self) -> Style {
        Style::new().fg(self.log_debug_fg)
    }
    pub(crate) fn log_trace(&self) -> Style {
        Style::new().fg(self.log_trace_fg)
    }
}

impl Default for TableColors {
    fn default() -> Self {
        Self::dark()
    }
}

impl From<Theme> for TableColors {
    fn from(theme: Theme) -> Self {
        match theme {
            Theme::Dark => Self::dark(),
            Theme::Light => Self::light(),
            Theme::Gruvbox => Self::gruvbox(),
            Theme::Nord => Self::nord(),
            Theme::Solarized => Self::solarized(),
            Theme::TokyoNight => Self::tokyo_night(),
            Theme::Pitch => Self::pitch(),
        }
    }
}

impl TableColors {
    /// Dark - true black background, neutral greys, emerald accents.
    fn dark() -> Self {
        Self {
            buffer_bg: Color::Black,
            header_bg: tailwind::NEUTRAL.c900,
            header_fg: tailwind::NEUTRAL.c200,
            row_fg: tailwind::NEUTRAL.c300,
            selected_row_fg: tailwind::BLUE.c400,
            selected_col_fg: tailwind::BLUE.c400,
            selected_cell_fg: tailwind::BLUE.c300,
            normal_row_color: Color::Black,
            normal_row_color_alt: tailwind::NEUTRAL.c950,
            offline_row_color: tailwind::RED.c900,
            offline_row_color_alt: tailwind::RED.c950,
            newly_updated_row_color: tailwind::GREEN.c900,
            newly_updated_row_color_alt: tailwind::GREEN.c950,
            recently_copied_cell_color: Color::White,
            border_fg: tailwind::NEUTRAL.c600,
            title_fg: tailwind::NEUTRAL.c200,
            gauge_fg: tailwind::BLUE.c400,
            gauge_accent_fg: tailwind::BLUE.c400,
            gauge_label_fg: None,
            log_pane_bg: Color::Black,
            log_err_fg: tailwind::RED.c400,
            log_warn_fg: tailwind::AMBER.c400,
            log_info_fg: tailwind::NEUTRAL.c300,
            log_debug_fg: tailwind::CYAN.c400,
            log_trace_fg: tailwind::NEUTRAL.c500,
            config_highlight_bg: tailwind::NEUTRAL.c800,
            config_doc_fg: Color::Rgb(142, 192, 124), // warm yellow-green, no blue cast
            success_fg: Color::Rgb(142, 192, 124),    // warm yellow-green, no blue cast
        }
    }

    /// Light - slate backgrounds, dark text, blue/green accents.
    fn light() -> Self {
        Self {
            buffer_bg: tailwind::SLATE.c50,
            header_bg: tailwind::SLATE.c300,
            header_fg: tailwind::SLATE.c900,
            row_fg: tailwind::SLATE.c900,
            selected_row_fg: tailwind::BLUE.c400,
            selected_col_fg: tailwind::BLUE.c400,
            selected_cell_fg: tailwind::BLUE.c300,
            normal_row_color: tailwind::SLATE.c50,
            normal_row_color_alt: tailwind::SLATE.c100,
            offline_row_color: tailwind::RED.c100,
            offline_row_color_alt: tailwind::RED.c200,
            newly_updated_row_color: tailwind::GREEN.c100,
            newly_updated_row_color_alt: tailwind::GREEN.c200,
            recently_copied_cell_color: Color::Black,
            border_fg: tailwind::SLATE.c500,
            title_fg: tailwind::SLATE.c900,
            gauge_fg: tailwind::BLUE.c500,
            gauge_accent_fg: tailwind::BLUE.c500,
            gauge_label_fg: None,
            log_pane_bg: tailwind::SLATE.c50,
            log_err_fg: tailwind::RED.c700,
            log_warn_fg: tailwind::AMBER.c700,
            log_info_fg: tailwind::SLATE.c900,
            log_debug_fg: tailwind::TEAL.c700,
            log_trace_fg: tailwind::SLATE.c400,
            config_highlight_bg: tailwind::SLATE.c200,
            config_doc_fg: tailwind::GREEN.c700,
            success_fg: tailwind::GREEN.c700,
        }
    }

    /// Gruvbox dark.
    fn gruvbox() -> Self {
        use gruvbox::*;
        Self {
            buffer_bg: BG0_HARD,
            header_bg: BG1,
            header_fg: FG,
            row_fg: FG,
            selected_row_fg: Color::Rgb(168, 153, 132),
            selected_col_fg: BR_YELLOW,
            selected_cell_fg: BR_YELLOW,
            normal_row_color: BG0_HARD,
            normal_row_color_alt: BG1,
            offline_row_color: RED,
            offline_row_color_alt: RED_DIM,
            newly_updated_row_color: AQUA,
            newly_updated_row_color_alt: AQUA_DIM,
            recently_copied_cell_color: FG,
            border_fg: BG3,
            title_fg: FG,
            gauge_fg: BR_BLUE,
            gauge_accent_fg: BR_BLUE,
            gauge_label_fg: None,
            log_pane_bg: BG0_HARD,
            log_err_fg: BR_RED,
            log_warn_fg: BR_YELLOW,
            log_info_fg: FG,
            log_debug_fg: BR_BLUE,
            log_trace_fg: BG3,
            config_highlight_bg: BG1,
            config_doc_fg: BR_AQUA,
            success_fg: BR_AQUA,
        }
    }

    /// Nord.
    fn nord() -> Self {
        use nord::*;
        Self {
            buffer_bg: POLAR0,
            header_bg: POLAR2,
            header_fg: SNOW2,
            row_fg: SNOW0,
            selected_row_fg: FROST1,
            selected_col_fg: FROST2,
            selected_cell_fg: FROST0,
            normal_row_color: POLAR0,
            normal_row_color_alt: POLAR1,
            offline_row_color: tailwind::RED.c800,
            offline_row_color_alt: tailwind::RED.c900,
            newly_updated_row_color: GREEN_DARK,
            newly_updated_row_color_alt: GREEN_DARK_ALT,
            recently_copied_cell_color: SNOW2,
            border_fg: POLAR3,
            title_fg: SNOW2,
            gauge_fg: FROST1,
            gauge_accent_fg: FROST1,
            gauge_label_fg: None,
            log_pane_bg: POLAR0,
            log_err_fg: RED,
            log_warn_fg: YELLOW,
            log_info_fg: SNOW0,
            log_debug_fg: FROST2,
            log_trace_fg: POLAR3,
            config_highlight_bg: POLAR2,
            config_doc_fg: FROST3,
            success_fg: Color::Rgb(163, 190, 140), // #a3be8c Aurora green
        }
    }

    /// Solarized dark.
    fn solarized() -> Self {
        use solarized::*;
        Self {
            buffer_bg: BASE03,
            header_bg: BASE02,
            header_fg: BASE1,
            row_fg: BASE0,
            selected_row_fg: BLUE,
            selected_col_fg: BLUE,
            selected_cell_fg: CYAN,
            normal_row_color: BASE03,
            normal_row_color_alt: BASE02,
            offline_row_color: RED,
            offline_row_color_alt: RED_DIM,
            newly_updated_row_color: Color::Rgb(40, 65, 0),
            newly_updated_row_color_alt: Color::Rgb(28, 46, 0),
            recently_copied_cell_color: BASE1,
            border_fg: BASE01,
            title_fg: BASE1,
            gauge_fg: CYAN,
            gauge_accent_fg: CYAN,
            gauge_label_fg: None,
            log_pane_bg: BASE03,
            log_err_fg: RED,
            log_warn_fg: YELLOW,
            log_info_fg: BASE0,
            log_debug_fg: CYAN,
            log_trace_fg: BASE01,
            config_highlight_bg: BASE02,
            config_doc_fg: GREEN,
            success_fg: GREEN,
        }
    }

    /// Tokyo Night (Storm) - clearly blue background, blue-tinted foreground, bright blue/cyan accents.
    fn tokyo_night() -> Self {
        use tokyo_night::*;
        Self {
            buffer_bg: BG,
            header_bg: BG_HEADER,
            header_fg: FG,
            row_fg: FG,
            selected_row_fg: BLUE,
            selected_col_fg: BLUE,
            selected_cell_fg: CYAN,
            normal_row_color: BG,
            normal_row_color_alt: SELECTION,
            offline_row_color: tailwind::RED.c700,
            offline_row_color_alt: tailwind::RED.c800,
            newly_updated_row_color: GREEN_DARK,
            newly_updated_row_color_alt: GREEN_DARK_ALT,
            recently_copied_cell_color: FG,
            border_fg: COMMENT,
            title_fg: FG,
            gauge_fg: BLUE,
            gauge_accent_fg: BLUE,
            gauge_label_fg: None,
            log_pane_bg: BG_DARK,
            log_err_fg: RED,
            log_warn_fg: YELLOW,
            log_info_fg: FG,
            log_debug_fg: CYAN,
            log_trace_fg: COMMENT,
            config_highlight_bg: SELECTION,
            config_doc_fg: CYAN,
            success_fg: Color::Rgb(158, 206, 106), // #9ece6a Tokyo Night green
        }
    }

    /// Pitch - true RGB black throughout (#000000), high contrast for OLED displays.
    fn pitch() -> Self {
        const BLACK: Color = Color::Rgb(0, 0, 0);
        Self {
            buffer_bg: BLACK,
            header_bg: BLACK,
            header_fg: Color::White,
            row_fg: tailwind::NEUTRAL.c100,
            selected_row_fg: Color::Rgb(100, 150, 190),
            selected_col_fg: Color::Rgb(100, 150, 190),
            selected_cell_fg: Color::Rgb(120, 170, 210),
            normal_row_color: BLACK,
            normal_row_color_alt: Color::Rgb(18, 18, 18),
            offline_row_color: tailwind::RED.c900,
            offline_row_color_alt: tailwind::RED.c950,
            newly_updated_row_color: tailwind::GREEN.c800,
            newly_updated_row_color_alt: tailwind::GREEN.c900,
            recently_copied_cell_color: Color::White,
            border_fg: tailwind::NEUTRAL.c400,
            title_fg: Color::White,
            gauge_fg: Color::Rgb(30, 70, 120), // dim steel blue bar - intentionally muted on OLED
            gauge_accent_fg: Color::Rgb(80, 140, 200), // brighter steel blue for popup readability
            gauge_label_fg: Some(Color::White),
            log_pane_bg: BLACK,
            log_err_fg: tailwind::RED.c300,
            log_warn_fg: tailwind::AMBER.c300,
            log_info_fg: tailwind::NEUTRAL.c100,
            log_debug_fg: tailwind::CYAN.c300,
            log_trace_fg: tailwind::NEUTRAL.c500,
            config_highlight_bg: Color::Rgb(38, 38, 38), // visible above both row (0,0,0) and alt-row (18,18,18)
            config_doc_fg: Color::Rgb(166, 218, 149),    // brighter warm green for pure-black bg
            success_fg: Color::Rgb(166, 218, 149),       // brighter warm green for pure-black bg
        }
    }
}

impl TableColors {
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

    /// Style for a regular data row with the given background.
    pub(crate) fn row_with_bg(&self, bg: Color) -> Style {
        Style::new().fg(self.row_fg).bg(bg)
    }

    /// Style for the selected row.
    pub(crate) fn selected_row(&self) -> Style {
        Style::default()
            .fg(self.selected_row_fg)
            .add_modifier(Modifier::REVERSED)
    }

    /// Style for the selected column.
    pub(crate) fn selected_col(&self) -> Style {
        Style::default().fg(self.selected_col_fg)
    }

    /// Style for the selected cell.
    pub(crate) fn selected_cell(&self) -> Style {
        Style::default()
            .fg(self.selected_cell_fg)
            .add_modifier(Modifier::REVERSED)
    }
}
