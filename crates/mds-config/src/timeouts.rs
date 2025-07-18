use std::{num::NonZeroU16, time::Duration};

use serde::{Deserialize, Serialize};

use crate::ConfigType;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Timeouts {
    pub tcp_port_ms: NonZeroU16,
    pub ping_ms: NonZeroU16,
    pub ip_check_ms: NonZeroU16,
}

impl Default for Timeouts {
    fn default() -> Self {
        Self {
            tcp_port_ms: mds_default::TIMEOUTS_TCP_PORT_MS.value.try_into().unwrap(),
            ping_ms: mds_default::TIMEOUTS_PING_MS.value.try_into().unwrap(),
            ip_check_ms: mds_default::TIMEOUTS_IP_CHECK_MS.value.try_into().unwrap(),
        }
    }
}

impl Timeouts {
    pub fn items(&mut self) -> Vec<ConfigType> {
        vec![
            ConfigType::NumberNonZeroU16 {
                key: "TCP Port connect [ms]",
                val: &mut self.tcp_port_ms,
            },
            ConfigType::NumberNonZeroU16 {
                key: "Ping [ms]",
                val: &mut self.ping_ms,
            },
            ConfigType::NumberNonZeroU16 {
                key: "IP Check [ms]",
                val: &mut self.ip_check_ms,
            },
        ]
    }

    pub fn tcp_port(&self) -> Duration {
        Duration::from_millis(self.tcp_port_ms.get().into())
    }

    pub fn ping(&self) -> Duration {
        Duration::from_millis(self.ping_ms.get().into())
    }

    pub fn ip_check(&self) -> Duration {
        Duration::from_millis(self.ip_check_ms.get().into())
    }
}
