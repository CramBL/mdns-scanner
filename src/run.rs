use std::{io, time::Duration};

use mds_config::AppConfig;
use mds_log::prelude::setup_logger;
use ratatui::{
    Terminal,
    crossterm::event::{self, Event, KeyEvent},
    prelude::Backend,
};

use crate::version::app_version;

fn poll_key_event(timeout: Duration) -> io::Result<Option<KeyEvent>> {
    if event::poll(timeout)? {
        if let Event::Key(key) = event::read()? {
            return Ok(Some(key));
        }
    }
    Ok(None)
}

pub(crate) fn run(mut terminal: Terminal<impl Backend>, cfg: AppConfig) -> color_eyre::Result<()> {
    let (logger, log_rx) = setup_logger(cfg.ui.log_level.as_str().try_into()?);
    let mut model = mds_tui::Model::new(cfg, app_version(), (logger, log_rx));

    while !model.is_done() {
        terminal.draw(|frame| model.render(frame))?;

        // Handle events and map to a Message
        let mut current_msg = None;
        if let Ok(Some(key)) = poll_key_event(model.passive_refresh_interval()) {
            current_msg = model.handle_key(key);
        }

        // Process updates as long as they return a non-None message
        while current_msg.is_some() {
            current_msg = model.update(current_msg.unwrap());
        }

        model.recv_new_ip_info();
        model.recv_new_logs();
    }
    Ok(())
}
