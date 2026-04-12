use std::num::{NonZero, NonZeroU16};

use mds_config::{
    AppConfig,
    config_type::ConfigType,
    scan::{
        IoThreads,
        io_threads::{MAX_IO_THREADS, MIN_LOW_TIER_THREADS},
    },
    shared_config::SharedConfig,
};
use ratatui::{style::Style, widgets::ListState};
use tui_textarea::TextArea;

use crate::error_box::ErrorBox;
use crate::option_selector::{OptionSelector, SelectorSideEffect};

/// A function that produces the list of editable items for a config section.
///
/// Stored inside `CfgPickerState` so that all selector and navigation logic
/// can live there rather than in `SelectedTab`, which would otherwise have to
/// repeat a 4-arm match for every operation.
pub(super) type ItemsFn = for<'a> fn(&'a mut AppConfig) -> Vec<ConfigType<'a>>;

/// X position of overlay popups (text editor and option selector) relative to
/// the left edge of the config pane. Shared by both overlay types so they
/// align visually.
pub(super) const OVERLAY_X_OFFSET: u16 = 28;
/// Y position of overlay popups relative to the selected list item.
pub(super) const OVERLAY_Y_OFFSET_FROM_ITEM: u16 = 2;

#[derive(Clone)]
pub(crate) struct CfgPickerState<'t> {
    pub(super) cfg: SharedConfig,
    pub(super) txt_edit: Option<TextArea<'t>>,
    pub(super) option_selector: Option<OptionSelector>,
    pub(super) state: ListState,
    /// Produces config items for this picker's section of the config.
    pub(super) items_fn: ItemsFn,
}

impl<'t> CfgPickerState<'t> {
    pub(super) fn new(cfg: SharedConfig, items_fn: ItemsFn) -> Self {
        let mut state = ListState::default();
        state.select(Some(0));
        Self {
            cfg,
            txt_edit: None,
            option_selector: None,
            state,
            items_fn,
        }
    }

    pub(super) fn selector_open(&self) -> bool {
        self.option_selector.is_some()
    }

    /// Check whether the selected item is a `StringSelect` and open a selector
    /// for it.  Returns `true` when a selector was opened.
    pub(super) fn try_open_selector(&mut self) -> bool {
        let selected_idx = self.state.selected().unwrap_or(0);
        let items_fn = self.items_fn;
        let spec = self.cfg.modify(|cfg| {
            let mut items = (items_fn)(cfg);
            match items.get_mut(selected_idx) {
                Some(ConfigType::StringSelect {
                    key,
                    options,
                    val,
                    side_effect,
                    ..
                }) => Some((*key, *options, (*val).clone(), *side_effect)),
                _ => None,
            }
        });
        if let Some((key, options, current, side_effect)) = spec {
            self.option_selector = Some(OptionSelector::new(key, options, &current, side_effect));
            true
        } else {
            false
        }
    }

    /// Accept the current selector value.  The value is already written to the
    /// config via `apply_selector_preview`, so we only need to close the overlay.
    pub(super) fn confirm_selector(&mut self) {
        self.option_selector = None;
    }

    /// Dismiss the selector and restore the value that was active when it opened.
    pub(super) fn cancel_selector(&mut self) {
        let Some(sel) = &self.option_selector else {
            return;
        };
        let original = sel.original_value().to_owned();
        let config_key = sel.config_key;
        let side_effect = sel.side_effect;
        let selected_idx = self.state.selected().unwrap_or(0);

        self.option_selector = None; // close before modifying

        let items_fn = self.items_fn;
        self.cfg.modify(|cfg| {
            let mut items = (items_fn)(cfg);
            write_string_select(&mut items, selected_idx, config_key, &original);
        });
        self.apply_side_effect(side_effect);
    }

    /// Write the selector's current value to the config immediately so the
    /// user sees changes live (e.g. theme preview while navigating).
    pub(super) fn apply_selector_preview(&self) {
        let Some(sel) = &self.option_selector else {
            return;
        };
        let value = sel.current_value().to_owned();
        let config_key = sel.config_key;
        let side_effect = sel.side_effect;
        let selected_idx = self.state.selected().unwrap_or(0);

        let items_fn = self.items_fn;
        self.cfg.modify(|cfg| {
            let mut items = (items_fn)(cfg);
            write_string_select(&mut items, selected_idx, config_key, &value);
        });
        self.apply_side_effect(side_effect);
    }

    pub(super) fn navigate_selector_up(&mut self) {
        if let Some(sel) = &mut self.option_selector {
            sel.navigate_up();
        }
        self.apply_selector_preview();
    }

    pub(super) fn navigate_selector_down(&mut self) {
        if let Some(sel) = &mut self.option_selector {
            sel.navigate_down();
        }
        self.apply_selector_preview();
    }

    fn apply_side_effect(&self, effect: SelectorSideEffect) {
        match effect {
            SelectorSideEffect::None => {}
            SelectorSideEffect::BumpThemeVersion => self.cfg.bump_theme_version(),
        }
    }

    pub(super) fn handle_selected_item(&mut self) -> Result<(), ErrorBox> {
        let Some(selected) = self.state.selected() else {
            return Ok(());
        };
        let items_fn = self.items_fn;
        self.cfg.modify(|cfg| -> Result<(), ErrorBox> {
            let mut items = (items_fn)(cfg);
            if let Some(item) = items.get_mut(selected) {
                CfgPickerState::handle_confirm_action(&mut self.txt_edit, item)?;
            }
            Ok(())
        })?;
        Ok(())
    }

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
                        // SAFETY: IoThreads::valid_value guarantees non-zero
                        IoThreads::Fixed(NonZero::<u16>::new(num).unwrap())
                    };
                    **val = new_val;
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
                        if let Ok(n) = num.trim_ascii().parse::<u16>()
                            && !new_val.contains(&n)
                        {
                            new_val.push(n);
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

            ConfigType::StringSelect { .. } => {
                // Handled via OptionSelector; confirm/cancel go through the picker methods.
            }
        }
        Ok(())
    }
}

/// Write `value` into the `StringSelect` at `idx` whose key matches
/// `config_key`.  Pure data write - side effects are handled separately.
fn write_string_select(items: &mut [ConfigType<'_>], idx: usize, config_key: &str, value: &str) {
    if let Some(ConfigType::StringSelect { val, key, .. }) = items.get_mut(idx)
        && *key == config_key
    {
        **val = value.to_owned();
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
    text_area.set_placeholder_style(Style::default());
    text_area
}
