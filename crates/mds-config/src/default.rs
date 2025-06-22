use mds_default::default_config_without_doc_header;

use crate::AppConfig;

const DEFAULT_CONFIG: &str = include_str!("../../../docs/default_config.toml");
const DEFAULT_CONFIG_HEADER: &str = "\
# mdns-scanner configuration file
#
# Possible Locations:
";
const CONFIG_LOC_DESCRIPTION: &str = if cfg!(target_os = "windows") {
    r"
# - %APPDATA%\mdns-scanner\config.toml    (user-level, persisted)
# - .\mdns-scanner.toml                   (directory local)"
} else if cfg!(target_os = "macos") {
    "\
# - ~/Library/Application Support/mdns-scanner/config.toml    (user-level, persisted)
# - ./mdns-scanner.toml                                       (directory local)"
} else {
    "\
# - ~/.config/mdns-scanner/config.toml    (user-level, persisted)
# - ./mdns-scanner.toml                   (directory local)"
};

impl AppConfig {
    /// Return the default config as a string
    pub fn default_config() -> String {
        let mut config = String::with_capacity(
            DEFAULT_CONFIG.len() + DEFAULT_CONFIG_HEADER.len() + CONFIG_LOC_DESCRIPTION.len(),
        );

        config.push_str(DEFAULT_CONFIG_HEADER);
        config.push_str(CONFIG_LOC_DESCRIPTION);
        config.push('\n');
        config.push('\n');

        let def_conf = default_config_without_doc_header();
        config.push_str(&def_conf);
        config
    }
}

#[cfg(test)]
mod tests {
    use testresult::TestResult;

    use super::*;

    #[test]
    fn test_gen_default_config_str() -> TestResult {
        let default = AppConfig::default_config();
        assert!(default.starts_with("#"));

        let config: AppConfig = toml::from_str(&default)?;

        println!("{config:?}");

        assert_eq!(config, AppConfig::default());

        Ok(())
    }
}
