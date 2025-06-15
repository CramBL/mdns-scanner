#[cfg(windows)]
fn main() -> anyhow::Result<()> {
    use anyhow::{Context, Result, bail};
    use std::env;
    use std::fs;
    use std::path::{Path, PathBuf};

    let target_os = env::var("CARGO_CFG_TARGET_OS")?;
    if target_os != "windows" {
        return Ok(());
    }

    let target_arch = env::var("CARGO_CFG_TARGET_ARCH")?;
    let lib_dir = match target_arch.as_str() {
        "x86_64" | "x86" => "x64",
        "aarch64" => "arm64",
        _ => bail!("Unsupported architecture: {target_arch}"),
    };

    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    let sdk_path = out_dir.join("npcap-sdk");
    let lib_path = sdk_path.join("Lib").join(lib_dir);
    let include_path = sdk_path.join("Include");

    if !sdk_path.exists() {
        download_and_extract_npcap_sdk(&sdk_path, &out_dir)?;
    }

    let packet_lib = lib_path.join("Packet.lib");
    if !packet_lib.exists() {
        bail!("Packet.lib not found at '{}'", packet_lib.display());
    }

    if !include_path.is_dir() {
        bail!(
            "Npcap Include directory not found at '{}'",
            include_path.display()
        );
    }

    println!("cargo:rustc-link-search={}", lib_path.display());
    println!("cargo:rustc-link-lib=Packet");
    println!("cargo:include={}", include_path.display());

    Ok(())
}

#[cfg(windows)]
fn download_and_extract_npcap_sdk(dest_dir: &Path, out_dir: &Path) -> anyhow::Result<()> {
    use std::fs::File;
    use zip::ZipArchive;

    let version = "1.13";
    let url = format!("https://npcap.com/dist/npcap-sdk-{version}.zip");
    let zip_path = out_dir.join("npcap-sdk.zip");

    println!("cargo:warning=Downloading Npcap SDK from {}", url);
    let response = reqwest::blocking::get(&url).context("Failed to download Npcap SDK")?;
    let bytes = response
        .bytes()
        .context("Failed to read Npcap SDK response")?;
    fs::write(&zip_path, &bytes).context("Failed to save Npcap SDK zip")?;

    let file = File::open(&zip_path)?;
    let mut archive = ZipArchive::new(file).context("Failed to read zip archive")?;
    archive
        .extract(dest_dir)
        .context("Failed to extract Npcap SDK")?;

    println!(
        "cargo:warning=Npcap SDK extracted to '{}'",
        dest_dir.display()
    );
    Ok(())
}

#[cfg(not(windows))]
fn main() {
    // No-op on non-Windows targets
}
