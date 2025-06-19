/// Declare config keys and their default values.
///
/// # Usage
///
/// ```
/// # use mds_config::config_fields;
/// config_fields! {
///     iface_ignore_re: &[&str] = &[];
///     iface_include_docker: bool = false;
///     tcp_port_timeout_ms: u16 = 100;
/// }
/// ```
///
/// This expands to:
///
/// - `pub const` keys as string literals,
/// - a typed `ConfigField` struct constant for each,
/// - separate modules `keys` and `values` with constants for each.
#[macro_export]
macro_rules! config_fields {
    (
        $(
            $field:ident : $ty:ty = $value:expr;
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

        pub struct ConfigField<'a, T> {
            pub key: &'a str,
            pub value: T,
        }

        paste::paste! {
            $(
                pub const [<$field:upper>]: ConfigField<$ty> = ConfigField {
                    key: keys::[<$field:upper>],
                    value: values::[<$field:upper>],
                };
            )*
        }
    };
}
