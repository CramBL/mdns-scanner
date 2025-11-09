// The test tries running the binary and that won't work on windows due to missing npcap dll
#![cfg(not(target_os = "windows"))]

use std::io;

use insta::assert_snapshot;
use mds_config::AppConfig;
use mds_log::{LogLevel, prelude::Logger};
use mds_tui::{
    Model,
    message::{Action, Message},
};
use ratatui::{
    Terminal,
    backend::TestBackend,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
};
use semver::Version;

const TEST_APP_VERSION: Version = Version::new(1, 2, 3);

fn setup_app(cfg: AppConfig) -> Model<'static, 'static> {
    let (tx, rx) = std::sync::mpsc::channel();
    let logger = Logger::new(tx, LogLevel::Info);
    Model::new(cfg, &TEST_APP_VERSION, (logger, rx))
}

fn draw(mut model: Model<'_, '_>) -> io::Result<Terminal<TestBackend>> {
    let mut terminal = Terminal::new(TestBackend::new(80, 20))?;
    terminal.draw(|frame| model.render(frame))?;
    Ok(terminal)
}

fn insta_filter_random_vals() -> Vec<(&'static str, &'static str)> {
    vec![(
        ".*Scanning potential hosts 0/[0-9]+.*",
        "\"                         Scanning potential hosts 0/1337                        \"",
    )]
}

#[test]
fn test_render_default() {
    let model = setup_app(AppConfig::default());
    let term = draw(model).unwrap();
    insta::with_settings!({filters => insta_filter_random_vals()}, {
        assert_snapshot!(term.backend());
    });
}

#[test]
fn test_render_compact_mode() {
    let mut cfg = AppConfig::default();
    cfg.ui.compact = true;
    let model = setup_app(cfg);
    let term = draw(model).unwrap();
    insta::with_settings!({filters => insta_filter_random_vals()}, {
        assert_snapshot!(term.backend());
    });
}

#[test]
fn test_render_default_search_box() {
    let mut model = setup_app(AppConfig::default());

    let mut msg = model.update(Action::Search);
    while msg.is_some() {
        msg = model.update(msg.unwrap());
    }

    let term = draw(model).unwrap();
    insta::with_settings!({filters => insta_filter_random_vals()}, {
        assert_snapshot!(term.backend());
    });
}

#[test]
fn test_render_default_config_editor_box() {
    let mut model = setup_app(AppConfig::default());

    let mut msg: Option<Message> = model.update(Action::Config);
    while msg.is_some() {
        msg = model.update(msg.unwrap());
    }

    let term = draw(model).unwrap();
    assert_snapshot!(term.backend());
}

#[test]
fn test_render_default_config_editor_box_next_tab() {
    let mut model = setup_app(AppConfig::default());

    let mut msg: Option<Message> = model.update(Action::Config);
    while msg.is_some() {
        msg = model.update(msg.unwrap());
    }
    msg = model.update(Message::BoxInput(KeyEvent::new(
        KeyCode::Right,
        KeyModifiers::empty(),
    )));
    while msg.is_some() {
        msg = model.update(msg.unwrap());
    }

    let term = draw(model).unwrap();
    assert_snapshot!(term.backend());
}

#[test]
fn test_render_default_config_editor_box_select_edit() {
    let mut model = setup_app(AppConfig::default());

    let mut msg: Option<Message> = model.update(Action::Config);
    while msg.is_some() {
        msg = model.update(msg.unwrap());
    }

    msg = model.update(Message::BoxInput(KeyEvent::new(
        KeyCode::Enter,
        KeyModifiers::empty(),
    )));
    while msg.is_some() {
        msg = model.update(msg.unwrap());
    }

    let term = draw(model).unwrap();
    assert_snapshot!(term.backend());
}
