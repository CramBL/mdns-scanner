use crate::AppConfig;

#[derive(Debug, Clone, PartialEq)]
pub enum ConfigToggle {
    ConfigField {
        label: String,
        field_id: ConfigFieldId,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConfigFieldId {
    ServiceDiscovery,
    IncludeDocker,
    Compact,
    HideBareIps,
}

impl ConfigToggle {
    pub fn name(&self) -> &str {
        match self {
            ConfigToggle::ConfigField { label, .. } => label,
        }
    }

    pub fn enabled(&self, cfg: &AppConfig) -> bool {
        match self {
            ConfigToggle::ConfigField { field_id, .. } => match field_id {
                ConfigFieldId::ServiceDiscovery => cfg.service_discovery_enabled(),
                ConfigFieldId::IncludeDocker => cfg.iface_include_docker(),
                ConfigFieldId::Compact => cfg.compact(),
                ConfigFieldId::HideBareIps => cfg.hide_bare_ips(),
            },
        }
    }

    pub fn toggle(&mut self, cfg: &mut AppConfig) {
        match self {
            ConfigToggle::ConfigField { field_id, .. } => match field_id {
                ConfigFieldId::ServiceDiscovery => {
                    cfg.service_discovery = !cfg.service_discovery;
                }
                ConfigFieldId::IncludeDocker => {
                    cfg.iface_include_docker = !cfg.iface_include_docker;
                }
                ConfigFieldId::Compact => {
                    cfg.compact = !cfg.compact;
                }
                ConfigFieldId::HideBareIps => cfg.hide_bare_ips = !cfg.hide_bare_ips,
            },
        }
    }
}
