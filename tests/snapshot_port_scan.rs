// The test tries running the binary and that won't work on windows due to missing npcap dll
#![cfg(not(target_os = "windows"))]

mod common;

use common::{ModelHarness, insta_filters, make_ip};
use insta::assert_snapshot;
use mds_config::AppConfig;
use mds_config::scan::IoThreads;
use mds_ipinfo::IanaPortCategory;
use mds_ipinfo::port_scan_result::MdnsProbeOutcome;
use mds_keybindings::Action;
use mds_netscan::port_scan::{PortOutcome, PortScanUpdate};
use mds_tui::message::Message;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::net::{IpAddr, Ipv4Addr};
use std::num::NonZero;
use std::sync::mpsc::Sender;

const NAS_IP: IpAddr = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 3));

fn harness_with_one_host() -> ModelHarness<'static, 'static, 'static> {
    // A fixed port-scan thread count keeps the worker count in rendered headers
    // deterministic across machines, since `dynamic` resolves to host capacity.
    let mut cfg = AppConfig::default();
    cfg.scan.port_scan_io_threads = IoThreads::Fixed(NonZero::new(64).unwrap());
    let mut h = ModelHarness::new(cfg);
    h.inject_ip(make_ip(NAS_IP, &["nas.local"], &[], 12));
    h
}

fn type_text(h: &mut ModelHarness, text: &str) {
    for c in text.chars() {
        h.run(Message::BoxInput(KeyEvent::new(
            KeyCode::Char(c),
            KeyModifiers::empty(),
        )));
    }
}

/// Moves the setup selection down to the custom port row.
fn focus_custom_port_row(h: &mut ModelHarness) {
    for _ in 0..3 {
        h.run(Action::NavigateDown);
    }
}

fn report_scan(tx: &Sender<PortScanUpdate>, outcomes: &[(u16, PortOutcome)]) {
    for (port, outcome) in outcomes {
        tx.send(PortScanUpdate::PortScanned {
            port: *port,
            outcome: *outcome,
        })
        .expect("port scan update channel unexpectedly closed");
    }
}

/// The header's two lines, as rendered inside the dialog. Box-drawing characters
/// are multi-byte, so the popup borders are located by character index; the text
/// between the popup's left and right borders is returned.
fn settings_header_block(h: &mut ModelHarness) -> String {
    let rendered = h.draw().unwrap().backend().to_string();
    let rows: Vec<Vec<char>> = rendered
        .lines()
        .map(|line| line.chars().collect())
        .collect();
    let metrics_row = rows
        .iter()
        .position(|row| row.iter().collect::<String>().contains("workers"))
        .expect("metrics header row");
    let dialog_text = |row: &[char]| {
        let bars: Vec<usize> = row
            .iter()
            .enumerate()
            .filter(|(_, c)| **c == '│')
            .map(|(index, _)| index)
            .collect();
        row[bars[1] + 1..bars[2]]
            .iter()
            .collect::<String>()
            .trim()
            .to_owned()
    };
    format!(
        "{}\n{}",
        dialog_text(&rows[metrics_row - 1]),
        dialog_text(&rows[metrics_row])
    )
}

/// The number of terminal rows the port-scan dialog box spans, top border to
/// bottom border inclusive.
fn dialog_height(h: &mut ModelHarness) -> usize {
    let rendered = h.draw().unwrap().backend().to_string();
    let rows: Vec<&str> = rendered.lines().collect();
    let top = rows
        .iter()
        .position(|row| row.contains('╭'))
        .expect("dialog top border");
    let bottom = rows
        .iter()
        .rposition(|row| row.contains('╰'))
        .expect("dialog bottom border");
    bottom - top + 1
}

#[test]
fn test_port_scan_setup_renders() {
    let mut h = harness_with_one_host();
    h.run(Action::PortScan);
    let term = h.draw().unwrap();
    insta::with_settings!({filters => insta_filters()}, {
        assert_snapshot!(term.backend());
    });
}

#[test]
fn test_port_scan_setup_with_valid_custom_ports() {
    let mut h = harness_with_one_host();
    h.run(Action::PortScan);
    focus_custom_port_row(&mut h);
    type_text(&mut h, "22,80,8000-8100");
    h.run(Action::NavigateSelect);
    let term = h.draw().unwrap();
    insta::with_settings!({filters => insta_filters()}, {
        assert_snapshot!(term.backend());
    });
}

