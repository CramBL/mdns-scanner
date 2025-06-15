use axoupdater::{AxoUpdater, UpdateRequest};
use color_eyre::eyre::eyre;

use crate::get_app_version;

pub(crate) fn run_self_update(version: Option<String>, dry_run: bool) -> color_eyre::Result<()> {
    tokio::runtime::Builder::new_current_thread()
        .worker_threads(1)
        .max_blocking_threads(128)
        .enable_all()
        .build()
        .expect("Initializing tokio runtime failed")
        .block_on(self_update(version, dry_run))
}

/// Attempt to update the mdns-scanner binary.
pub(crate) async fn self_update(version: Option<String>, dry_run: bool) -> color_eyre::Result<()> {
    let mut updater = AxoUpdater::new_for("mdns-scanner");
    updater.disable_installer_output();

    // Load the "install receipt" for the current binary. If the receipt is not found, then
    // mdns-scanner was likely installed via a package manager.
    let Ok(updater) = updater.load_receipt() else {
        return Err(eyre!(
            "Self-update is only available for mdns-scanner binaries installed via the standalone installation scripts."
        ));
    };

    // If we know what our version is, ignore whatever the receipt thinks it is!
    // This makes us behave better if someone manually installs a random version of mdns-scanner
    // in a way that doesn't update the receipt.
    // This is best-effort, it's fine if it fails (also it can't actually fail)
    let _ = updater.set_current_version(get_app_version().clone());

    // Ensure the receipt is for the current binary. If it's not, then the user likely has multiple
    // mdns-scanner binaries installed, and the current binary was _not_ installed via the standalone
    // installation scripts.
    if !updater.check_receipt_is_for_this_executable()? {
        let current_exe = std::env::current_exe()?;
        let receipt_prefix = updater.install_prefix_root()?;
        let current_exe_path = current_exe.display();
        let receipt_prefix_path = receipt_prefix.to_string();
        let hint = format!(
            "The current executable is at `{current_exe_path}` but the standalone installer was used to install mdns-scanner to `{receipt_prefix_path}`. Are multiple copies of mdns-scanner installed?"
        );
        let report = eyre!(
            "Self-update is only available for mdns-scanner binaries installed via the standalone installation scripts.\n{hint}"
        );

        return Err(report);
    }

    eprintln!("Checking for updates...");

    let update_request = if let Some(version) = version {
        UpdateRequest::SpecificTag(version)
    } else {
        UpdateRequest::Latest
    };

    updater.configure_version_specifier(update_request.clone());

    if dry_run {
        if updater.is_update_needed().await? {
            let version = match update_request {
                UpdateRequest::Latest | UpdateRequest::LatestMaybePrerelease => {
                    "the latest version".to_string()
                }
                UpdateRequest::SpecificTag(version) | UpdateRequest::SpecificVersion(version) => {
                    format!("v{version}")
                }
            };
            eprintln!(
                "Would update mdns-scanner from v{} to {version}",
                get_app_version(),
            );
        } else {
            eprintln!(
                "{}",
                format_args!(
                    "You're on the latest version of mdns-scanner (v{})",
                    get_app_version()
                )
            );
        }
        return Ok(());
    }

    match updater.run().await {
        Ok(Some(result)) => {
            let direction = if result
                .old_version
                .as_ref()
                .is_some_and(|old_version| *old_version > result.new_version)
            {
                "Downgraded"
            } else {
                "Upgraded"
            };

            let version_information = if let Some(old_version) = result.old_version {
                format!("from v{old_version} to v{}", result.new_version,)
            } else {
                format!("to v{}", result.new_version)
            };

            eprintln!(
                "success: {direction} mdns-scanner {version_information}! https://github.com/CramBL/mdns-scanner/releases/tag/{}",
                result.new_version_tag
            );
        }
        Ok(None) => {
            eprintln!(
                "success: You're on the latest version of mdns-scanner (v{})",
                get_app_version()
            );
        }
        Err(err) => {
            return Err(err.into());
        }
    }

    Ok(())
}
