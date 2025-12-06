use mds_default::default_keymap_without_doc_header;

use crate::KeyBindings;

const DEFAULT_KEYMAP: &str = include_str!("../../../docs/default_config.toml");
const DEFAULT_KEYMAP_HEADER: &str = "\
# mdns-scanner keymap file
#
# Possible Locations:
";
const KEYMAP_LOC_DESCRIPTION: &str = if cfg!(target_os = "windows") {
    r"
# - %APPDATA%\mdns-scanner\keymap.toml         (user-level)"
} else if cfg!(target_os = "macos") {
    "\
# - ~/Library/Application Support/mdns-scanner/keymap.toml   (user-level)"
} else {
    "\
# - ~/.config/mdns-scanner/keymap.toml    (user-level)"
};

impl KeyBindings {
    /// Return the default config as a string
    pub fn default_keymap() -> String {
        let mut config = String::with_capacity(
            DEFAULT_KEYMAP.len() + DEFAULT_KEYMAP_HEADER.len() + KEYMAP_LOC_DESCRIPTION.len(),
        );

        config.push_str(DEFAULT_KEYMAP_HEADER);
        config.push_str(KEYMAP_LOC_DESCRIPTION);
        config.push('\n');
        config.push('\n');

        let def_conf = default_keymap_without_doc_header();
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
        let default = KeyBindings::default_keymap();
        assert!(default.starts_with("#"));

        let config: KeyBindings = toml::from_str(&default)?;

        println!("{config:?}");

        assert_eq!(config, KeyBindings::default());

        Ok(())
    }
}
