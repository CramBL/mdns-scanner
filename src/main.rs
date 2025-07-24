#[cfg(target_os = "windows")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[cfg(all(
    feature = "jemalloc",
    not(target_os = "windows"),
    not(target_os = "openbsd"),
    not(target_os = "freebsd"),
    target_arch = "x86_64"
))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

#[cfg(feature = "self-update")]
mod self_update;
pub mod version;

use std::fs;

use mds_config::AppConfig;
use mds_log::prelude::setup_logger;

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

    let (logger, log_rx) = setup_logger(cfg.ui.log_level.as_str().try_into()?);
    let mut model = mds_tui::Model::new(cfg, version::app_version(), (logger, log_rx));

    while !model.is_done() {
        terminal.draw(|f| mds_tui::view(&mut model, f))?;

        // Handle events and map to a Message
        let mut current_msg = mds_tui::handle_event(&mut model)?;

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
