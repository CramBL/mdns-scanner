use std::sync::OnceLock;

use semver::Version;

pub const APP_VERSION_MAJOR: &str = env!("CARGO_PKG_VERSION_MAJOR");
pub const APP_VERSION_MINOR: &str = env!("CARGO_PKG_VERSION_MINOR");
pub const APP_VERSION_PATCH: &str = env!("CARGO_PKG_VERSION_PATCH");
pub static APP_VERSION: OnceLock<Version> = OnceLock::new();

pub fn app_version() -> &'static Version {
    APP_VERSION.get_or_init(|| {
        Version::new(
            APP_VERSION_MAJOR.parse().expect("Invalid major version"),
            APP_VERSION_MINOR.parse().expect("Invalid minor version"),
            APP_VERSION_PATCH.parse().expect("Invalid patch version"),
        )
    })
}