#[test]
fn test_port_scan_setup_with_invalid_custom_ports() {
    let mut h = harness_with_one_host();
    h.run(Action::PortScan);
    focus_custom_port_row(&mut h);
    type_text(&mut h, "22,70000");
    h.run(Action::NavigateSelect);
    let term = h.draw().unwrap();
    insta::with_settings!({filters => insta_filters()}, {
        assert_snapshot!(term.backend());
    });
}

/// Two IANA range checkboxes combine into one deduplicated, sorted union.
#[test]
fn test_port_scan_setup_with_ranges_checked() {
    let mut h = harness_with_one_host();
    h.run(Action::PortScan);
    h.run(Action::NavigateSelect); // System ports
    h.run(Action::NavigateDown);
    h.run(Action::NavigateDown);
    h.run(Action::NavigateSelect); // Dynamic ports
    let term = h.draw().unwrap();
    insta::with_settings!({filters => insta_filters()}, {
        assert_snapshot!(term.backend());
    });
}

/// With only the mDNS probe checked, the scan can still start.
#[test]
fn test_port_scan_setup_with_mdns_probe_only() {
    let mut h = harness_with_one_host();
    h.run(Action::PortScan);
    for _ in 0..4 {
        h.run(Action::NavigateDown);
    }
    h.run(Action::NavigateSelect); // Also probe mDNS
    let term = h.draw().unwrap();
    insta::with_settings!({filters => insta_filters()}, {
        assert_snapshot!(term.backend());
    });
}

/// A responding mDNS host is shown on its own result line, separate from the
/// open TCP ports.
#[test]
fn test_port_scan_results_with_mdns_probe() {
    let mut h = harness_with_one_host();
    h.run(Action::PortScan);
    let tx = h
        .model
        .start_stubbed_port_scan(vec![IanaPortCategory::System]);
    report_scan(&tx, &[(22, PortOutcome::Open)]);
    tx.send(PortScanUpdate::MdnsProbed(
        MdnsProbeOutcome::UnicastAndMulticast,
    ))
    .unwrap();
    tx.send(PortScanUpdate::Finished).unwrap();
    h.model.recv_port_scan_updates();

    let term = h.draw().unwrap();
    insta::with_settings!({filters => insta_filters()}, {
        assert_snapshot!(term.backend());
    });
}

/// Open ports are listed in numeric order regardless of the order they are found in.
#[test]
fn test_port_scan_in_progress_renders() {
    let mut h = harness_with_one_host();
    h.run(Action::PortScan);
    let tx = h
        .model
        .start_stubbed_port_scan(vec![IanaPortCategory::System]);
    report_scan(
        &tx,
        &[
            (443, PortOutcome::Open),
            (81, PortOutcome::Closed),
            (22, PortOutcome::Open),
            (82, PortOutcome::Filtered),
        ],
    );
    h.model.recv_port_scan_updates();

    let term = h.draw().unwrap();
    insta::with_settings!({filters => insta_filters()}, {
        assert_snapshot!(term.backend());
    });
}

/// A single scanned category yields one table with no category header, sorted
/// regardless of the order ports were found in.
#[test]
fn test_port_scan_results_render() {
    let mut h = harness_with_one_host();
    h.run(Action::PortScan);
    let tx = h
        .model
        .start_stubbed_port_scan(vec![IanaPortCategory::System]);
    report_scan(
        &tx,
        &[
            (443, PortOutcome::Open),
            (22, PortOutcome::Open),
            (80, PortOutcome::Open),
            (81, PortOutcome::Closed),
            (82, PortOutcome::Filtered),
        ],
    );
    tx.send(PortScanUpdate::Finished).unwrap();
    h.model.recv_port_scan_updates();

    let term = h.draw().unwrap();
    insta::with_settings!({filters => insta_filters()}, {
        assert_snapshot!(term.backend());
    });
}

/// Two scanned categories each get a headed two-column table.
#[test]
fn test_port_scan_multi_category_results_render() {
    let mut h = harness_with_one_host();
    h.run(Action::PortScan);
    let tx = h
        .model
        .start_stubbed_port_scan(vec![IanaPortCategory::System, IanaPortCategory::Dynamic]);
    report_scan(
        &tx,
        &[
            (22, PortOutcome::Open),
            (80, PortOutcome::Open),
            (443, PortOutcome::Open),
            (631, PortOutcome::Open),
            (993, PortOutcome::Open),
            (49152, PortOutcome::Open),
            (51413, PortOutcome::Open),
        ],
    );
    tx.send(PortScanUpdate::Finished).unwrap();
    h.model.recv_port_scan_updates();

    let term = h.draw().unwrap();
    insta::with_settings!({filters => insta_filters()}, {
        assert_snapshot!(term.backend());
    });
}

