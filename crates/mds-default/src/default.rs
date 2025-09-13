/// Declare config keys and their default values with optional nesting.
///
/// # Usage
///
/// ```
/// # use mds_default::config_fields;
/// config_fields! {
///     /// Use compact output format
///     /// Hides help footer
///     compact: bool = false;
///
///     #[section]
///     /// Network timeout settings
///     Timeouts {
///         /// How long to wait for TCP connection attempts
///         /// This timeout applies to each port individually
///         tcp_port_ms: u16 = 100;
///         /// How long to wait for ping/echo replies
///         ping_ms: u16 = 300;
///         /// Upper time limit for checking if a host is up on an IP address
///         /// This is the total timeout for all host-up detection methods combined
///         ip_check_ms: u16 = 5000;
///     }
/// }
///
/// // Test the generated constants
/// assert_eq!(COMPACT.key, "compact");
/// assert_eq!(COMPACT.value, false);
/// assert_eq!(TIMEOUTS_TCP_PORT_MS.key, "timeout.tcp_port_ms");
/// assert_eq!(TIMEOUTS_TCP_PORT_MS.value, 100);
/// ```
#[macro_export]
macro_rules! config_fields {
    (
        $(
            // Regular field
            $(#[doc = $desc:expr])*
            $field:ident : $ty:ty = $value:expr;
        )*
        $(
            // Section with #[section] attribute
            #[section]
            $(#[doc = $section_desc:expr])*
            $section:ident {
                $(
                    $(#[doc = $nested_desc:expr])*
                    $nested_field:ident : $nested_ty:ty = $nested_value:expr;
                )*
            }
        )*
    ) => {
        pub mod keys {
            pastey::paste! {
                $(
                    pub const [<$field:upper>]: &str = stringify!($field);
                )*
                $(
                    $(
                        pub const [<$section:upper _ $nested_field:upper>]: &str = concat!(stringify!([<$section:snake>]), ".", stringify!($nested_field));
                    )*
                )*
            }
        }

        pub mod values {
            pastey::paste! {
                $(
                    pub const [<$field:upper>]: $ty = $value;
                )*
                $(
                    $(
                        pub const [<$section:upper _ $nested_field:upper>]: $nested_ty = $nested_value;
                    )*
                )*
            }
        }

        pub mod descriptions {
            pastey::paste! {
                $(
                    pub const [<$field:upper>]: &str = concat!($($desc, "\n"),*);
                )*
                $(
                    $(
                        pub const [<$section:upper _ $nested_field:upper>]: &str = concat!($($nested_desc, "\n"),*);
                    )*
                )*
            }
        }

        pub struct ConfigField<'a, T> {
            pub key: &'a str,
            pub value: T,
            pub description: &'a str,
        }

        pastey::paste! {
            $(
                $(#[doc = $desc])*
                pub const [<$field:upper>]: ConfigField<$ty> = ConfigField {
                    key: keys::[<$field:upper>],
                    value: values::[<$field:upper>],
                    description: descriptions::[<$field:upper>],
                };
            )*
            $(
                $(
                    $(#[doc = $nested_desc])*
                    pub const [<$section:upper _ $nested_field:upper>]: ConfigField<$nested_ty> = ConfigField {
                        key: keys::[<$section:upper _ $nested_field:upper>],
                        value: values::[<$section:upper _ $nested_field:upper>],
                        description: descriptions::[<$section:upper _ $nested_field:upper>],
                    };
                )*
            )*
        }

        /// Iterator over all config field metadata
        pub fn all_config_fields() -> impl Iterator<Item = (&'static str, &'static str, &'static str, &'static str)> {
            pastey::paste! {
                [
                    $(
                        (keys::[<$field:upper>], stringify!($value), descriptions::[<$field:upper>], stringify!($ty)),
                    )*
                    $(
                        $(
                            (keys::[<$section:upper _ $nested_field:upper>], stringify!($nested_value), descriptions::[<$section:upper _ $nested_field:upper>], stringify!($nested_ty)),
                        )*
                    )*
                ].into_iter()
            }
        }

        /// Get section information for nested fields
        pub fn get_sections() -> impl Iterator<Item = (&'static str, &'static str)> {
            pastey::paste! {
                [
                    $(
                        (stringify!([<$section:snake>]), concat!($($section_desc, "\n"),*)),
                    )*
                ].into_iter()
            }
        }

        pub fn generate_default_toml() -> String {
            let mut final_str = String::new();
            let mut fields = all_config_fields().peekable();

            while let Some((key, value, description, _type_name)) = fields.next() {
                let mut comment = String::new();
                for l in description.lines() {
                    comment.push('#');
                    comment.push(' ');
                    comment.push_str(l.trim());
                    comment.push('\n');
                }
                final_str.push_str(&comment);

                let val_str = value.strip_prefix("&").unwrap_or(value);
                let key_str = format!("{key} = {val_str}");

                final_str.push_str(&key_str);

                if fields.peek().is_some() {
                    final_str.push_str("\n\n");
                }
            }
            final_str
        }

        pub fn generate_default_toml_sections() -> String {
            use std::collections::BTreeMap;

            // First collect all fields grouped by section with their descriptions
            let mut sections: BTreeMap<Option<&str>, Vec<(&str, &str, &str)>> = BTreeMap::new();
            let mut section_descriptions: BTreeMap<&str, &str> = BTreeMap::new();

            // Collect section descriptions first
            for (section, description) in get_sections() {
                section_descriptions.insert(section, description);
            }

            // Then collect all fields
            for (key, value, description, _type_name) in all_config_fields() {
                let mut parts = key.split('.');
                let section = parts.next();
                let field = parts.next();

                if field.is_some() {
                    // This is a nested field
                    sections.entry(section)
                        .or_default()
                        .push((field.unwrap(), value, description));
                } else {
                    // This is a top-level field
                    sections.entry(None)
                        .or_default()
                        .push((key, value, description));
                }
            }

            let mut final_str = String::new();
            let mut has_previous_section = false; // To manage newlines between sections

            for (section_opt, fields) in sections.into_iter() { // Iterate over all entries
                if has_previous_section && section_opt.is_some() { // Add newline only between actual sections
                    final_str.push('\n');
                }

                match section_opt {
                    None => { // Handle top-level fields
                        for (key, value, description) in fields {
                            // Add field description comments
                            for line in description.lines() {
                                if !line.trim().is_empty() {
                                    final_str.push_str("# ");
                                    final_str.push_str(line.trim());
                                    final_str.push('\n');
                                }
                            }

                            let val_str = value.strip_prefix("&").unwrap_or(value);
                            final_str.push_str(&format!("{key} = {val_str}\n\n"));
                        }
                    },
                    Some(section) => { // Handle sections
                        // Add section description if available
                        if let Some(desc) = section_descriptions.get(section) {
                            for line in desc.lines() {
                                if !line.trim().is_empty() {
                                    final_str.push_str("# ");
                                    final_str.push_str(line.trim());
                                    final_str.push('\n');
                                }
                            }
                        }

                        // Add section header
                        final_str.push_str(&format!("[{section}]\n"));

                        for (key, value, description) in fields {
                            // Add field description comments
                            for line in description.lines() {
                                if !line.trim().is_empty() {
                                    final_str.push_str("# ");
                                    final_str.push_str(line.trim());
                                    final_str.push('\n');
                                }
                            }

                            let val_str = value.strip_prefix("&").unwrap_or(value);
                            final_str.push_str(&format!("{key} = {val_str}\n\n"));
                        }
                    },
                }
                has_previous_section = true;
            }

            final_str
        }
    };
}
