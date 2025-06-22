use std::{
    fs, io,
    path::{Path, PathBuf},
};

use toml_edit::{DocumentMut, Item, Value};

use crate::{AppConfig, error::ConfigLoadError};

fn update_toml_value<T>(doc: &mut DocumentMut, key: &str, value: T)
where
    T: Into<Value>,
{
    if let Some(dot_pos) = key.find('.') {
        let (section, field) = key.split_at(dot_pos);
        let field = &field[1..];

        // Ensure section exists
        if !doc.contains_key(section) {
            doc[section] = Item::Table(toml_edit::Table::new());
        }

        if let Some(table) = doc[section].as_table_mut() {
            table[field] = Item::Value(value.into());
        }
    } else {
        doc[key] = Item::Value(value.into());
    }
}

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

        fs::write(&config_path, Self::default_config())?;

        Ok(config_path)
    }

    pub fn update_document(
        doc: &mut DocumentMut,
        config: &AppConfig,
    ) -> Result<(), ConfigLoadError> {
        // Handle the array field specially
        let iface_ignore_re = mds_default::IFACE_IGNORE_RE.key;
        if let Some(array) = doc[iface_ignore_re].as_array_mut() {
            array.clear();
            for pattern in &config.iface_ignore_re {
                array.push(pattern.as_str());
            }
        } else {
            let mut arr = toml_edit::Array::new();
            for pattern in &config.iface_ignore_re {
                arr.push(pattern.as_str());
            }
            update_toml_value(doc, iface_ignore_re, Value::Array(arr));
        }

        // Update all other fields using the helper
        update_toml_value(
            doc,
            mds_default::IFACE_INCLUDE_DOCKER.key,
            config.iface_include_docker,
        );
        update_toml_value(
            doc,
            mds_default::SERVICE_DISCOVERY.key,
            config.service_discovery,
        );
        update_toml_value(doc, mds_default::COMPACT.key, config.compact);
        update_toml_value(
            doc,
            mds_default::TIMEOUTS_TCP_PORT_MS.key,
            config.timeouts.tcp_port().as_millis() as i64,
        );
        update_toml_value(
            doc,
            mds_default::TIMEOUTS_PING_MS.key,
            config.timeouts.ping().as_millis() as i64,
        );
        update_toml_value(
            doc,
            mds_default::TIMEOUTS_IP_CHECK_MS.key,
            config.timeouts.ip_check().as_millis() as i64,
        );
        update_toml_value(doc, mds_default::HIDE_BARE_IPS.key, config.hide_bare_ips);

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
        iface_include_docker = false
        iface_ignore_re = []
        service_discovery = true
        hide_bare_ips = true
        [timeouts]
        tcp_port_ms = 1
        ping_ms = 1
        ip_check_ms = 1
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

        iface_ignore_re = []
        service_discovery = true
        compact = true
        hide_bare_ips = true

        [timeouts]
        tcp_port_ms = 1
        # ping timeout
        ping_ms = 1
        ip_check_ms= 1
    "#;

        fs::write(&config_path, original)?;

        // Load, modify, and save
        let (mut config, doc) = AppConfig::load_with_comments(&config_path)?;
        assert!(!config.iface_include_docker);
        config.iface_include_docker = true;
        AppConfig::save_with_comments(&config_path, &config, Some(doc))?;

        // Reload as plain text and check for comment preservation
        let updated_content = fs::read_to_string(&config_path)?;
        println!("%%%");
        println!("{updated_content}");
        println!("---");
        assert!(updated_content.contains("# This is a comment before iface_include_docker"));
        assert!(updated_content.contains("ip_check_ms= 1"));
        assert!(updated_content.contains("# ping timeout"));

        Ok(())
    }
}
