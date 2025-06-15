#[cfg(windows)]
use {
    anyhow::{Context, bail},
    std::env,
    std::path::PathBuf,
    std::{
        fs::{self, File},
        path::Path,
    },
    zip::ZipArchive,
};

#[cfg(windows)]
fn main() -> anyhow::Result<()> {
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH")?;
    let subdir = match target_arch.as_str() {
        "x86_64" => "x64",
        "aarch64" => "ARM64",
        "x86" => "", // use top-level Lib for 32-bit
        _ => bail!("Unsupported architecture: {}", target_arch),
    };

    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    let sdk_path = out_dir.join("npcap-sdk");
    let lib_path = if subdir.is_empty() {
        sdk_path.join("Lib")
    } else {
        sdk_path.join("Lib").join(subdir)
    };
    let include_path = sdk_path.join("Include");

    if !sdk_path.exists() {
        download_and_extract_npcap_sdk(&sdk_path, &out_dir)?;
    }

    for lib in ["Packet.lib", "wpcap.lib"] {
        let lib_file = lib_path.join(lib);
        if !lib_file.exists() {
            bail!("Expected '{}' in '{}'", lib, lib_path.display());
        }
    }

    println!("cargo:rustc-link-search={}", lib_path.display());
    println!("cargo:rustc-link-lib=Packet");
    println!("cargo:rustc-link-lib=wpcap");
    println!("cargo:include={}", include_path.display());
    println!("cargo:rustc-env=NPCAP_LIB_PATH={}", lib_path.display());

    Ok(())
}

#[cfg(windows)]
fn download_and_extract_npcap_sdk(dest_dir: &Path, out_dir: &Path) -> anyhow::Result<()> {
    let version = "1.15";
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
