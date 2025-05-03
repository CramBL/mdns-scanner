use clap::Parser;

pub(crate) mod cli;
pub(crate) mod constants;
pub(crate) mod dns_sd;
pub(crate) mod host_up;
pub(crate) mod info_collector;
pub(crate) mod ip_info;
pub(crate) mod log;
pub(crate) mod network_scanner;
pub(crate) mod ping;
pub(crate) mod tui;
pub(crate) mod util;

fn main() -> color_eyre::Result<()> {
    let args = cli::Args::parse();

    tui::plumbing::install_panic_hook();
    let mut terminal = tui::plumbing::init_terminal()?;
    let mut model = tui::model::Model::new(args);

    while !model.is_done() {
        terminal.draw(|f| tui::view(&mut model, f))?;

        // Handle events and map to a Message
        let mut current_msg = tui::handle_event(&model)?;

        // Process updates as long as they return a non-None message
        while current_msg.is_some() {
            current_msg = tui::update(&mut model, current_msg.unwrap());
        }

        model.recv_new_ip_info();
        model.recv_new_logs();
    }

    tui::plumbing::restore_terminal()?;
    Ok(())
}
