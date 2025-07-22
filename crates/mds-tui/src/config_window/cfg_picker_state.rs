use std::num::NonZeroU16;

use mds_config::{
    config_type::ConfigType,
    scan::{
        IoThreads,
        io_threads::{MAX_IO_THREADS, MIN_LOW_TIER_THREADS},
    },
    shared_config::SharedConfig,
};
use mds_log::LogLevel;
use ratatui::{
    style::{Color, Style},
    widgets::{Block, Borders, ListState},
};
use tui_textarea::TextArea;

use crate::error_box::ErrorBox;

#[derive(Clone)]
pub(crate) struct CfgPickerState<'t> {
    pub(super) cfg: SharedConfig,
    pub(super) txt_edit: Option<TextArea<'t>>,
    pub(super) state: ListState,
}

impl<'t> CfgPickerState<'t> {
    pub fn new(cfg: SharedConfig) -> Self {
        let mut state = ListState::default();
        state.select(Some(0));
        Self {
            cfg,
            txt_edit: None,
            state,
        }
    }

    pub fn handle_selected_item(
        &mut self,
        get_items: impl FnOnce(&mut mds_config::AppConfig) -> Vec<ConfigType<'_>>,
    ) -> Result<(), ErrorBox> {
        let Some(selected) = self.state.selected() else {
            return Ok(());
        };

        self.cfg.modify(|cfg| {
            let mut items = get_items(cfg);
            if let Some(item) = items.get_mut(selected) {
                CfgPickerState::handle_confirm_action(&mut self.txt_edit, item)?;
            }
            Ok(())
        })
    }

    /// Enter/spacebar ...
    pub(crate) fn handle_confirm_action(
        txt_edit: &mut Option<TextArea<'_>>,
        item: &mut ConfigType<'_>,
    ) -> Result<(), ErrorBox> {
        let value_str = item.value_str();
        match item {
            ConfigType::Toggle { val, .. } => **val = !**val,

            ConfigType::NumberNonZeroU16 { val, .. } => {
                if let Some(txt) = edit_or_enter_mode(txt_edit, &value_str) {
                    let Ok(num) = txt.parse::<u16>() else {
                        return Err("Could not parse as u16".into());
                    };
                    let Some(new_val) = NonZeroU16::new(num) else {
                        return Err(format!("Expected Non-zero u16, got '{num}'").into());
                    };
                    **val = new_val;
                }
            }

            ConfigType::ScanIoThreads { val, .. } => {
                if let Some(txt) = edit_or_enter_mode(txt_edit, &value_str) {
                    let new_val = if txt.eq_ignore_ascii_case("dynamic") {
                        IoThreads::Dynamic
                    } else {
                        let err_msg = format!(
                            "Valid values are {MIN_LOW_TIER_THREADS}-{MAX_IO_THREADS} or 'dynamic'"
                        );
                        let Ok(num) = txt.parse::<u16>() else {
                            return Err(err_msg.into());
                        };
                        if !IoThreads::valid_value(num as usize) {
                            return Err(err_msg.into());
                        }
                        IoThreads::Fixed(num)
                    };
                    **val = new_val;
                }
            }

            ConfigType::LogLevelString { val, .. } => {
                if let Some(txt) = edit_or_enter_mode(txt_edit, &value_str) {
                    txt.parse::<LogLevel>().map_err(|e| e.to_string())?;
                    **val = txt;
                }
            }

            ConfigType::Numberu32 { val, .. } => {
                if let Some(txt) = edit_or_enter_mode(txt_edit, &value_str) {
                    let Ok(new_val) = txt.parse::<u32>() else {
                        return Err(format!("Could not parse '{txt}' as u32").into());
                    };
                    **val = new_val;
                }
            }

            ConfigType::NumberList { val, .. } => {
                if let Some(txt) = edit_or_enter_mode(txt_edit, &value_str) {
                    let mut new_val = vec![];
                    for num in txt.split(',') {
                        if let Ok(n) = num.trim_ascii().parse::<u16>() {
                            if !new_val.contains(&n) {
                                new_val.push(n);
                            }
                        }
                    }
                    **val = Some(new_val);
                }
            }

            ConfigType::RegexStringList { val, .. } => {
                if let Some(txt) = edit_or_enter_mode(txt_edit, &value_str) {
                    let mut new_val = vec![];
                    for pattern in txt.split(',') {
                        let pat = pattern.trim_ascii().to_owned();
                        if !new_val.contains(&pat) {
                            new_val.push(pat);
                        }
                    }

                    // Validate all regexes
                    for new_pattern in &new_val {
                        if let Err(e) = regex::Regex::new(new_pattern) {
                            return Err(
                                format!("Invalid Regex pattern '{new_pattern}'\n{e}").into()
                            );
                        }
                    }

                    **val = new_val;
                }
            }
        }
        Ok(())
    }
}

fn edit_or_enter_mode(txt_edit: &mut Option<TextArea<'_>>, value_str: &str) -> Option<String> {
    if let Some(txt) = txt_edit
        .as_mut()
        .and_then(|e| e.lines().first().map(|s| s.trim_ascii().to_string()))
    {
        Some(txt)
    } else {
        let mut text_area = build_text_edit_area();
        text_area.insert_str(value_str);
        *txt_edit = Some(text_area);
        None
    }
}

fn build_text_edit_area<'a>() -> TextArea<'a> {
    let mut text_area = tui_textarea::TextArea::default();
    text_area.set_block(
        Block::default()
            .borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM)
            .border_style(Style::default().fg(Color::LightBlue)),
    );

    text_area.set_style(Style::default().fg(Color::Yellow));
    text_area.set_placeholder_style(Style::default());
    text_area
}
