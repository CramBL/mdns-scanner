use std::{num::NonZeroU16, sync::Arc};

use mds_config::{AppConfig, config_type::ConfigType};
use parking_lot::RwLock;
use ratatui::{
    style::{Color, Style},
    widgets::{Block, Borders, ListState},
};
use tui_textarea::TextArea;

use crate::error_box::ErrorBox;

type ArcLockCfg = Arc<RwLock<AppConfig>>;

#[derive(Clone)]
pub(crate) struct CfgPickerState<'t> {
    pub(super) cfg: ArcLockCfg,
    pub(super) txt_edit: Option<TextArea<'t>>,
    pub(super) state: ListState,
}

impl<'t> CfgPickerState<'t> {
    pub fn new(cfg: ArcLockCfg) -> Self {
        let mut state = ListState::default();
        state.select(Some(0));
        Self {
            cfg,
            txt_edit: None,
            state,
        }
    }

    pub(super) fn selected(&self) -> Option<usize> {
        self.state.selected()
    }

    /// Enter/spacebar ...
    pub(crate) fn handle_confirm_action(
        txt_edit: &mut Option<TextArea<'_>>,
        item: &mut ConfigType<'_>,
    ) -> Result<(), ErrorBox> {
        match item {
            ConfigType::Toggle { val, .. } => **val = !**val,
            ConfigType::NumberNonZeroU16 { val, .. } => {
                if let Some(txt_edit) = txt_edit.as_mut() {
                    let txt = txt_edit
                        .lines()
                        .first()
                        .expect("unsound condition")
                        .trim_ascii();
                    let Ok(num) = txt.parse::<u16>() else {
                        return Err("Could not parse as u16".into());
                    };
                    let Some(new_val) = NonZeroU16::new(num) else {
                        return Err(format!("Expected Non-zero u16, got '{num}'").into());
                    };
                    **val = new_val;
                } else {
                    let mut text_area = build_text_edit_area();
                    text_area.insert_str(item.value_str());
                    *txt_edit = Some(text_area);
                }
            }
            ConfigType::Numberu32 { val, .. } => {
                if let Some(txt_edit) = txt_edit.as_mut() {
                    let txt = txt_edit
                        .lines()
                        .first()
                        .expect("unsound condition")
                        .trim_ascii();
                    let Ok(new_val) = txt.parse::<u32>() else {
                        return Err(format!("Could not parse '{txt}' as u32").into());
                    };
                    **val = new_val;
                } else {
                    let mut text_area = build_text_edit_area();
                    text_area.insert_str(item.value_str());
                    *txt_edit = Some(text_area);
                }
            }
            ConfigType::NumberList { val, .. } => {
                if let Some(txt_edit) = txt_edit.as_mut() {
                    let mut new_val = vec![];
                    for l in txt_edit.lines() {
                        for num in l.split_terminator(",") {
                            if let Ok(num) = num.trim_ascii().parse::<u16>() {
                                new_val.push(num);
                            }
                        }
                    }
                    new_val.dedup();
                    **val = Some(new_val);
                } else {
                    let mut text_area = build_text_edit_area();
                    text_area.insert_str(item.value_str());
                    *txt_edit = Some(text_area);
                }
            }
            ConfigType::StringList { val, .. } => {
                if let Some(txt_edit) = txt_edit.as_mut() {
                    let mut new_val = vec![];
                    for l in txt_edit.lines() {
                        for pattern in l.split_terminator(",") {
                            new_val.push(pattern.trim_ascii().to_owned());
                        }
                    }
                    new_val.dedup();
                    **val = new_val;
                } else {
                    let mut text_area = build_text_edit_area();
                    text_area.insert_str(item.value_str());
                    *txt_edit = Some(text_area);
                }
            }
        }
        Ok(())
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
