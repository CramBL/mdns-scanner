use std::env;
use std::path::Path;

use anyhow::bail;

fn main() -> anyhow::Result<()> {
    // Only apply special handling for Windows targets
    if env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        if try_set_npcap_sdk_path_by_env().is_ok() {
            return Ok(());
        }

        try_find_npcap_sdk()?;
    }
    Ok(())
}

fn try_set_npcap_sdk_path_by_env() -> anyhow::Result<()> {
    // Check if running in GitHub Actions (using the composite action)
    let Ok(npcap_lib) = env::var("LIB") else {
        bail!("$LIB needs to be set to a directory with the npcap SDK");
    };
    println!("cargo:warning=$LIB='{npcap_lib}'");
    if npcap_lib.contains("npcap-sdk") {
        let npcap_lib_path = Path::new(&npcap_lib);
        println!(
            "cargo:warning=npcap-sdk lib at '{}'",
            npcap_lib_path.display()
        );
        if !npcap_lib_path.is_dir() {
            bail!(
                "$LIB is not a directory: LIB='{}'",
                npcap_lib_path.display()
            )
        }

        // We're using the Npcap SDK installed by the composite action
        println!("cargo:rustc-link-search={npcap_lib}");
    } else {
        bail!("$LIB does not contain a reference to npcap-sdk")
    }

    let Ok(npcap_include) = env::var("INCLUDE") else {
        bail!("$INCLUDE needs to be set to a directory with the npcap SDK headers");
    };

    println!("cargo:warning=$INCLUDE='{npcap_lib}'");
    let npcap_include_path = Path::new(&npcap_include);
    println!(
        "cargo:warning=npcap-sdk include at '{}'",
        npcap_include_path.display()
    );
    if npcap_include.contains("npcap-sdk") {
        if !npcap_include_path.is_dir() {
            bail!(
                "$INCLUDE is not a directory: INCLUDE='{}'",
                npcap_include_path.display()
            )
        }
        println!("cargo:include={npcap_include}");
        return Ok(());
    } else {
        bail!("$INCLUDE does not contain a reference to npcap-sdk");
    }
}

fn try_find_npcap_sdk() -> anyhow::Result<()> {
    // Check for local Npcap SDK installation
    let possible_sdk_paths = [
        "C:/Program Files/Npcap/SDK",
        "C:/Program Files (x86)/Npcap/SDK",
        "C:/npcap-sdk",
        "./npcap-sdk",
    ];

    for sdk_path in &possible_sdk_paths {
        let path = Path::new(sdk_path);
        if path.exists() {
            let lib_path = path.join("Lib/x64");
            if lib_path.exists() {
                println!("cargo:rustc-link-search={}", lib_path.display());
                println!("cargo:warning=Using Npcap SDK found at: {sdk_path}");
                return Ok(());
            }
        }
    }

    bail!("Npcap SDK not found. Please install it locally or use the composite action");
}
