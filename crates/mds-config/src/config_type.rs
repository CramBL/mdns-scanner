use ratatui::{
    style::{Color, Style},
    text::Line,
    widgets::ListItem,
};
use std::num::NonZeroU16;

use crate::scan;

/// Post-change behaviour to run whenever a `StringSelect` value is applied.
///
/// Declared on the `StringSelect` variant itself so the TUI layer never needs
/// to match on config key strings.  Add a new variant whenever a new selector
/// field requires post-change behaviour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectorSideEffect {
    None,
    /// Increment the `SharedConfig` theme-generation counter so the TUI
    /// reloads `TableColors` on the next frame.
    BumpThemeVersion,
}

#[derive(Debug)]
pub enum ConfigType<'c> {
    Toggle {
        key: &'static str,
        val: &'c mut bool,
        description: &'static str,
    },
    NumberNonZeroU16 {
        key: &'static str,
        val: &'c mut NonZeroU16,
        description: &'static str,
    },
    Numberu32 {
        key: &'static str,
        val: &'c mut u32,
        description: &'static str,
    },
    NumberList {
        key: &'static str,
        val: &'c mut Option<Vec<u16>>,
        description: &'static str,
    },
    RegexStringList {
        key: &'static str,
        val: &'c mut Vec<String>,
        description: &'static str,
    },
    ScanIoThreads {
        key: &'static str,
        val: &'c mut scan::IoThreads,
        description: &'static str,
    },
    /// Select from a fixed list of string options via an interactive picker.
    StringSelect {
        key: &'static str,
        val: &'c mut String,
        options: &'static [&'static str],
        description: &'static str,
        side_effect: SelectorSideEffect,
    },
}

const KEY_STR_LEN: usize = 25;

impl ConfigType<'_> {
    pub fn key(&self) -> &'static str {
        match self {
            ConfigType::Toggle { key, .. }
            | ConfigType::NumberNonZeroU16 { key, .. }
            | ConfigType::Numberu32 { key, .. }
            | ConfigType::NumberList { key, .. }
            | ConfigType::RegexStringList { key, .. }
            | ConfigType::ScanIoThreads { key, .. }
            | ConfigType::StringSelect { key, .. } => key,
        }
    }

    pub fn value_str(&self) -> String {
        match self {
            ConfigType::Toggle { val, .. } => if **val { "[*]" } else { "[ ]" }.to_owned(),
            ConfigType::NumberNonZeroU16 { val, .. } => format!("{:>4}", val.get()),
            ConfigType::Numberu32 { val, .. } => val.to_string(),
            ConfigType::StringSelect { val, .. } => (*val).to_owned(),
            ConfigType::NumberList { val, .. } => val
                .iter()
                .flatten()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", "),
            ConfigType::RegexStringList { val, .. } => val.join(", "),
            ConfigType::ScanIoThreads { val, .. } => val.to_string(),
        }
    }

    fn format_list_value(&self, items: &[impl ToString], empty_char: char) -> String {
        let mut value = format!("{key:<KEY_STR_LEN$}", key = self.key());
        if items.is_empty() {
            value.push(empty_char);
        } else {
            let joined = items
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ");
            value.push_str(&joined);
        }
        value
    }
}

impl From<ConfigType<'_>> for ListItem<'_> {
    fn from(cfg_ty: ConfigType) -> Self {
        match cfg_ty {
            ConfigType::Toggle { ref val, .. } => {
                let text = format!(
                    "{key:<KEY_STR_LEN$}{val}",
                    key = cfg_ty.key(),
                    val = cfg_ty.value_str()
                );
                let line = if **val {
                    Line::styled(text, Style::default().fg(Color::Green))
                } else {
                    Line::raw(text)
                };
                ListItem::new(line)
            }
            ConfigType::NumberNonZeroU16 { .. }
            | ConfigType::Numberu32 { .. }
            | ConfigType::ScanIoThreads { .. }
            | ConfigType::StringSelect { .. } => ListItem::new(format!(
                "{key:<KEY_STR_LEN$}{val}",
                key = cfg_ty.key(),
                val = cfg_ty.value_str()
            )),
            ConfigType::NumberList { ref val, .. } => {
                let items: Vec<u16> = val.iter().flatten().copied().collect();
                ListItem::new(cfg_ty.format_list_value(&items, '-'))
            }
            ConfigType::RegexStringList { ref val, .. } => {
                ListItem::new(cfg_ty.format_list_value(val, '-'))
            }
        }
    }
}
