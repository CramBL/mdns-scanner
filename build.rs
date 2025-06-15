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

fn get_lib_subdir() -> &'static str {
    // Get the target architecture
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();

    match target_arch.as_str() {
        "aarch64" => "LIBarm64",
        "x86_64" | "x86" => "LIBx64",
        _ => "LIBx64", // Default to x64 for unknown architectures
    }
}

fn try_set_npcap_sdk_path_by_env() -> anyhow::Result<()> {
    // Check if running in GitHub Actions (using the composite action)
    // The action sets architecture-specific environment variables
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();

    let (lib_env_var, npcap_lib) = match target_arch.as_str() {
        "aarch64" => {
            let lib = env::var("LIBarm64").or_else(|_| env::var("LIB"));
            ("LIBarm64", lib)
        }
        "x86_64" | "x86" | _ => {
            let lib = env::var("LIBx64").or_else(|_| env::var("LIB"));
            ("LIBx64", lib)
        }
    };

    let Ok(npcap_lib) = npcap_lib else {
        bail!("${lib_env_var} or $LIB needs to be set to a directory with the npcap SDK");
    };

    println!("cargo:warning=${lib_env_var}='{npcap_lib}'");
    if npcap_lib.contains("npcap-sdk") {
        let npcap_lib_path = Path::new(&npcap_lib);
        println!(
            "cargo:warning=npcap-sdk lib at '{}'",
            npcap_lib_path.display()
        );
        if !npcap_lib_path.is_dir() {
            bail!(
                "${lib_env_var} is not a directory: {lib_env_var}='{}'",
                npcap_lib_path.display()
            )
        }
        // Verify that Packet.lib exists in the directory
        let packet_lib_path = npcap_lib_path.join("Packet.lib");
        if !packet_lib_path.exists() {
            bail!(
                "Packet.lib not found in ${lib_env_var} directory: expected at '{}'",
                packet_lib_path.display()
            );
        }

        // We're using the Npcap SDK installed by the composite action
        println!("cargo:rustc-link-search={npcap_lib}");
    } else {
        bail!("${lib_env_var} does not contain a reference to npcap-sdk")
    }

    let Ok(npcap_include) = env::var("INCLUDE") else {
        bail!("$INCLUDE needs to be set to a directory with the npcap SDK headers");
    };
    println!("cargo:warning=$INCLUDE='{npcap_include}'");
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

    let lib_subdir = get_lib_subdir();

    for sdk_path in &possible_sdk_paths {
        let path = Path::new(sdk_path);
        if path.exists() {
            let lib_path = path.join("Lib").join(lib_subdir);
            if lib_path.exists() {
                // Verify that Packet.lib exists in the directory
                let packet_lib_path = lib_path.join("Packet.lib");
                if !packet_lib_path.exists() {
                    println!(
                        "cargo:warning=Skipping {sdk_path}: Packet.lib not found in {lib_subdir} directory"
                    );
                    continue;
                }

                println!("cargo:rustc-link-search={}", lib_path.display());
                println!(
                    "cargo:warning=Using Npcap SDK found at: {sdk_path} (architecture: {lib_subdir})"
                );
                return Ok(());
            }
        }
    }

    bail!("Npcap SDK not found. Please install it locally or use the composite action");
}
