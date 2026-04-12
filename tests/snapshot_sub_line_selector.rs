// The test tries running the binary and that won't work on windows due to missing npcap dll
#![cfg(not(target_os = "windows"))]

mod common;

use common::{ModelHarness, insta_filters, ip_with_names};
use insta::assert_snapshot;
use mds_config::AppConfig;
use mds_ipinfo::{IpForHost, IpInfo};
use mds_keybindings::Action;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// Verify that entering sub-line selection mode on a multi-name cell renders
/// the snapshot lines with a ▶ cursor and a "copy all" option.
#[test]
fn test_sub_line_selector_renders() {
    let mut h = ModelHarness::new(AppConfig::default());
    h.inject_ip(ip_with_names(&["alpha.local", "beta.local"]));

    // Navigate right twice to land on the Name column.
    h.run(Action::NavigateRight);
    h.run(Action::NavigateRight);
    h.run(Action::CopyToClipboard);

    let term = h.draw().unwrap();
    insta::with_settings!({filters => insta_filters()}, {
        assert_snapshot!(term.backend());
    });
}

/// When a new name arrives for the same IP *while* sub-line selection is open,
/// the sub-line selector cell must show the same frozen snapshot content.
/// Live-data columns (Hits, scanning progress) are allowed to update normally.
#[test]
fn test_sub_line_selector_is_stable_when_new_name_arrives() {
    let mut h = ModelHarness::new(AppConfig::default());
    h.inject_ip(ip_with_names(&["alpha.local", "beta.local"]));

    h.run(Action::NavigateRight);
    h.run(Action::NavigateRight);
    h.run(Action::CopyToClipboard);

    let before = h.draw().unwrap();
    let before_str = before.backend().to_string();

    // "aardvark.local" sorts before "alpha.local" - it would shift positional
    // indices and column width if the selector were not using a frozen snapshot.
    h.inject_ip(ip_with_names(&["aardvark.local"]));

    let after = h.draw().unwrap();
    let after_str = after.backend().to_string();

    // The frozen snapshot lines must still be present exactly as before.
    // "aardvark.local" must not appear - it arrived after the snapshot was taken.
    assert!(
        !after_str.contains("aardvark"),
        "new name 'aardvark.local' leaked into the frozen sub-line selector display"
    );
    assert!(
        after_str.contains("▶ alpha.local"),
        "cursor line 'alpha.local' disappeared from the sub-line selector"
    );
    assert!(
        after_str.contains("beta.local"),
        "'beta.local' disappeared from the sub-line selector"
    );
    assert!(
        after_str.contains("[ copy all ]"),
        "'copy all' option disappeared from the sub-line selector"
    );

    // The continuation lines (beta.local and copy-all) are not on the IP/Hits
    // row, so their full terminal lines are stable across the injection.
    let continuation_lines_before: Vec<&str> = before_str
        .lines()
        .filter(|l| l.contains("beta.local") || l.contains("copy all"))
        .collect();
    let continuation_lines_after: Vec<&str> = after_str
        .lines()
        .filter(|l| l.contains("beta.local") || l.contains("copy all"))
        .collect();
    assert_eq!(
        continuation_lines_before, continuation_lines_after,
        "column layout shifted after a new (longer) name arrived during sub-line selection"
    );
}

/// After cancelling sub-line mode the cell must return to its normal live-data rendering.
#[test]
fn test_sub_line_selector_cancel_returns_to_normal() {
    let mut h = ModelHarness::new(AppConfig::default());
    h.inject_ip(ip_with_names(&["alpha.local", "beta.local"]));

    h.run(Action::NavigateRight);
    h.run(Action::NavigateRight);
    h.run(Action::CopyToClipboard); // enter sub-line mode
    h.run(Action::Close); // cancel

    let term = h.draw().unwrap();
    insta::with_settings!({filters => insta_filters()}, {
        assert_snapshot!(term.backend());
    });
}

/// When a new IP that sorts *before* the one under sub-line selection is
/// discovered, the table row index of the tracked IP shifts.  The selection
/// highlight and sub-line selector must both follow the IP, not stay frozen on
/// the old index.
#[test]
fn test_sub_line_selector_follows_row_when_new_ip_sorts_above() {
    let mut h = ModelHarness::new(AppConfig::default());
    h.inject_ip(ip_with_names(&["alpha.local", "beta.local"]));
    h.run(Action::NavigateRight);
    h.run(Action::NavigateRight);
    h.run(Action::CopyToClipboard);

    // Navigate to "beta.local" so we can verify the cursor stays correct.
    h.run(Action::NavigateDown);

    // Inject an IP that sorts before the selected one, shifting its row index.
    let mut earlier_ip = IpInfo::from_ip(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 0)));
    earlier_ip.add_name("earlier.local".to_owned());
    h.inject_ip(earlier_ip);

    let term = h.draw().unwrap();
    let screen = term.backend().to_string();

    assert!(
        screen.contains("earlier.local"),
        "newly injected IP should be visible in the table"
    );
    assert!(
        screen.contains("▶ beta.local"),
        "cursor should still be on beta.local after the row shifted"
    );
    assert!(
        screen.contains("alpha.local"),
        "alpha.local should still be visible in the frozen snapshot"
    );

    // The ▶ cursor must not appear on the earlier.local row.
    let earlier_row = screen
        .lines()
        .find(|l| l.contains("earlier.local"))
        .unwrap_or("");
    assert!(
        !earlier_row.contains('▶'),
        "sub-line cursor leaked onto the wrong row after row index shifted"
    );

    h.run(Action::NavigateDown); // move to "copy all"
    h.run(Action::CopyToClipboard); // confirm copy - must not panic

    let term = h.draw().unwrap();
    insta::with_settings!({filters => insta_filters()}, {
        assert_snapshot!("sub_line_selector_after_row_shift_copy", term.backend());
    });
}

