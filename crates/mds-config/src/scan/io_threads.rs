use serde::{Deserialize, Serialize, Serializer, de::Unexpected};

use std::fmt;

// Even if the host has 1 CPU, we will use a fair number of threads
pub const MIN_LOW_TIER_THREADS: usize = 32;
pub const MAX_IO_THREADS: usize = 8192;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IoThreads {
    Dynamic,
    Fixed(u16),
}

pub(crate) fn default_io_threads() -> IoThreads {
    IoThreads::Dynamic
}

impl IoThreads {
    pub fn valid_value(value: usize) -> bool {
        (MIN_LOW_TIER_THREADS..=MAX_IO_THREADS).contains(&value)
    }
}

impl fmt::Display for IoThreads {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IoThreads::Dynamic => write!(f, "dynamic"),
            IoThreads::Fixed(n) => write!(f, "{n}"),
        }
    }
}

impl<'de> Deserialize<'de> for IoThreads {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Try to deserialize as either a string or an integer
        struct ThreadCountVisitor;

        impl<'de> serde::de::Visitor<'de> for ThreadCountVisitor {
            type Value = IoThreads;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_fmt(format_args!(
                    "'dynamic' or an integer between {MIN_LOW_TIER_THREADS} and {MAX_IO_THREADS}"
                ))
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if v.eq_ignore_ascii_case("dynamic") {
                    Ok(IoThreads::Dynamic)
                } else {
                    Err(E::invalid_value(Unexpected::Str(v), &self))
                }
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if IoThreads::valid_value(v as usize) {
                    Ok(IoThreads::Fixed(v as u16))
                } else {
                    Err(E::invalid_value(Unexpected::Unsigned(v), &self))
                }
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if IoThreads::valid_value(v as usize) {
                    Ok(IoThreads::Fixed(v as u16))
                } else {
                    Err(E::invalid_value(Unexpected::Signed(v), &self))
                }
            }
        }

        deserializer.deserialize_any(ThreadCountVisitor)
    }
}

impl Serialize for IoThreads {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            IoThreads::Dynamic => serializer.serialize_str("dynamic"),
            IoThreads::Fixed(n) => serializer.serialize_u16(*n),
        }
    }
}

impl From<IoThreads> for toml_edit::Value {
    fn from(io_threads: IoThreads) -> Self {
        use toml_edit::{Formatted, Value};
        match io_threads {
            IoThreads::Dynamic => Value::String(Formatted::new("dynamic".to_string())),
            IoThreads::Fixed(n) => Value::Integer(Formatted::new(n as i64)),
        }
    }
}
