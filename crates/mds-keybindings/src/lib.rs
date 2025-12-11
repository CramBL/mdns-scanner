use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Write;
use std::path::PathBuf;

pub use action::Action;
use mds_default::DEFAULT_KEYMAP;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::Serialize;
use serde::{Deserialize, de::Deserializer};

pub mod action;
pub mod default;

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Category {
    #[default]
    Global,
}

#[derive(Clone, Debug, PartialEq)]
pub struct KeyBindings(pub HashMap<Category, HashMap<KeyEvent, Action>>);

impl Default for KeyBindings {
    fn default() -> Self {
        toml::from_str(DEFAULT_KEYMAP).expect("invalid default keybindings")
    }
}

impl KeyBindings {
    /// Validates and provides detailed feedback about the keymap.
    ///
    /// If `path` is None, uses the default config location.
    pub fn validate_and_report(path: Option<PathBuf>) -> Result<String, String> {
        let keymap_path = if let Some(p) = path {
            p
        } else {
            let user_conf = dirs::config_dir()
                .map(|d| d.join("mdns-scanner").join("keymap.toml"))
                .ok_or_else(|| "Could not determine config directory".to_string())?;
            eprintln!("Validating discovered keymap: {}", user_conf.display());
            user_conf
        };

        if !keymap_path.exists() {
            return Err(format!(
                "No keymap.toml found at: {}\nRun with --dump-default-keymap to create one.",
                keymap_path.display()
            ));
        }

        let contents = fs::read(&keymap_path)
            .map_err(|e| format!("Failed to read {}: {e}", keymap_path.display()))?;

        let keymap: KeyBindings =
            toml::from_slice(&contents).map_err(|e| format!("Invalid keymap.toml: {e}"))?;

        let mut report = format!("Valid keymap: {}\n", keymap_path.display());

        // Sort categories for stable output
        let mut categories: Vec<_> = keymap.0.iter().collect();
        categories.sort_unstable_by_key(|(category, _)| format!("{category:?}"));

        for (category, bindings) in keymap.0 {
            report.push_str(&format!("Category: {category:?}\n"));
            report.push_str(&format!("  {} key bindings defined\n", bindings.len()));

            // Count actions
            let mut action_counts: HashMap<Action, usize> = HashMap::new();
            for action in bindings.values() {
                *action_counts.entry(*action).or_insert(0) += 1;
            }

            // Sort actions for stable output
            let mut sorted_actions: Vec<_> = action_counts.into_iter().collect();
            sorted_actions.sort_unstable_by_key(|(action, _)| format!("{action:?}"));

            for (action, count) in sorted_actions {
                if count > 1 {
                    report.push_str(&format!("  - {action:?}: {count} keys bound\n"));
                }
            }
        }

        Ok(report)
    }

    pub fn is_key_basic_navigation(&self, key: KeyEvent) -> bool {
        self.handle_key(key)
            .is_some_and(|a| a.is_basic_navigation())
    }

    pub fn is_key_copy_to_clipboard(&self, key: KeyEvent) -> bool {
        self.handle_key(key)
            .is_some_and(|a| a == Action::CopyToClipboard)
    }

    pub fn handle_key(&self, key: KeyEvent) -> Option<Action> {
        let act = self
            .0
            .get(&Category::Global)
            .and_then(|bindings| bindings.get(&key))
            .copied();

        #[cfg(debug_assertions)]
        if option_env!("MDNS_SCANNER_DEVELOPMENT").is_some() {
            use std::time::SystemTime;

            // Get current time with milliseconds
            let now = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap();
            let secs = now.as_secs() % 86400; // seconds since midnight
            let millis = now.subsec_millis();
            let hours = (secs / 3600) % 24;
            let minutes = (secs % 3600) / 60;
            let seconds = secs % 60;
            let timestamp = format!("{hours:02}:{minutes:02}:{seconds:02}.{millis:03}");

            // Build modifiers string
            let mut mods = Vec::new();
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                mods.push("CTRL");
            }
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                mods.push("SHIFT");
            }
            if key.modifiers.contains(KeyModifiers::ALT) {
                mods.push("ALT");
            }
            if key.modifiers.contains(KeyModifiers::SUPER) {
                mods.push("SUPER");
            }

            let modifier_str = if mods.is_empty() {
                String::new()
            } else {
                format!("[{}]", mods.join("+"))
            };

            // Format key code for better readability
            let key_str = match key.code {
                KeyCode::Char(c) => format!("'{c}'"),
                other => format!("{other:?}"),
            };

