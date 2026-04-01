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
const NPCAP_SDK_VERSION: &str = "1.16";

#[cfg(windows)]
fn main() -> anyhow::Result<()> {
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH")?;
    let subdir = match target_arch.as_str() {
        "x86_64" => "x64",
        "aarch64" => "ARM64",
        "x86" => "", // use top-level Lib for 32-bit
        _ => bail!("Unsupported architecture: {}", target_arch),
    };

    let cache_dir = get_cache_dir()?;
    let sdk_path = cache_dir.join("npcap-sdk");
    let lib_path = if subdir.is_empty() {
        sdk_path.join("Lib")
    } else {
        sdk_path.join("Lib").join(subdir)
    };
    let include_path = sdk_path.join("Include");

    if !is_sdk_complete(&sdk_path, &lib_path) {
        download_and_extract_npcap_sdk(&sdk_path, &cache_dir)?;
    }

    for lib in ["Packet.lib", "wpcap.lib"] {
        let lib_file = lib_path.join(lib);
        if !lib_file.exists() {
            bail!("Expected '{lib}' in '{}'", lib_path.display());
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
fn get_cache_dir() -> anyhow::Result<PathBuf> {
    // Try to use a stable cache directory
    if let Ok(cache_root) = env::var("CARGO_TARGET_DIR") {
        // Use target dir if available (persists across builds)
        return Ok(PathBuf::from(cache_root).join("npcap-cache"));
    }

    if let Ok(home) = env::var("USERPROFILE") {
        // Fallback to user profile cache
        return Ok(PathBuf::from(home).join(".cargo-cache").join("npcap"));
    }

    // Last resort: use OUT_DIR but warn about it
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    println!("cargo:warning=Using OUT_DIR for Npcap cache - may cause frequent re-downloads");
    Ok(out_dir)
}

#[cfg(windows)]
fn is_sdk_complete(sdk_path: &Path, lib_path: &Path) -> bool {
    if !sdk_path.exists() {
        return false;
    }

    // Check for version marker to detect incomplete/outdated installations
    let version_file = sdk_path.join(".version");
    if let Ok(installed_version) = fs::read_to_string(&version_file) {
        if installed_version.trim() != NPCAP_SDK_VERSION {
            return false;
        }
    } else {
        return false;
    }

    // Verify key files exist
    let required_files = [
        lib_path.join("Packet.lib"),
        lib_path.join("wpcap.lib"),
        sdk_path.join("Include").join("pcap.h"),
    ];

    required_files.iter().all(|f| f.exists())
}

#[cfg(windows)]
fn download_and_extract_npcap_sdk(dest_dir: &Path, cache_dir: &Path) -> anyhow::Result<()> {
    let url = format!("https://npcap.com/dist/npcap-sdk-{NPCAP_SDK_VERSION}.zip");
    let zip_path = cache_dir.join(format!("npcap-sdk-{NPCAP_SDK_VERSION}.zip"));

    // Create cache directory
    fs::create_dir_all(cache_dir).context("Failed to create cache directory")?;

    // Download if zip doesn't exist or is corrupted
    if !zip_path.exists() || !is_valid_zip(&zip_path) {
        println!("cargo:warning=Downloading Npcap SDK v{NPCAP_SDK_VERSION} from {url}");
        let response = reqwest::blocking::get(&url).context("Failed to download Npcap SDK")?;
        let bytes = response
            .bytes()
            .context("Failed to read Npcap SDK response")?;
        fs::write(&zip_path, &bytes).context("Failed to save Npcap SDK zip")?;
    }

    // Clean destination before extraction
    if dest_dir.exists() {
        fs::remove_dir_all(dest_dir).context("Failed to clean old SDK")?;
    }
    fs::create_dir_all(dest_dir).context("Failed to create SDK directory")?;

    // Extract
    let zip_file = File::open(&zip_path)?;
    let mut archive = ZipArchive::new(zip_file).context("Failed to read zip archive")?;
    archive
        .extract(dest_dir)
        .context("Failed to extract Npcap SDK")?;

    // Write version marker for future checks
    fs::write(dest_dir.join(".version"), NPCAP_SDK_VERSION)
        .context("Failed to write version marker")?;

    println!(
        "cargo:warning=Npcap SDK v{NPCAP_SDK_VERSION} extracted to '{}'",
        dest_dir.display()
    );
    Ok(())
}

#[cfg(windows)]
fn is_valid_zip(path: &Path) -> bool {
    if let Ok(zip) = File::open(path) {
        ZipArchive::new(zip).is_ok()
    } else {
        false
    }
}

#[cfg(not(windows))]
fn main() {
    // No-op on non-Windows targets
}