/// More open ports than fit in the view scroll under a scrollbar.
#[test]
fn test_port_scan_results_scrollbar_render() {
    let mut h = harness_with_one_host();
    h.run(Action::PortScan);
    let tx = h
        .model
        .start_stubbed_port_scan(vec![IanaPortCategory::System, IanaPortCategory::Dynamic]);
    let system = [
        21, 22, 23, 25, 53, 67, 69, 80, 110, 111, 123, 135, 139, 143, 161, 389, 443, 445,
    ];
    let dynamic = [49152, 49153];
    for port in system.into_iter().chain(dynamic) {
        report_scan(&tx, &[(port, PortOutcome::Open)]);
    }
    tx.send(PortScanUpdate::Finished).unwrap();
    h.model.recv_port_scan_updates();

    let term = h.draw().unwrap();
    let rendered = term.backend().to_string();
    assert!(
        rendered.contains("rescan"),
        "the results footer must stay visible when the open-port list overflows:\n{rendered}"
    );
    insta::with_settings!({filters => insta_filters()}, {
        assert_snapshot!(term.backend());
    });
}

/// Navigating down scrolls the grouped results list so later ports come into view.
#[test]
fn test_port_scan_results_scrolled_down_render() {
    let mut h = harness_with_one_host();
    h.run(Action::PortScan);
    let tx = h
        .model
        .start_stubbed_port_scan(vec![IanaPortCategory::System, IanaPortCategory::Dynamic]);
    let system = [
        21, 22, 23, 25, 53, 67, 69, 80, 110, 111, 123, 135, 139, 143, 161, 389, 443, 445,
    ];
    let dynamic = [49152, 49153];
    for port in system.into_iter().chain(dynamic) {
        report_scan(&tx, &[(port, PortOutcome::Open)]);
    }
    tx.send(PortScanUpdate::Finished).unwrap();
    h.model.recv_port_scan_updates();
    for _ in 0..6 {
        h.run(Action::NavigateDown);
    }

    let term = h.draw().unwrap();
    let rendered = term.backend().to_string();
    assert!(
        rendered.contains("rescan"),
        "the results footer must stay visible when the open-port list overflows:\n{rendered}"
    );
    insta::with_settings!({filters => insta_filters()}, {
        assert_snapshot!(term.backend());
    });
}

#[test]
fn test_port_scan_results_without_open_ports_render() {
    let mut h = harness_with_one_host();
    h.run(Action::PortScan);
    let tx = h
        .model
        .start_stubbed_port_scan(vec![IanaPortCategory::System]);
    report_scan(
        &tx,
        &[(81, PortOutcome::Closed), (82, PortOutcome::Filtered)],
    );
    tx.send(PortScanUpdate::Finished).unwrap();
    h.model.recv_port_scan_updates();

    let term = h.draw().unwrap();
    insta::with_settings!({filters => insta_filters()}, {
        assert_snapshot!(term.backend());
    });
}

/// Escape during a scan keeps what was found so far and marks it cancelled. The
/// full range was scanned, so the one open port's category is still headed.
#[test]
fn test_cancelled_port_scan_results_render() {
    let mut h = harness_with_one_host();
    h.run(Action::PortScan);
    let tx = h.model.start_stubbed_port_scan(vec![
        IanaPortCategory::System,
        IanaPortCategory::User,
        IanaPortCategory::Dynamic,
    ]);
    report_scan(&tx, &[(22, PortOutcome::Open), (81, PortOutcome::Closed)]);
    h.model.recv_port_scan_updates();
    h.run(Action::Close);

    let term = h.draw().unwrap();
    insta::with_settings!({filters => insta_filters()}, {
        assert_snapshot!(term.backend());
    });
}

#[test]
fn test_host_info_popup_shows_last_port_scan_result() {
    let mut h = harness_with_one_host();
    h.run(Action::PortScan);
    let tx = h
        .model
        .start_stubbed_port_scan(vec![IanaPortCategory::System, IanaPortCategory::User]);
    report_scan(
        &tx,
        &[
            (53, PortOutcome::Open),
            (443, PortOutcome::Open),
            (8009, PortOutcome::Open),
        ],
    );
    tx.send(PortScanUpdate::Finished).unwrap();
    h.model.recv_port_scan_updates();
    h.run(Action::Close);
    h.run(Action::NavigateSelect);

    let term = h.draw().unwrap();
    insta::with_settings!({filters => insta_filters()}, {
        assert_snapshot!(term.backend());
    });
}