            // Format action
            let action_str = act.map_or("None".to_string(), |a| format!("{a:?}"));

            // Write log line with aligned columns
            if let Ok(mut file) = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("key_events.txt")
            {
                let _ = writeln!(
                    file,
                    "[{timestamp}] {key_str:12} {modifier_str:15} -> {action_str}"
                );
            }
        }

        act
    }

    pub fn new_or_default(user_keys: Self) -> Self {
        Self::merge_with_defaults(user_keys, Self::default())
    }

    /// Merge user keybindings with defaults, removing default bindings for actions
    /// that the user has explicitly rebound.
    pub fn merge_with_defaults(mut user_keys: Self, default_keys: Self) -> Self {
        for (mode, default_bindings) in default_keys.0 {
            let merged_bindings = user_keys.0.entry(mode).or_default();

            // Collect all actions that the user has explicitly defined for this mode
            let user_defined_actions: HashSet<Action> = merged_bindings.values().copied().collect();

            // Add default bindings only for actions not defined by the user
            for (key, action) in default_bindings {
                if !user_defined_actions.contains(&action) {
                    merged_bindings.insert(key, action);
                }
            }
        }

        user_keys
    }

    pub fn load() -> Result<Self, toml::de::Error> {
        if let Some(user_path) =
            dirs::config_dir().map(|d| d.join("mdns-scanner").join("keymap.toml"))
            && user_path.is_file()
        {
            let user_keymap = fs::read(user_path).expect("failed reading keymap.toml");
            let user_keymap = toml::from_slice(&user_keymap)?;
            Ok(Self::new_or_default(user_keymap))
        } else {
            Ok(Self::default())
        }
    }

    pub fn get_keys_for_action(&self, action: Action) -> Vec<KeyEvent> {
        self.0
            .get(&Category::Global)
            .map(|bindings| {
                bindings
                    .iter()
                    .filter_map(|(key, act)| if *act == action { Some(*key) } else { None })
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn get_key_display_for_action(&self, action: Action) -> String {
        let keys = self.get_keys_for_action(action);
        if keys.is_empty() {
            return String::from("(unbound)");
        }

        let mut keys: Vec<String> = keys.iter().map(key_event_to_string).collect();
        keys.sort_unstable();
        keys.join("/")
    }
}

impl<'de> Deserialize<'de> for KeyBindings {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;

        let parsed_map = HashMap::<Category, HashMap<String, Action>>::deserialize(deserializer)?;

        let keybindings = parsed_map
            .into_iter()
            .map(|(mode, inner_map)| {
                let converted_inner_map: Result<HashMap<KeyEvent, Action>, D::Error> = inner_map
                    .into_iter()
                    .map(|(key_str, cmd)| {
                        let key = parse_key(&key_str).map_err(|e| {
                            D::Error::custom(format!("Failed to parse key '{key_str}': {e}"))
                        })?;
                        #[cfg(debug_assertions)]
                        eprintln!("{key_str}: {key:?} -> {cmd}");
                        Ok((key, cmd))
                    })
                    .collect();
                Ok((mode, converted_inner_map?))
            })
            .collect::<Result<HashMap<Category, HashMap<KeyEvent, Action>>, D::Error>>()?;

        Ok(KeyBindings(keybindings))
    }
}

fn parse_key_event(raw: &str) -> Result<KeyEvent, String> {
    let raw_lower = raw.to_ascii_lowercase();
    let (remaining, modifiers) = extract_modifiers(&raw_lower);
    parse_key_code_with_modifiers(remaining, modifiers)
}

fn extract_modifiers(raw: &str) -> (&str, KeyModifiers) {
    let mut modifiers = KeyModifiers::empty();
    let mut current = raw;

    loop {
        match current {
            rest if rest.starts_with("ctrl-") => {
                modifiers.insert(KeyModifiers::CONTROL);
                current = &rest[5..];
            }
            rest if rest.starts_with("alt-") => {
                modifiers.insert(KeyModifiers::ALT);
                current = &rest[4..];
            }
            rest if rest.starts_with("shift-") => {
                modifiers.insert(KeyModifiers::SHIFT);
                current = &rest[6..];
            }
            _ => break, // break out of the loop if no known prefix is detected
        };
    }

    (current, modifiers)
}

fn parse_key_code_with_modifiers(
    raw: &str,
    mut modifiers: KeyModifiers,
) -> Result<KeyEvent, String> {
    let c = match raw {
        "esc" => KeyCode::Esc,
        "enter" => KeyCode::Enter,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "home" => KeyCode::Home,
        "end" => KeyCode::End,
        "pageup" => KeyCode::PageUp,
        "pagedown" => KeyCode::PageDown,
        "backtab" => {
            modifiers.insert(KeyModifiers::SHIFT);
            KeyCode::BackTab
        }
        "backspace" => KeyCode::Backspace,
        "delete" => KeyCode::Delete,
        "insert" => KeyCode::Insert,
        "f1" => KeyCode::F(1),
        "f2" => KeyCode::F(2),
        "f3" => KeyCode::F(3),
        "f4" => KeyCode::F(4),
        "f5" => KeyCode::F(5),
        "f6" => KeyCode::F(6),
        "f7" => KeyCode::F(7),
        "f8" => KeyCode::F(8),
        "f9" => KeyCode::F(9),
        "f10" => KeyCode::F(10),
        "f11" => KeyCode::F(11),
        "f12" => KeyCode::F(12),
        "space" => KeyCode::Char(' '),
        "hyphen" | "minus" => KeyCode::Char('-'),
        "tab" => KeyCode::Tab,
        c if c.len() == 1 => {
            let mut c = c.chars().next().unwrap();
            if modifiers.contains(KeyModifiers::SHIFT) {
                c = c.to_ascii_uppercase();
            }
            KeyCode::Char(c)
        }
        _ => return Err(format!("Unable to parse {raw}")),
    };
    Ok(KeyEvent::new(c, modifiers))
}

pub fn key_event_to_string(key_event: &KeyEvent) -> String {
    let char;
    let key_code = match key_event.code {
        KeyCode::Backspace => "Backspace",
        KeyCode::Enter => "Enter",
        KeyCode::Left => "Left",
        KeyCode::Right => "Right",
        KeyCode::Up => "Up",
        KeyCode::Down => "Down",
        KeyCode::Home => "Home",
        KeyCode::End => "End",
        KeyCode::PageUp => "Pageup",
        KeyCode::PageDown => "Pagedown",
        KeyCode::Tab => "Tab",
        KeyCode::BackTab => "Backtab",
        KeyCode::Delete => "Delete",
        KeyCode::Insert => "Insert",
        KeyCode::F(c) => {
            char = format!("F{c}");
            &char
        }
        KeyCode::Char(' ') => "Space",
        KeyCode::Char(c) => {
            char = c.to_uppercase().to_string();
            &char
        }
        KeyCode::Esc => "Esc",
        KeyCode::Null
        | KeyCode::CapsLock
        | KeyCode::Menu
        | KeyCode::ScrollLock
        | KeyCode::Media(_)
        | KeyCode::NumLock
        | KeyCode::PrintScreen
        | KeyCode::Pause
        | KeyCode::KeypadBegin
        | KeyCode::Modifier(_) => "",
    };

    let mut modifiers = Vec::with_capacity(3);

    if key_event.modifiers.intersects(KeyModifiers::CONTROL) {
        modifiers.push("Ctrl");
    }

    if key_event.modifiers.intersects(KeyModifiers::SHIFT) {
        modifiers.push("Shift");
    }

    if key_event.modifiers.intersects(KeyModifiers::ALT) {
        modifiers.push("Alt");
    }

    let mut key = modifiers.join("-");

    if !key.is_empty() {
        key.push('-');
    }
    key.push_str(key_code);

    key
}

pub fn parse_key(raw: &str) -> Result<KeyEvent, String> {
    if raw.chars().filter(|c| *c == '>').count() != raw.chars().filter(|c| *c == '<').count() {
        return Err(format!(
            "Unable to parse `{raw} - Token start/end mismatch`"
        ));
    }
    let raw = if !raw.contains("><") {
        let raw = raw.strip_prefix('<').unwrap_or(raw);
        raw.strip_suffix('>').unwrap_or(raw)
    } else {
        raw
    };

    parse_key_event(raw)
}

#[cfg(test)]
mod tests {

    use testresult::TestResult;

    use super::*;

    #[test]
    fn test_simple_keys() {
        assert_eq!(
            parse_key_event("a").unwrap(),
            KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty())
        );

        assert_eq!(
            parse_key_event("enter").unwrap(),
            KeyEvent::new(KeyCode::Enter, KeyModifiers::empty())
        );

        assert_eq!(
            parse_key_event("esc").unwrap(),
            KeyEvent::new(KeyCode::Esc, KeyModifiers::empty())
        );
    }

    #[test]
    fn test_with_modifiers() {
        assert_eq!(
            parse_key_event("ctrl-a").unwrap(),
            KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL)
        );

        assert_eq!(
            parse_key_event("alt-enter").unwrap(),
            KeyEvent::new(KeyCode::Enter, KeyModifiers::ALT)
        );

        assert_eq!(
            parse_key_event("shift-esc").unwrap(),
            KeyEvent::new(KeyCode::Esc, KeyModifiers::SHIFT)
        );
    }

    #[test]
    fn test_multiple_modifiers() {
        assert_eq!(
            parse_key_event("ctrl-alt-a").unwrap(),
            KeyEvent::new(
                KeyCode::Char('a'),
                KeyModifiers::CONTROL | KeyModifiers::ALT
            )
        );

        assert_eq!(
            parse_key_event("ctrl-shift-enter").unwrap(),
            KeyEvent::new(KeyCode::Enter, KeyModifiers::CONTROL | KeyModifiers::SHIFT)
        );
    }

    #[test]
    fn test_deser_keybindings() -> TestResult {
        let keys: KeyBindings = toml::from_str(DEFAULT_KEYMAP)?;
        let key_event = KeyEvent::new(KeyCode::Char(' '), KeyModifiers::empty());
        let act = keys.0[&Category::Global].get(&key_event);
        assert_eq!(act, Some(&Action::NavigateSelect));
        Ok(())
    }

    #[test]
    fn test_default_keybindings_snapshot() -> TestResult {
        let keybindings = KeyBindings::new_or_default(KeyBindings::default());
        let global_keybindings = &keybindings.0[&Category::Global];

        let mut keymap: Vec<(String, String)> = global_keybindings
            .iter()
            .map(|(k, v)| {
                let modifier_str = k
                    .modifiers
                    .iter_names()
                    .map(|(name, _)| name)
                    .collect::<Vec<_>>()
                    .join("+");
                // Normalize KeyCode string representation for cross-platform consistent snapshots
                let key_code = match k.code {
                    KeyCode::Enter => "Enter".to_owned(), // called "Return" on Mac OS
                    other => other.to_string(),
                };
                let key_binding = if modifier_str.is_empty() {
                    key_code.clone()
                } else {
                    format!("{modifier_str}+{key_code}")
                };

                (key_binding, v.to_string())
            })
            .collect();

        // Sort for stable output
        keymap.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));

        // Find the maximum key binding length for alignment
        let max_len = keymap.iter().map(|(k, _)| k.len()).max().unwrap_or(0);
        let formatted: Vec<String> = keymap
            .into_iter()
            .map(|(key, action)| format!("{key:max_len$} : {action}"))
            .collect();

        insta::assert_debug_snapshot!(formatted);
        Ok(())
    }

    #[test]
    fn test_user_rebind_removes_default_key() {
        // Default: 'a' -> NavigateSelect
        let defaults = HashMap::from([(
            Category::Global,
            HashMap::from([(
                KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty()),
                Action::NavigateSelect,
            )]),
        )]);

        // User: 'b' -> NavigateSelect
        let user = HashMap::from([(
            Category::Global,
            HashMap::from([(
                KeyEvent::new(KeyCode::Char('b'), KeyModifiers::empty()),
                Action::NavigateSelect,
            )]),
        )]);

        let result = KeyBindings::merge_with_defaults(KeyBindings(user), KeyBindings(defaults));

        // 'b' should work
        assert_eq!(
            result.handle_key(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::empty())),
            Some(Action::NavigateSelect)
        );

        // 'a' should NOT work
        assert_eq!(
            result.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty())),
            None
        );
    }

    #[test]
    fn test_user_multiple_keys_same_action() {
        // Default: 'a' -> NavigateSelect
        let defaults = HashMap::from([(
            Category::Global,
            HashMap::from([(
                KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty()),
                Action::NavigateSelect,
            )]),
        )]);

        // User: 'b' -> NavigateSelect, 'c' -> NavigateSelect
        let user = HashMap::from([(
            Category::Global,
            HashMap::from([
                (
                    KeyEvent::new(KeyCode::Char('b'), KeyModifiers::empty()),
                    Action::NavigateSelect,
                ),
                (
                    KeyEvent::new(KeyCode::Char('c'), KeyModifiers::empty()),
                    Action::NavigateSelect,
                ),
            ]),
        )]);

        let result = KeyBindings::merge_with_defaults(KeyBindings(user), KeyBindings(defaults));

        let keys = result.get_keys_for_action(Action::NavigateSelect);
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&KeyEvent::new(KeyCode::Char('b'), KeyModifiers::empty())));
        assert!(keys.contains(&KeyEvent::new(KeyCode::Char('c'), KeyModifiers::empty())));
        assert!(!keys.contains(&KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty())));
    }

    #[test]
    fn test_unmodified_actions_keep_defaults() {
        // Default: 'a' -> NavigateSelect, 'q' -> Quit
        let defaults = HashMap::from([(
            Category::Global,
            HashMap::from([
                (
                    KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty()),
                    Action::NavigateSelect,
                ),
                (
                    KeyEvent::new(KeyCode::Char('q'), KeyModifiers::empty()),
                    Action::Quit,
                ),
            ]),
        )]);

        // User: only rebinds NavigateSelect to 'b'
        let user = HashMap::from([(
            Category::Global,
            HashMap::from([(
                KeyEvent::new(KeyCode::Char('b'), KeyModifiers::empty()),
                Action::NavigateSelect,
            )]),
        )]);

        let result = KeyBindings::merge_with_defaults(KeyBindings(user), KeyBindings(defaults));

        // NavigateSelect should use new key 'b'
        assert_eq!(
            result.handle_key(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::empty())),
            Some(Action::NavigateSelect)
        );
        assert_eq!(
            result.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty())),
            None
        );

        // Quit should still use default 'q'
        assert_eq!(
            result.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::empty())),
            Some(Action::Quit)
        );
    }

    #[test]
    fn test_empty_user_keeps_all_defaults() {
        // Default: 'a' -> NavigateSelect, 'q' -> Quit
        let defaults = HashMap::from([(
            Category::Global,
            HashMap::from([
                (
                    KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty()),
                    Action::NavigateSelect,
                ),
                (
                    KeyEvent::new(KeyCode::Char('q'), KeyModifiers::empty()),
                    Action::Quit,
                ),
            ]),
        )]);

        let empty_user = KeyBindings(HashMap::new());

        let result = KeyBindings::merge_with_defaults(empty_user, KeyBindings(defaults));

        // Both defaults should work
        assert_eq!(
            result.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty())),
            Some(Action::NavigateSelect)
        );
        assert_eq!(
            result.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::empty())),
            Some(Action::Quit)
        );
    }

    #[test]
    fn test_validate_nonexistent_keymap() -> TestResult {
        let temp_dir = tempfile::tempdir()?;
        let keymap_path = temp_dir.path().join("keymap.toml");

        let result = KeyBindings::validate_and_report(Some(keymap_path.clone()));

        let error = result.unwrap_err();
        assert!(error.starts_with("No keymap.toml found at:"));
        assert!(error.contains("Run with --dump-default-keymap to create one."));
        Ok(())
    }

    #[test]
    fn test_validate_invalid_toml() -> TestResult {
        let temp_dir = tempfile::tempdir()?;
        let keymap_path = temp_dir.path().join("keymap.toml");

        fs::write(&keymap_path, "this is not valid toml [[[")?;

        let result = KeyBindings::validate_and_report(Some(keymap_path));

        let error = result.unwrap_err();
        assert!(error.starts_with("Invalid keymap.toml:"));
        Ok(())
    }

    #[test]
    fn test_validate_valid_keymap_report() -> TestResult {
        let temp_dir = tempfile::tempdir()?;
        let keymap_path = temp_dir.path().join("keymap.toml");

        let valid_keymap = r#"
    [Global]
    "<a>" = "navigate-select"
    "<b>" = "navigate-select"
    "<space>" = "navigate-select"
    "<q>" = "quit"
    "<ctrl-c>" = "quit"
    "<esc>" = "close"
    "#;
        fs::write(&keymap_path, valid_keymap)?;

        let report = KeyBindings::validate_and_report(Some(keymap_path))?;

        insta::with_settings!({
            filters => vec![
                (r"Valid keymap: .*[/\\]keymap\.toml", "Valid keymap: [TEMP_PATH]/keymap.toml"),
            ]
        }, {
            insta::assert_snapshot!(report);
        });

        Ok(())
    }

    #[test]
    fn test_key_event_to_string_f11() {
        let f11_key = KeyEvent::new(KeyCode::F(11), KeyModifiers::empty());
        assert_eq!(&key_event_to_string(&f11_key), "F11");
    }
}
