// The test tries running the binary and that won't work on windows due to missing npcap dll
#![cfg(not(target_os = "windows"))]

use std::io;

use insta::assert_snapshot;
use mds_config::AppConfig;
use mds_log::{LogLevel, prelude::Logger};
use mds_tui::Model;
use ratatui::{Terminal, backend::TestBackend};
use semver::Version;

const TEST_APP_VERSION: Version = Version::new(1, 2, 3);

fn setup_app(cfg: AppConfig) -> Model<'static, 'static> {
    let (tx, rx) = std::sync::mpsc::channel();
    let logger = Logger::new(tx, LogLevel::Info);
    Model::new(cfg, &TEST_APP_VERSION, (logger, rx))
}

fn draw(mut model: Model<'_, '_>) -> io::Result<Terminal<TestBackend>> {
    let mut terminal = Terminal::new(TestBackend::new(80, 20))?;
    terminal.draw(|frame| mds_tui::view(&mut model, frame))?;
    Ok(terminal)
}

#[test]
fn test_render_default() {
    let model = setup_app(AppConfig::default());
    let term = draw(model).unwrap();
    assert_snapshot!(term.backend());
}

#[test]
fn test_render_default_search_box() {
    let mut model = setup_app(AppConfig::default());

    mds_tui::update(&mut model, mds_tui::Message::PopupSearch);

    let term = draw(model).unwrap();
    assert_snapshot!(term.backend());
}

#[test]
fn test_render_default_config_editor_box() {
    let mut model = setup_app(AppConfig::default());

    mds_tui::update(&mut model, mds_tui::Message::PopupConfig);

    let term = draw(model).unwrap();
    assert_snapshot!(term.backend());
}
