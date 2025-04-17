use std::sync::mpsc::{self, Receiver};

use crate::log::{LogLevel, LogMessage, db::LogDb, logger::Logger};
use ratatui::{prelude::*, widgets::*};

pub(crate) struct LogPane {
    log_db: LogDb,
    logger: Logger,
    rx_logs: Receiver<LogMessage>,
}

impl Default for LogPane {
    fn default() -> Self {
        let (tx_logs, rx_logs) = mpsc::channel();
        let logger = Logger::new(tx_logs, LogLevel::default());

        Self {
            log_db: Default::default(),
            logger,
            rx_logs,
        }
    }
}

impl LogPane {
    pub fn render(&mut self, frame: &mut Frame, area: Rect, in_focus: bool) {
        let logs = self.log_db.latest_logs(self.log_level());
        let mut list_items: Vec<ListItem> = vec![];
        for msg in logs {
            match msg {
                LogMessage::Error(s) => {
                    list_items.push(ListItem::new(s.as_ref()).red());
                }
                LogMessage::Warn(s) => list_items.push(ListItem::new(s.as_ref()).yellow()),
                LogMessage::Info(s) => list_items.push(ListItem::new(s.as_ref())),
                LogMessage::Debug(s) => list_items.push(ListItem::new(s.as_ref()).cyan()),
                LogMessage::Trace(s) => list_items.push(ListItem::new(s.as_ref()).blue()),
            }
        }

        let list = List::new(list_items);

        let block_border_symbol = if in_focus {
            symbols::border::PLAIN
        } else {
            symbols::border::EMPTY
        };

        let log_block = Block::bordered()
            .title(format!("Log Level: {}", self.log_level()))
            .border_set(block_border_symbol);

        let log_widget = list.block(log_block);

        frame.render_widget(log_widget, area);
    }

    pub fn get_logger_clone(&self) -> Logger {
        self.logger.clone()
    }

    pub(crate) fn recv_new_logs(&mut self) {
        while let Ok(l) = self.rx_logs.try_recv() {
            self.log_db.push(l);
        }
    }

    pub(crate) fn increase_verbosity(&mut self) {
        self.logger.increase_verbosity();
    }

    pub(crate) fn decrease_verbosity(&mut self) {
        self.logger.decrease_verbosity();
    }

    pub(crate) fn log_level(&self) -> LogLevel {
        self.logger.verbosity()
    }
}
