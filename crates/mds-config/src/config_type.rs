use ratatui::{
    style::{Color, Style},
    text::{Line, Text},
    widgets::ListItem,
};
use std::num::NonZeroU16;

use crate::scan;

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
    },
}

/// Maximum key length on a single line. Keys longer than this wrap to a second line.
/// "TCP Port connect [ms]" (21 chars) is the intended maximum single-line key.
pub const KEY_MAX_LEN: usize = 21;
/// Column width reserved for the key in each list row (key + three columns of padding).
/// The value column starts immediately after this reserved width.
pub const KEY_STR_LEN: usize = KEY_MAX_LEN + 3;

/// Split a key at the last word boundary at or before the single-line limit.
/// Returns `(line1, Some(line2))` when wrapping is needed, else `(key, None)`.
fn split_key(key: &'static str) -> (&'static str, Option<&'static str>) {
    if key.len() <= KEY_MAX_LEN {
        return (key, None);
    }
    if let Some(pos) = key[..=KEY_MAX_LEN].rfind(' ') {
        (&key[..pos], Some(&key[pos + 1..]))
    } else {
        // No word boundary - hard-split at the limit.
        (&key[..KEY_MAX_LEN], Some(&key[KEY_MAX_LEN..]))
    }
}

/// Build a list item from pre-split key parts, a value string, and an optional line style.
/// When the key wraps, a two-line item is created: line 1 shows the first part of the key,
/// line 2 shows the remainder padded to the value column followed by the value.
/// The list widget renders the highlight symbol automatically and indents continuation lines
/// to match, so no manual indentation is needed here.
fn make_list_item(
    key1: &'static str,
    key2: Option<&'static str>,
    val: &str,
    style: Option<Style>,
) -> ListItem<'static> {
    match key2 {
        None => {
            let text = format!("{key1:<KEY_STR_LEN$}{val}");
            ListItem::new(match style {
                Some(s) => Line::styled(text, s),
                None => Line::raw(text),
            })
        }
        Some(k2) => {
            let line1 = match style {
                Some(s) => Line::styled(key1, s),
                None => Line::raw(key1),
            };
            let line2_text = format!("{k2:<KEY_STR_LEN$}{val}");
            let line2 = match style {
                Some(s) => Line::styled(line2_text, s),
                None => Line::raw(line2_text),
            };
            ListItem::new(Text::from(vec![line1, line2]))
        }
    }
}

impl ConfigType<'_> {
    /// Number of terminal rows this item occupies (1 for short keys, 2 for wrapped keys).
    pub fn row_height(&self) -> u16 {
        if split_key(self.key()).1.is_some() {
            2
        } else {
            1
        }
    }

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
}

impl From<ConfigType<'_>> for ListItem<'_> {
    fn from(cfg_ty: ConfigType) -> Self {
        let key = cfg_ty.key();
        let val_str = cfg_ty.value_str();
        let (key1, key2) = split_key(key);
        match cfg_ty {
            ConfigType::Toggle { val, .. } => {
                let style = (*val).then_some(Style::default().fg(Color::Green));
                make_list_item(key1, key2, &val_str, style)
            }
            ConfigType::NumberNonZeroU16 { .. }
            | ConfigType::Numberu32 { .. }
            | ConfigType::ScanIoThreads { .. }
            | ConfigType::StringSelect { .. } => make_list_item(key1, key2, &val_str, None),
            ConfigType::NumberList { val, .. } => {
                let v = val
                    .iter()
                    .flatten()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ");
                let v = if v.is_empty() { "-".to_owned() } else { v };
                make_list_item(key1, key2, &v, None)
            }
            ConfigType::RegexStringList { val, .. } => {
                let v = if val.is_empty() {
                    "-".to_owned()
                } else {
                    val.join(", ")
                };
                make_list_item(key1, key2, &v, None)
            }
        }
    }
}