#[test]
fn test_reopening_the_dialog_shows_the_previous_result() {
    let mut h = harness_with_one_host();
    h.run(Action::PortScan);
    let tx = h
        .model
        .start_stubbed_port_scan(vec![IanaPortCategory::System]);
    report_scan(&tx, &[(22, PortOutcome::Open)]);
    tx.send(PortScanUpdate::Finished).unwrap();
    h.model.recv_port_scan_updates();
    h.run(Action::Close);
    h.run(Action::PortScan);

    let rendered = h.draw().unwrap().backend().to_string();
    assert!(
        rendered.contains("1 open port"),
        "reopened dialog did not show the previous result:\n{rendered}"
    );
}

/// The settings header is frozen at scan start and shown unchanged while the scan
/// runs and after it finishes.
#[test]
fn test_settings_header_is_identical_across_states() {
    let mut h = harness_with_one_host();
    h.run(Action::PortScan);
    h.run(Action::NavigateSelect); // System ports
    h.run(Action::NavigateDown);
    h.run(Action::NavigateDown);
    h.run(Action::NavigateSelect); // Dynamic ports
    let setup_header = settings_header_block(&mut h);

    let tx = h
        .model
        .start_stubbed_port_scan(vec![IanaPortCategory::System, IanaPortCategory::Dynamic]);
    let scanning_header = settings_header_block(&mut h);

    tx.send(PortScanUpdate::Finished).unwrap();
    h.model.recv_port_scan_updates();
    let results_header = settings_header_block(&mut h);

    assert_eq!(setup_header, scanning_header);
    assert_eq!(scanning_header, results_header);
    insta::assert_snapshot!(setup_header, @"
    1-1023, 49152-65535
    64 workers   100ms   max 27.2s
    ");
}

/// Starting a scan must not resize the dialog: setup and scanning share a height.
#[test]
fn test_setup_and_scanning_dialogs_have_equal_height() {
    let mut h = harness_with_one_host();
    h.run(Action::PortScan);
    h.run(Action::NavigateSelect); // System ports
    let setup_height = dialog_height(&mut h);

    h.model
        .start_stubbed_port_scan(vec![IanaPortCategory::System]);
    let scanning_height = dialog_height(&mut h);

    assert_eq!(setup_height, scanning_height);
}

/// The dialog stays pinned to the host it was opened on: navigation keys must
/// not move the table selection behind it.
#[test]
fn test_table_selection_is_frozen_while_the_dialog_is_open() {
    let mut h = ModelHarness::new(AppConfig::default());
    h.inject_ip(make_ip(NAS_IP, &["nas.local"], &[], 12));
    h.inject_ip(make_ip(
        IpAddr::V4(Ipv4Addr::new(10, 0, 0, 9)),
        &["printer.local"],
        &[],
        4,
    ));

    h.run(Action::PortScan);
    for _ in 0..3 {
        h.run(Action::NavigateDown);
    }
    h.run(Action::NavigateScrollToEnd);
    h.run(Action::Close);
    h.run(Action::PortScan);

    let rendered = h.draw().unwrap().backend().to_string();
    assert!(
        rendered.contains("Port scan 10.0.0.3 (nas.local)"),
        "table selection moved while the port scan dialog was open:\n{rendered}"
    );
}

#[test]
fn test_dialog_does_not_open_without_a_selected_host() {
    let mut h = ModelHarness::new(AppConfig::default());
    h.run(Action::PortScan);

    let rendered = h.draw().unwrap().backend().to_string();
    assert!(
        !rendered.contains("Port scan"),
        "port scan dialog opened without a host to scan:\n{rendered}"
    );
}

/// Refresh is the rescan key while the dialog is open, so it must not clear the
/// table the way a global refresh does.
#[test]
fn test_refresh_does_not_clear_the_table_while_the_dialog_is_open() {
    let mut h = harness_with_one_host();
    h.run(Action::PortScan);
    h.run(Action::Refresh);
    h.model.recv_new_ip_info();

    let rendered = h.draw().unwrap().backend().to_string();
    assert!(
        rendered.contains("1 IPs discovered"),
        "refresh cleared the table while the port scan dialog was open:\n{rendered}"
    );
}

#[test]
fn test_refresh_clears_the_table_when_the_dialog_is_closed() {
    let mut h = harness_with_one_host();
    h.run(Action::Refresh);
    h.model.recv_new_ip_info();

    let rendered = h.draw().unwrap().backend().to_string();
    assert!(
        rendered.contains("0 IPs discovered"),
        "global refresh did not clear the table:\n{rendered}"
    );
}

/// The metrics line shows an em dash for the maximum duration when nothing
/// is selected.
#[test]
fn test_setup_header_shows_no_max_duration_when_nothing_is_selected() {
    let mut h = harness_with_one_host();
    h.run(Action::PortScan);
    let header = settings_header_block(&mut h);
    assert!(
        header.contains("max —"),
        "expected a dashed max in the nothing-selected header:\n{header}"
    );
}

/// Focusing the empty custom-port row overlays the text cursor on the example's
/// first character without shifting the example to the right.
#[test]
fn test_port_scan_setup_custom_row_focused_keeps_placeholder_in_place() {
    let mut h = harness_with_one_host();
    h.run(Action::PortScan);
    focus_custom_port_row(&mut h);
    let term = h.draw().unwrap();
    insta::with_settings!({filters => insta_filters()}, {
        assert_snapshot!(term.backend());
    });
}

/// Before the first port reports, the scanning view already shows the elapsed
/// ticker above an empty progress bar. The stubbed scan's fixed elapsed time
/// pins the ticker to a deterministic value.
#[test]
fn test_port_scan_shows_the_ticker_before_the_first_progress_update() {
    let mut h = harness_with_one_host();
    h.run(Action::PortScan);
    h.model
        .start_stubbed_port_scan(vec![IanaPortCategory::System]);

    let term = h.draw().unwrap();
    insta::with_settings!({filters => insta_filters()}, {
        assert_snapshot!(term.backend());
    });
}

/// A rescan from results keeps the dialog at the results height.
#[test]
fn test_rescanning_keeps_the_dialog_at_the_results_height() {
    let mut h = harness_with_one_host();
    h.run(Action::PortScan);
    let tx = h
        .model
        .start_stubbed_port_scan(vec![IanaPortCategory::System, IanaPortCategory::Dynamic]);
    for port in [22, 80, 443, 631, 49152, 49153, 49154, 49155] {
        report_scan(&tx, &[(port, PortOutcome::Open)]);
    }
    tx.send(PortScanUpdate::Finished).unwrap();
    h.model.recv_port_scan_updates();
    let results_height = dialog_height(&mut h);

    h.model
        .start_stubbed_port_scan(vec![IanaPortCategory::System, IanaPortCategory::Dynamic]);
    let rescanning_height = dialog_height(&mut h);

    assert_eq!(results_height, rescanning_height);
}

/// A fixed port-scan thread count is honoured end to end: the header reports
/// the configured worker count.
#[test]
fn test_port_scan_uses_the_configured_port_scan_thread_count() {
    let mut cfg = AppConfig::default();
    cfg.scan.port_scan_io_threads = IoThreads::Fixed(NonZero::new(40).unwrap());
    let mut h = ModelHarness::new(cfg);
    h.inject_ip(make_ip(NAS_IP, &["nas.local"], &[], 12));

    h.run(Action::PortScan);
    let header = settings_header_block(&mut h);
    assert!(
        header.contains("40 workers"),
        "expected the configured worker count in the header:\n{header}"
    );
}

fn header_worker_count(h: &mut ModelHarness) -> u16 {
    let header = settings_header_block(h);
    let metrics = header.lines().last().expect("metrics header line");
    metrics
        .split_whitespace()
        .next()
        .and_then(|n| n.parse().ok())
        .unwrap_or_else(|| panic!("no worker count in header line: {metrics:?}"))
}

/// A Port Scan Threads config change made while the dialog stays open reaches the
/// open dialog: it updates the setup header immediately and drives the next scan
/// that starts.
#[test]
fn test_port_scan_thread_count_config_change_reaches_open_dialog() {
    let mut cfg = AppConfig::default();
    cfg.scan.port_scan_io_threads = IoThreads::Fixed(NonZero::new(512).unwrap());
    let mut h = ModelHarness::new(cfg);
    h.inject_ip(make_ip(NAS_IP, &["nas.local"], &[], 12));

    h.run(Action::PortScan);
    assert_eq!(header_worker_count(&mut h), 512);

    h.cfg.modify(|c| {
        c.scan.port_scan_io_threads = IoThreads::Fixed(NonZero::new(12).unwrap());
    });
    h.model.recv_port_scan_updates();
    assert_eq!(header_worker_count(&mut h), 12);

    let tx = h
        .model
        .start_stubbed_port_scan(vec![IanaPortCategory::System]);
    tx.send(PortScanUpdate::Finished).unwrap();
    h.model.recv_port_scan_updates();
    assert_eq!(header_worker_count(&mut h), 12);
}
