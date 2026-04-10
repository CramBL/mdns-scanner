// The test tries running the binary and that won't work on windows due to missing npcap dll
#![cfg(not(target_os = "windows"))]

mod common;

use common::{ModelHarness, insta_filters};
use insta::assert_snapshot;
use mds_config::AppConfig;
use mds_keybindings::Action;
use mds_tui::message::Message;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[test]
fn test_render_default() {
    let mut h = ModelHarness::new(AppConfig::default());
    let term = h.draw().unwrap();
    insta::with_settings!({filters => insta_filters()}, {
        assert_snapshot!(term.backend());
    });
}

#[test]
fn test_render_default_search_box() {
    let mut h = ModelHarness::new(AppConfig::default());
    h.run(Action::Search);
    let term = h.draw().unwrap();
    insta::with_settings!({filters => insta_filters()}, {
        assert_snapshot!(term.backend());
    });
}

#[test]
fn test_render_default_config_editor_box() {
    let mut h = ModelHarness::new(AppConfig::default());
    h.run(Action::Config);
    let term = h.draw().unwrap();
    assert_snapshot!(term.backend());
}

#[test]
fn test_render_default_config_editor_box_next_tab() {
    let mut h = ModelHarness::new(AppConfig::default());
    h.run(Action::Config);
    h.run(Message::BoxInput(KeyEvent::new(
        KeyCode::Right,
        KeyModifiers::empty(),
    )));
    let term = h.draw().unwrap();
    assert_snapshot!(term.backend());
}

#[test]
fn test_render_default_config_editor_box_select_edit() {
    let mut h = ModelHarness::new(AppConfig::default());
    h.run(Action::Config);
    h.run(Message::BoxInput(KeyEvent::new(
        KeyCode::Enter,
        KeyModifiers::empty(),
    )));
    let term = h.draw().unwrap();
    assert_snapshot!(term.backend());
}

#[test]
fn test_render_default_keybindings_popup() {
    let mut h = ModelHarness::new(AppConfig::default());
    h.run(Action::Keybindings);
    let term = h.draw().unwrap();
    assert_snapshot!(term.backend());
}
