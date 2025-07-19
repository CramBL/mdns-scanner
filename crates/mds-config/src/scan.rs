use serde::{Deserialize, Serialize, Serializer, de::Unexpected};

use crate::config_type::ConfigType;

use std::fmt;

// Even if the target has 1 CPU, we will use a fair number of threads
pub const MIN_LOW_TIER_THREADS: usize = 32;
pub const MAX_IO_THREADS: usize = 8192;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IoThreads {
    Dynamic,
    Fixed(u16),
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
                    "'dynamic' or an integeger between {MIN_LOW_TIER_THREADS} and {MAX_IO_THREADS}"
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

fn default_io_threads() -> IoThreads {
    IoThreads::Dynamic
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Scan {
    pub service_discovery: bool,
    pub tcp_ports: Option<Vec<u16>>,
    #[serde(default = "default_io_threads")]
    pub io_threads: IoThreads,
}

impl Default for Scan {
    fn default() -> Self {
        Self {
            service_discovery: mds_default::SCAN_SERVICE_DISCOVERY.value,
            tcp_ports: Some(mds_default::SCAN_TCP_PORTS.value.to_vec()),
            io_threads: IoThreads::Dynamic,
        }
    }
}

impl Scan {
    pub fn items(&mut self) -> Vec<ConfigType> {
        vec![
            ConfigType::Toggle {
                key: "Service Discovery",
                val: &mut self.service_discovery,
                description: mds_default::SCAN_SERVICE_DISCOVERY.description,
            },
            ConfigType::NumberList {
                key: "TCP Ports",
                val: &mut self.tcp_ports,
                description: mds_default::SCAN_TCP_PORTS.description,
            },
            ConfigType::ScanIoThreads {
                key: "I/O Threads",
                val: &mut self.io_threads,
                description: mds_default::SCAN_IO_THREADS.description,
            },
        ]
    }
}
