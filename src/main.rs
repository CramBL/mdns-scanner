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

pub(crate) mod run;
#[cfg(feature = "self-update")]
mod self_update;
mod setup;
pub mod version;

fn main() -> color_eyre::Result<()> {
    let Some((cfg, keybindings)) = setup::setup()? else {
        return Ok(());
    };

    mds_tui::plumbing::install_panic_hook();
    let terminal = mds_tui::plumbing::init_terminal()?;
    run::run(terminal, cfg, keybindings)?;
    mds_tui::plumbing::restore_terminal()?;
    Ok(())
}
