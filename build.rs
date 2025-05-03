use std::env;
use std::path::Path;

fn main() {
    // Only apply special handling for Windows targets
    if env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        // Check if running in GitHub Actions (using the composite action)
        if let Ok(npcap_lib) = env::var("LIB") {
            if npcap_lib.contains("npcap-sdk") {
                // We're using the Npcap SDK installed by the composite action
                println!("cargo:rustc-link-search={npcap_lib}");

                if let Ok(npcap_include) = env::var("INCLUDE") {
                    if npcap_include.contains("npcap-sdk") {
                        println!("cargo:include={npcap_include}");
                    }
                }
                return;
            }
        }

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
                    return;
                }
            }
        }

        panic!("Npcap SDK not found. Please install it locally or use the composite action");
    }
}