/// When additional hostnames are resolved for the *same* IP that is under
/// sub-line selection, the frozen snapshot must remain stable and the selector
/// must stay functional (navigate + copy).
#[test]
fn test_sub_line_selector_stable_when_same_ip_gets_more_names() {
    let mut h = ModelHarness::new(AppConfig::default());
    h.inject_ip(ip_with_names(&["alpha.local", "beta.local"]));
    h.run(Action::NavigateRight);
    h.run(Action::NavigateRight);
    h.run(Action::CopyToClipboard);
    h.run(Action::NavigateDown); // cursor on "beta.local"

    h.inject_ip(ip_with_names(&["alpha.local", "beta.local", "gamma.local"]));

    let term = h.draw().unwrap();
    let screen = term.backend().to_string();

    // Snapshot is frozen: gamma.local must not appear inside the selector.
    assert!(
        !screen.contains("gamma.local"),
        "gamma.local must not appear in the frozen sub-line snapshot"
    );
    assert!(
        screen.contains("▶ beta.local"),
        "cursor must still be on beta.local"
    );
    assert!(
        screen.contains("[ copy all ]"),
        "copy-all option must still be visible"
    );

    h.run(Action::CopyToClipboard);
}

/// When the tracked IP gains an IPv6 address while sub-line selection is open,
/// the IpInfo entry upgrades from V4(x) to V4andV6((x, y)).  The selector was
/// opened with V4(x), so `==` comparison no longer matches - the sub-line UI
/// must stay visible and functional using `shares_ip_with` semantics.
///
/// V4andV6 also sorts after V4 for the same IPv4 address because the tuple
/// (Some(v4), Some(v6)) orders after (Some(v4), None), so the row index
/// shifts at the same time.  Both effects must be handled.
#[test]
fn test_sub_line_selector_survives_ipv6_upgrade() {
    let mut h = ModelHarness::new(AppConfig::default());

    h.inject_ip(ip_with_names(&["alpha.local", "beta.local"]));
    h.run(Action::NavigateRight);
    h.run(Action::NavigateRight);
    h.run(Action::CopyToClipboard);
    h.run(Action::NavigateDown); // cursor on "beta.local"

    // The same host now also has an IPv6 address.  We model this by injecting
    // an IpInfo whose ip is already V4andV6 (the collector would have merged
    // the two protocol addresses before sending).  IpDb finds the existing
    // V4(10.0.0.1) entry via shares_ip_with and merges it, upgrading the
    // stored entry's ip() from V4(10.0.0.1) → V4andV6((10.0.0.1, ::1)).
    let ipv6 = Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1);
    let mut upgrade = IpInfo::from_ip(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)));
    upgrade.ip = IpForHost::V4andV6((Ipv4Addr::new(10, 0, 0, 1), ipv6));
    upgrade.add_name("alpha.local".to_owned());
    upgrade.add_name("beta.local".to_owned());
    h.inject_ip(upgrade);

    let term = h.draw().unwrap();
    let screen = term.backend().to_string();

    assert!(
        screen.contains("▶ beta.local"),
        "sub-line cursor must survive the V4 → V4andV6 upgrade"
    );
    assert!(
        screen.contains("alpha.local"),
        "alpha.local must still appear in the frozen snapshot"
    );
    assert!(
        screen.contains("[ copy all ]"),
        "copy-all option must still be present"
    );

    h.run(Action::NavigateDown); // move to "copy all"
    h.run(Action::CopyToClipboard); // confirm - must not panic
}

/// Navigating down in sub-line mode moves the cursor through snapshot lines
/// and then onto the "copy all" virtual option.
#[test]
fn test_sub_line_selector_navigation() {
    let mut h = ModelHarness::new(AppConfig::default());
    h.inject_ip(ip_with_names(&["alpha.local", "beta.local"]));

    h.run(Action::NavigateRight);
    h.run(Action::NavigateRight);
    h.run(Action::CopyToClipboard);

    h.run(Action::NavigateDown);
    let at_beta = h.draw().unwrap();

    h.run(Action::NavigateDown);
    let at_copy_all = h.draw().unwrap();

    // Cursor should not advance past "copy all"
    h.run(Action::NavigateDown);
    let still_at_copy_all = h.draw().unwrap();

    assert_ne!(
        at_beta.backend().to_string(),
        at_copy_all.backend().to_string(),
        "cursor did not move from beta.local to copy-all"
    );
    assert_eq!(
        at_copy_all.backend().to_string(),
        still_at_copy_all.backend().to_string(),
        "cursor advanced past the copy-all option"
    );

    insta::with_settings!({filters => insta_filters()}, {
        assert_snapshot!("sub_line_selector_at_beta", at_beta.backend());
        assert_snapshot!("sub_line_selector_at_copy_all", at_copy_all.backend());
    });
}

