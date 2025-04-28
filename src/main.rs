use clap::Parser;
use regex::Regex;

mod collect_ip;

pub(crate) mod constants;
pub(crate) mod host_up;
pub(crate) mod info_collecter;
pub(crate) mod ip_info;
pub(crate) mod log;
pub(crate) mod network_scan;
pub(crate) mod new_network_scan;
pub(crate) mod ping;
pub(crate) mod scan_ip;
pub(crate) mod tui;
pub(crate) mod util;

/// Network scanner application
#[derive(Parser, Debug)]
#[command(about = "Scans network interfaces and IPs")]
struct Args {
    /// Regex pattern to ignore network interfaces (can be used multiple times)
    #[arg(long = "ignore-re-iface")]
    ignore_re_iface: Vec<String>,
}

/// Compile all regex patterns, returning an error if any are invalid
fn compile_patterns(patterns: &[String]) -> Vec<Regex> {
    let mut compiled_patterns = Vec::with_capacity(patterns.len());

    for pattern in patterns {
        let re = Regex::new(&pattern).expect("Invalid regular expression");
        compiled_patterns.push(re);
    }

    compiled_patterns
}

fn main() -> color_eyre::Result<()> {
    let args = Args::parse();

    let ignore_iface_patterns: Vec<regex::Regex> = compile_patterns(&args.ignore_re_iface);

    tui::plumbing::install_panic_hook();
    let mut terminal = tui::plumbing::init_terminal()?;
    let mut model = tui::model::Model::new(ignore_iface_patterns);

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
