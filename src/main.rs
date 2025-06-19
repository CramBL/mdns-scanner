#[cfg(target_os = "windows")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[cfg(all(
    not(target_os = "windows"),
    not(target_os = "openbsd"),
    not(target_os = "freebsd"),
    any(
        target_arch = "x86_64",
        target_arch = "aarch64",
        target_arch = "powerpc64"
    )
))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

#[cfg(feature = "self-update")]
mod self_update;

use std::{fs, sync::OnceLock};

use mds_config::AppConfig;
use semver::Version;

pub const APP_VERSION_MAJOR: &str = env!("CARGO_PKG_VERSION_MAJOR");
pub const APP_VERSION_MINOR: &str = env!("CARGO_PKG_VERSION_MINOR");
pub const APP_VERSION_PATCH: &str = env!("CARGO_PKG_VERSION_PATCH");
pub static APP_VERSION: OnceLock<Version> = OnceLock::new();
pub fn get_app_version() -> &'static Version {
    APP_VERSION.get_or_init(|| {
        Version::new(
            APP_VERSION_MAJOR.parse().expect("Invalid major version"),
            APP_VERSION_MINOR.parse().expect("Invalid minor version"),
            APP_VERSION_PATCH.parse().expect("Invalid patch version"),
        )
    })
}

fn main() -> color_eyre::Result<()> {
    let arg_count = std::env::args().count();
    let cfg = if arg_count == 1 {
        mds_config::AppConfig::load()
    } else {
        let args = mds_cli::parse_cli_args();
        if let Some(cmd) = args.command() {
            match cmd {
                mds_cli::cli::Commands::DumpDefaultConfig { output } => {
                    if let Some(output) = output {
                        fs::write(output, AppConfig::default_config())?
                    } else {
                        print!("{}", AppConfig::default_config());
                    }
                    return Ok(());
                }
                #[cfg(feature = "self-update")]
                mds_cli::cli::Commands::Update(self_update_args) => {
                    self_update::run_self_update(
                        self_update_args.target_version,
                        self_update_args.token,
                        self_update_args.dry_run,
                    )?;
                    return Ok(());
                }
            }
        }
        mds_config::AppConfig::load_with_cli(&args)
    }?;

    mds_tui::plumbing::install_panic_hook();
    let mut terminal = mds_tui::plumbing::init_terminal()?;
    let mut model = mds_tui::Model::new(cfg, get_app_version());

    while !model.is_done() {
        terminal.draw(|f| mds_tui::view(&mut model, f))?;

        // Handle events and map to a Message
        let mut current_msg = mds_tui::handle_event(&model)?;

        // Process updates as long as they return a non-None message
        while current_msg.is_some() {
            current_msg = mds_tui::update(&mut model, current_msg.unwrap());
        }

        model.recv_new_ip_info();
        model.recv_new_logs();
    }

    mds_tui::plumbing::restore_terminal()?;
    Ok(())
}
