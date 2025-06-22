/// Declare config keys and their default values.
///
/// # Usage
///
/// ```
/// # use mds_config::config_fields;
/// config_fields! {
///     iface_ignore_re: &[&str] = &[] => "Regular expressions for interfaces to ignore";
///     iface_include_docker: bool = false => "Whether to include Docker interfaces";
///     tcp_port_timeout_ms: u16 = 100 => "TCP port connection timeout in milliseconds";
/// }
///
/// // Example assertion to verify the configuration
/// assert_eq!(TCP_PORT_TIMEOUT_MS.key, "tcp_port_timeout_ms");
/// assert_eq!(TCP_PORT_TIMEOUT_MS.value, 100);
/// assert_eq!(TCP_PORT_TIMEOUT_MS.description, "TCP port connection timeout in milliseconds");
/// ```
///
/// This expands to:
///
/// - `pub const` keys as string literals,
/// - a typed `ConfigField` struct constant for each,
/// - separate modules `keys`, `values`, and `descriptions` with constants for each.
#[macro_export]
macro_rules! config_fields {
    (
        $(
            $field:ident : $ty:ty = $value:expr => $desc:expr;
        )*
    ) => {
        pub mod keys {
            paste::paste! {
                $(
                    pub const [<$field:upper>]: &str = stringify!($field);
                )*
            }
        }

        pub mod values {
            paste::paste! {
                $(
                    pub const [<$field:upper>]: $ty = $value;
                )*
            }
        }

        pub mod descriptions {
            paste::paste! {
                $(
                    pub const [<$field:upper>]: &str = $desc;
                )*
            }
        }

        pub struct ConfigField<'a, T> {
            pub key: &'a str,
            pub value: T,
            pub description: &'a str,
        }

        paste::paste! {
            $(
                pub const [<$field:upper>]: ConfigField<$ty> = ConfigField {
                    key: keys::[<$field:upper>],
                    value: values::[<$field:upper>],
                    description: descriptions::[<$field:upper>],
                };
            )*
        }
    };
}
