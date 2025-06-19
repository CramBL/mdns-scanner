use std::{
    fs, io,
    path::{Path, PathBuf},
};

use toml_edit::{DocumentMut, Item, Value};

use crate::{AppConfig, DEFAULT_CONFIG, error::ConfigLoadError};

impl AppConfig {
    /// Write the default configuration to the user config directory
    pub fn write_default_config() -> Result<PathBuf, ConfigLoadError> {
        let config_path = Self::user_config_path().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Could not determine user config directory",
            )
        })?;

        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&config_path, DEFAULT_CONFIG)?;

        Ok(config_path)
    }

    /// Update a TOML document with new values while preserving comments
    pub fn update_document(
        doc: &mut DocumentMut,
        config: &AppConfig,
    ) -> Result<(), ConfigLoadError> {
        // Update each field, preserving existing comments and structure
        let iface_ignore_re = mds_default::IFACE_IGNORE_RE.key;
        if let Some(array) = doc[iface_ignore_re].as_array_mut() {
            array.clear();
            for pattern in &config.iface_ignore_re {
                array.push(pattern.as_str());
            }
        } else {
            doc[iface_ignore_re] = Item::Value({
                let mut arr = toml_edit::Array::new();
                for pattern in &config.iface_ignore_re {
                    arr.push(pattern.as_str());
                }
                Value::Array(arr)
            });
        }

        doc[mds_default::IFACE_INCLUDE_DOCKER.key] = Item::Value(Value::Boolean(
            toml_edit::Formatted::new(config.iface_include_docker),
        ));
        doc[mds_default::SERVICE_DISCOVERY.key] = Item::Value(Value::Boolean(
            toml_edit::Formatted::new(config.service_discovery),
        ));
        doc[mds_default::COMPACT.key] =
            Item::Value(Value::Boolean(toml_edit::Formatted::new(config.compact)));
        doc[mds_default::TCP_PORT_TIMEOUT_MS.key] = Item::Value(Value::Integer(
            toml_edit::Formatted::new(config.tcp_port_timeout_ms as i64),
        ));
        doc[mds_default::PING_TIMEOUT_MS.key] = Item::Value(Value::Integer(
            toml_edit::Formatted::new(config.ping_timeout_ms as i64),
        ));
        doc[mds_default::IP_CHECK_TIMEOUT_MS.key] = Item::Value(Value::Integer(
            toml_edit::Formatted::new(config.ip_check_timeout_ms as i64),
        ));
        doc[mds_default::HIDE_BARE_IPS.key] = Item::Value(Value::Boolean(
            toml_edit::Formatted::new(config.hide_bare_ips),
        ));

        Ok(())
    }

    /// Save configuration to file while preserving comments and formatting
    pub fn save_with_comments(
        path: impl AsRef<Path>,
        config: &AppConfig,
        doc: Option<DocumentMut>,
    ) -> Result<(), ConfigLoadError> {
        let document = if let Some(mut doc) = doc {
            Self::update_document(&mut doc, config)?;
            doc
        } else {
            let toml_string = toml_edit::ser::to_string_pretty(config)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            toml_string.parse::<DocumentMut>()?
        };

        fs::write(path, document.to_string())?;
        Ok(())
    }

    /// Load, modify, and save config while preserving comments
    pub fn modify_user_config<F>(modifier: F) -> Result<PathBuf, ConfigLoadError>
    where
        F: FnOnce(&mut AppConfig),
    {
        let config_path = Self::user_config_path().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Could not determine user config directory",
            )
        })?;

        let (mut config, doc) = if config_path.exists() {
            Self::load_with_comments(&config_path)?
        } else {
            // Create default config first
            Self::write_default_config()?;
            Self::load_with_comments(&config_path)?
        };

        // Apply modifications
        modifier(&mut config);

        Self::save_with_comments(&config_path, &config, Some(doc))?;

        Ok(config_path)
    }
}

#[cfg(test)]
mod tests {

    use testresult::TestResult;

    use super::*;

    #[test]
    fn save_with_comments() -> TestResult {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("save.toml");
        fs::write(
            &path,
            r#"
        # Original comment
        compact = false
    "#,
        )?;

        let (mut cfg, doc) = AppConfig::load_with_comments(&path)?;
        cfg.compact = true;
        AppConfig::save_with_comments(&path, &cfg, Some(doc))?;

        let updated = fs::read_to_string(&path)?;
        assert!(updated.contains("compact = true"));
        assert!(updated.contains("# Original comment"));
        Ok(())
    }

    #[test]
    fn modify_user_config_creates_default_safe_and_preserves_comments() -> TestResult {
        let dir = tempfile::tempdir()?;
        let config_path = dir.path().join("mdns-scanner/config.toml");

        fs::create_dir_all(config_path.parent().unwrap())?;

        // Create a config with comments
        let original = r#"
        # This is a comment before iface_include_docker
        iface_include_docker = false

        # Timeout for TCP port checks
        tcp_port_timeout_ms = 150
    "#;

        fs::write(&config_path, original)?;

        // Load, modify, and save
        let (mut config, doc) = AppConfig::load_with_comments(&config_path)?;
        assert!(!config.iface_include_docker);
        config.iface_include_docker = true;
        AppConfig::save_with_comments(&config_path, &config, Some(doc))?;

        // Reload as plain text and check for comment preservation
        let updated_content = fs::read_to_string(&config_path)?;
        assert!(updated_content.contains("# This is a comment before iface_include_docker"));
        assert!(updated_content.contains("# Timeout for TCP port checks"));
        assert!(updated_content.contains("iface_include_docker = true"));

        Ok(())
    }
}
