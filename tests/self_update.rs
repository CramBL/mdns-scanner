use std::{path::PathBuf, process::Command};

use assert_cmd::prelude::*;
use axoupdater::{
    ReleaseSourceType,
    test::helpers::{RuntestArgs, perform_runtest},
};
use testresult::TestResult;

/// Returns the mdns-scanner binary that cargo built before launching the tests.
///
/// <https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-crates>
pub fn get_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_mdns-scanner"))
}

const APP_NAME: &str = env!("CARGO_PKG_NAME");

#[test]
fn test_self_update_dry_run() -> TestResult {
    let mut cmd = Command::cargo_bin(APP_NAME)?;
    cmd.args(["update", "--dry-run"]);

    let status = cmd.status().expect("Failed to run update --dry-run");

    // TODO: Flip the assertion when the first cargo-dist release is done
    assert!(
        !status.success(),
        "'mdns-scanner update --dry-run' returned non-zero"
    );
    Ok(())
}

#[test]
fn test_self_update_ci() {
    // To maximally emulate behaviour in practice, this test actually modifies CARGO_HOME
    // and therefore should only be run in CI by default, where it can't hurt developers.
    // We use the "CI" env-var that CI machines tend to run
    if std::env::var("CI").map(|s| s.is_empty()).unwrap_or(true) {
        return;
    }

    // Configure the runtest
    let args = RuntestArgs {
        app_name: "mdns-scanner".to_owned(),
        package: "mdns-scanner".to_owned(),
        owner: "CramBL".to_owned(),
        bin: get_bin(),
        binaries: vec!["mdns-scanner".to_owned()],
        args: vec!["update".to_owned()],
        release_type: ReleaseSourceType::GitHub,
    };

    // install and update the application
    let installed_bin = perform_runtest(&args);

    // check that the binary works like normal
    let status = Command::new(installed_bin)
        .arg("--version")
        .status()
        .expect("failed to run 'mdns-scanner --version'");
    assert!(
        status.success(),
        "'mdns-scanner --version' returned non-zero"
    );
}
