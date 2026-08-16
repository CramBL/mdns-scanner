use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Unexpected};

use std::{fmt, num::NonZero};

// Even if the host has 1 CPU, we will use a fair number of threads
pub const MIN_LOW_TIER_THREADS: usize = 32;
pub const MAX_IO_THREADS: usize = 8192;

/// Which scan a thread-count setting configures. The two accept different
/// minimums: subnet scanning keeps a floor so a whole network is swept
/// promptly, while a port scan may run as few as one worker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoThreadsField {
    Subnet,
    PortScan,
}

impl IoThreadsField {
    pub fn min_threads(self) -> usize {
        match self {
            IoThreadsField::Subnet => MIN_LOW_TIER_THREADS,
            IoThreadsField::PortScan => 1,
        }
    }

    /// Whether `value` is an accepted fixed thread count for this field.
    pub fn accepts(self, value: usize) -> bool {
        (self.min_threads()..=MAX_IO_THREADS).contains(&value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IoThreads {
    Dynamic,
    Fixed(NonZero<u16>),
}

pub(crate) fn default_io_threads() -> IoThreads {
    IoThreads::Dynamic
}

pub(crate) fn deserialize_subnet_io_threads<'de, D>(deserializer: D) -> Result<IoThreads, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_any(ThreadCountVisitor {
        field: IoThreadsField::Subnet,
    })
}

pub(crate) fn deserialize_port_scan_io_threads<'de, D>(
    deserializer: D,
) -> Result<IoThreads, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_any(ThreadCountVisitor {
        field: IoThreadsField::PortScan,
    })
}

impl fmt::Display for IoThreads {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IoThreads::Dynamic => write!(f, "dynamic"),
            IoThreads::Fixed(n) => write!(f, "{n}"),
        }
    }
}

struct ThreadCountVisitor {
    field: IoThreadsField,
}

impl ThreadCountVisitor {
    fn fixed_or_invalid<E: serde::de::Error>(
        &self,
        value: i128,
        unexpected: Unexpected,
    ) -> Result<IoThreads, E> {
        match u16::try_from(value)
            .ok()
            .filter(|&n| self.field.accepts(n as usize))
            .and_then(NonZero::new)
        {
            Some(n) => Ok(IoThreads::Fixed(n)),
            None => Err(E::invalid_value(unexpected, self)),
        }
    }
}

impl serde::de::Visitor<'_> for ThreadCountVisitor {
    type Value = IoThreads;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_fmt(format_args!(
            "'dynamic' or an integer between {min} and {MAX_IO_THREADS}",
            min = self.field.min_threads()
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
        self.fixed_or_invalid(v.into(), Unexpected::Unsigned(v))
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        self.fixed_or_invalid(v.into(), Unexpected::Signed(v))
    }
}

impl<'de> Deserialize<'de> for IoThreads {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserialize_subnet_io_threads(deserializer)
    }
}

impl Serialize for IoThreads {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            IoThreads::Dynamic => serializer.serialize_str("dynamic"),
            IoThreads::Fixed(n) => serializer.serialize_u16(n.get()),
        }
    }
}

impl From<IoThreads> for toml_edit::Value {
    fn from(io_threads: IoThreads) -> Self {
        use toml_edit::{Formatted, Value};
        match io_threads {
            IoThreads::Dynamic => Value::String(Formatted::new("dynamic".to_string())),
            IoThreads::Fixed(n) => Value::Integer(Formatted::new(n.get() as i64)),
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(IoThreadsField::Subnet, 1, false)]
    #[case(IoThreadsField::Subnet, 31, false)]
    #[case(IoThreadsField::Subnet, 32, true)]
    #[case(IoThreadsField::Subnet, 8192, true)]
    #[case(IoThreadsField::Subnet, 8193, false)]
    #[case(IoThreadsField::PortScan, 0, false)]
    #[case(IoThreadsField::PortScan, 1, true)]
    #[case(IoThreadsField::PortScan, 31, true)]
    #[case(IoThreadsField::PortScan, 8192, true)]
    #[case(IoThreadsField::PortScan, 8193, false)]
    fn field_accepts_values_within_its_own_range(
        #[case] field: IoThreadsField,
        #[case] value: usize,
        #[case] expected: bool,
    ) {
        assert_eq!(field.accepts(value), expected);
    }
}
