// The test tries running the binary and that won't work on windows due to missing npcap dll
#![cfg(not(target_os = "windows"))]

mod common;

use common::{ModelHarness, insta_filters, mixed_log_messages, rich_host_fleet};
use insta::assert_snapshot;
use mds_config::AppConfig;
use mds_keybindings::Action;
use mds_tui::message::Message;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[test]
fn test_render_many_hosts() {
    let mut h = ModelHarness::new(AppConfig::default());
    for ip in rich_host_fleet() {
        h.inject_ip(ip);
    }
    let term = h.draw_sized(120, 40).unwrap();
    insta::with_settings!({filters => insta_filters()}, {
        assert_snapshot!(term.backend());
    });
}

#[test]
fn test_render_many_hosts_scrolled_down() {
    let mut h = ModelHarness::new(AppConfig::default());
    for ip in rich_host_fleet() {
        h.inject_ip(ip);
    }
    for _ in 0..5 {
        h.run(Action::NavigateDown);
    }
    let term = h.draw_sized(120, 40).unwrap();
    insta::with_settings!({filters => insta_filters()}, {
        assert_snapshot!(term.backend());
    });
}

/// Sub-line copy mode triggered on a multi-service cell, rendered over a dense table.
#[test]
fn test_render_many_hosts_sub_line_on_service_column() {
    let mut h = ModelHarness::new(AppConfig::default());
    for ip in rich_host_fleet() {
        h.inject_ip(ip);
    }
    // Navigate right past IP → Names → Hits → Services
    h.run(Action::NavigateRight);
    h.run(Action::NavigateRight);
    h.run(Action::NavigateRight);
    h.run(Action::NavigateRight);
    // First row has multiple services, so copy enters sub-line mode.
    h.run(Action::CopyToClipboard);
    let term = h.draw_sized(120, 40).unwrap();
    insta::with_settings!({filters => insta_filters()}, {
        assert_snapshot!(term.backend());
    });
}

/// Both panes must render correctly together with the default split.
#[test]
fn test_render_many_hosts_with_many_logs() {
    let mut h = ModelHarness::new(AppConfig::default());
    for ip in rich_host_fleet() {
        h.inject_ip(ip);
    }
    for msg in mixed_log_messages() {
        h.inject_log(msg);
    }
    let term = h.draw_sized(120, 40).unwrap();
    insta::with_settings!({filters => insta_filters()}, {
        assert_snapshot!(term.backend());
    });
}

#[test]
fn test_render_log_pane_focused_scroll_end() {
    let mut h = ModelHarness::new(AppConfig::default());
    for ip in rich_host_fleet() {
        h.inject_ip(ip);
    }
    for msg in mixed_log_messages() {
        h.inject_log(msg);
    }
    h.run(Action::ToggleFocus);
    h.run(Action::NavigateScrollToEnd);
    let term = h.draw_sized(120, 40).unwrap();
    insta::with_settings!({filters => insta_filters()}, {
        assert_snapshot!(term.backend());
    });
}

/// Verify the layout when the log pane is shrunk to near-minimum height.
#[test]
fn test_render_log_pane_minimised() {
    let mut h = ModelHarness::new(AppConfig::default());
    for ip in rich_host_fleet() {
        h.inject_ip(ip);
    }
    for msg in mixed_log_messages() {
        h.inject_log(msg);
    }
    for _ in 0..4 {
        h.run(Action::IncreaseLayoutFill);
    }
    let term = h.draw_sized(120, 40).unwrap();
    insta::with_settings!({filters => insta_filters()}, {
        assert_snapshot!(term.backend());
    });
}

#[test]
fn test_render_search_filters_hosts() {
    let mut h = ModelHarness::new(AppConfig::default());
    for ip in rich_host_fleet() {
        h.inject_ip(ip);
    }
    h.run(Action::Search);
    for ch in "local".chars() {
        h.run(Message::BoxInput(KeyEvent::new(
            KeyCode::Char(ch),
            KeyModifiers::empty(),
        )));
    }
    let term = h.draw_sized(120, 40).unwrap();
    insta::with_settings!({filters => insta_filters()}, {
        assert_snapshot!(term.backend());
    });
}

#[test]
fn test_render_search_single_match() {
    let mut h = ModelHarness::new(AppConfig::default());
    for ip in rich_host_fleet() {
        h.inject_ip(ip);
    }
    h.run(Action::Search);
    for ch in "chromecast".chars() {
        h.run(Message::BoxInput(KeyEvent::new(
            KeyCode::Char(ch),
            KeyModifiers::empty(),
        )));
    }
    let term = h.draw_sized(120, 40).unwrap();
    insta::with_settings!({filters => insta_filters()}, {
        assert_snapshot!(term.backend());
    });
}