/// Copying a single-line cell bypasses sub-line selection and copies directly.
/// The cell flashes immediately; the screen must return to normal rendering.
#[test]
fn test_single_line_copy_flash() {
    let mut h = ModelHarness::new(AppConfig::default());
    h.inject_ip(ip_with_names(&["only-name.local"]));

    // Navigate to IP column (col 0) - always a single line.
    h.run(Action::NavigateRight);
    h.run(Action::CopyToClipboard);

    let term = h.draw().unwrap();
    insta::with_settings!({filters => insta_filters()}, {
        assert_snapshot!(term.backend());
    });
}

/// After confirming an individual sub-line copy the cell shows a flash
/// on the copied line only, not on the whole cell.
#[test]
fn test_sub_line_individual_copy_flash() {
    let mut h = ModelHarness::new(AppConfig::default());
    h.inject_ip(ip_with_names(&["alpha.local", "beta.local"]));

    h.run(Action::NavigateRight);
    h.run(Action::NavigateRight);
    h.run(Action::CopyToClipboard); // enter sub-line mode, cursor at "alpha.local"
    // Confirm "alpha.local" without moving the cursor.
    h.run(Action::CopyToClipboard);

    let term = h.draw().unwrap();
    insta::with_settings!({filters => insta_filters()}, {
        assert_snapshot!(term.backend());
    });
}

/// Pressing Enter (Action::NavigateSelect) on a row opens the IP info popup.
/// Pressing it again closes the popup.
#[test]
fn test_select_opens_and_closes_ip_info_popup() {
    let mut h = ModelHarness::new(AppConfig::default());
    h.inject_ip(ip_with_names(&["alpha.local"]));

    h.run(Action::NavigateSelect);
    let open = h.draw().unwrap();

    h.run(Action::NavigateSelect);
    let closed = h.draw().unwrap();

    let open_str = open.backend().to_string();
    let closed_str = closed.backend().to_string();

    assert_ne!(
        open_str, closed_str,
        "popup open and closed renders must differ"
    );

    insta::with_settings!({filters => insta_filters()}, {
        assert_snapshot!("ip_info_popup_open", open.backend());
        assert_snapshot!("ip_info_popup_closed", closed.backend());
    });
}

/// Pressing Enter (Action::NavigateSelect) while in sub-line mode confirms the copy.
#[test]
fn test_select_confirms_sub_line_copy() {
    let mut h = ModelHarness::new(AppConfig::default());
    h.inject_ip(ip_with_names(&["alpha.local", "beta.local"]));

    h.run(Action::NavigateRight);
    h.run(Action::NavigateRight);
    h.run(Action::CopyToClipboard); // enter sub-line mode at "alpha.local"
    h.run(Action::NavigateDown); // move to "beta.local"
    h.run(Action::NavigateSelect); // confirm via Enter

    // After confirmation, sub-line mode must be dismissed.
    let term = h.draw().unwrap();
    let screen = term.backend().to_string();
    assert!(
        !screen.contains('▶'),
        "sub-line cursor must be gone after confirmation"
    );
}

/// Verify that sub-line selection navigation and confirmation work correctly
/// even when the search box is open and the table is focused.
#[test]
fn test_sub_line_selection_works_with_search_open() {
    let mut h = ModelHarness::new(AppConfig::default());
    h.inject_ip(ip_with_names(&["alpha.local", "beta.local"]));

    // Open search box
    h.run(Action::Search);
    // Toggle focus to table
    h.run(Action::ToggleFocus);

    // Navigate to Name column (col 1)
    h.run(Action::NavigateRight);
    h.run(Action::NavigateRight);

    // Enter sub-line selection mode
    h.run(Action::CopyToClipboard);

    let screen_before = h.draw().unwrap().backend().to_string();
    assert!(
        screen_before.contains("▶ alpha.local"),
        "Should be in sub-line selection mode at alpha.local"
    );

    // Navigate down to beta.local - should work even with SearchBox open
    h.run(Action::NavigateDown);

    let screen_after_nav = h.draw().unwrap().backend().to_string();
    assert!(
        screen_after_nav.contains("▶ beta.local"),
        "Should have navigated to beta.local within the cell"
    );

    // Confirm selection - should work even with SearchBox open
    h.run(Action::NavigateSelect);

    let screen_after_confirm = h.draw().unwrap().backend().to_string();
    assert!(
        !screen_after_confirm.contains('▶'),
        "Sub-line selection should be closed after NavigateSelect"
    );
}
