use ratatui::{
    style::{Color, Style},
    text::Line,
    widgets::ListItem,
};
use std::num::NonZeroU16;

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
}

const KEY_STR_LEN: usize = 25;

impl ConfigType<'_> {
    pub fn value_str(&self) -> String {
        match self {
            ConfigType::Toggle { val, .. } => {
                if **val {
                    "[*]".to_owned()
                } else {
                    "[ ]".to_owned()
                }
            }
            ConfigType::NumberNonZeroU16 { val, .. } => format!("{:>4}", val.get()),
            ConfigType::Numberu32 { val, .. } => val.to_string(),
            ConfigType::NumberList { val, .. } => val
                .iter()
                .flatten()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", "),
            ConfigType::RegexStringList { val, .. } => val
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", "),
        }
    }
}

impl From<ConfigType<'_>> for ListItem<'_> {
    fn from(cfg_ty: ConfigType) -> Self {
        match cfg_ty {
            ConfigType::Toggle { key, val, .. } => {
                let checkbox = if *val { "[*]" } else { "[ ]" };

                let line = Line::styled(
                    format!("{key:<KEY_STR_LEN$}{checkbox}"),
                    if *val {
                        Style::default().fg(Color::Green)
                    } else {
                        Style::default().fg(Color::White)
                    },
                );

                ListItem::new(line)
            }
            ConfigType::NumberNonZeroU16 { key, val, .. } => {
                let formatted_val = format!("{:>4}", val.get()); // Right-align within 4 spaces
                let value = format!("{key:<KEY_STR_LEN$}{formatted_val}");
                ListItem::new(value)
            }
            ConfigType::Numberu32 { key, val, .. } => {
                ListItem::new(format!("{key:<KEY_STR_LEN$}{val}"))
            }
            ConfigType::NumberList { key, val, .. } => {
                let mut value = format!("{key:<KEY_STR_LEN$}");

                if let Some(vals) = val {
                    if vals.is_empty() {
                        value.push('-');
                    } else {
                        let joined = vals
                            .iter()
                            .map(ToString::to_string)
                            .collect::<Vec<_>>()
                            .join(", ");
                        value.push_str(&joined);
                    }
                }
                ListItem::new(value)
            }
            ConfigType::RegexStringList { key, val, .. } => {
                let mut value = format!("{key:<KEY_STR_LEN$}");

                if val.is_empty() {
                    value.push('-');
                } else {
                    let joined = val
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(", ");
                    value.push_str(&joined);
                }
                ListItem::new(value)
            }
        }
    }
}
