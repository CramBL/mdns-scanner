use serde::{Deserialize, Serialize, Serializer, de::Unexpected};

use crate::config_type::ConfigType;

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ThreadCount {
    Dynamic,
    Fixed(u16),
}

impl<'de> Deserialize<'de> for ThreadCount {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Try to deserialize as either a string or an integer
        struct ThreadCountVisitor;

        impl<'de> serde::de::Visitor<'de> for ThreadCountVisitor {
            type Value = ThreadCount;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str(r#""dynamic" or an integer between 10 and 2000"#)
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if v.eq_ignore_ascii_case("dynamic") {
                    Ok(ThreadCount::Dynamic)
                } else {
                    Err(E::invalid_value(Unexpected::Str(v), &self))
                }
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if (10..=2000).contains(&v) {
                    Ok(ThreadCount::Fixed(v as u16))
                } else {
                    Err(E::invalid_value(Unexpected::Unsigned(v), &self))
                }
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if (10..=2000).contains(&v) {
                    Ok(ThreadCount::Fixed(v as u16))
                } else {
                    Err(E::invalid_value(Unexpected::Signed(v), &self))
                }
            }
        }

        deserializer.deserialize_any(ThreadCountVisitor)
    }
}

impl Serialize for ThreadCount {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            ThreadCount::Dynamic => serializer.serialize_str("dynamic"),
            ThreadCount::Fixed(n) => serializer.serialize_u16(*n),
        }
    }
}

fn default_thread_count() -> ThreadCount {
    ThreadCount::Dynamic
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Scan {
    pub service_discovery: bool,
    pub tcp_ports: Option<Vec<u16>>,
    #[serde(default = "default_thread_count")]
    pub thread_count: ThreadCount,
}

impl Default for Scan {
    fn default() -> Self {
        Self {
            service_discovery: mds_default::SCAN_SERVICE_DISCOVERY.value,
            tcp_ports: Some(mds_default::SCAN_TCP_PORTS.value.to_vec()),
            thread_count: ThreadCount::Dynamic,
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
        ]
    }
}
