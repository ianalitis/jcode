//! Browser bridge downloads, extension archives, and native-host manifests.
use super::{
    BrowserStatus, CHROMIUM_EXTENSION_ID, EXTENSION_ID_LISTED, EXTENSION_ID_LOCAL,
    GITHUB_API_LATEST, NATIVE_HOST_NAME, browser_binary_path, chromium_extension_dir,
    host_binary_path, safari_extension_dir, xpi_path,
};
use crate::browser_detect::{BrowserFamily, BrowserKind};
use anyhow::{Context, Result};
use std::path::PathBuf;

pub(super) fn manual_install_hint(kind: BrowserKind) -> String {
    match kind.family() {
        BrowserFamily::Gecko => format!(
            "       Manually install: Firefox > about:addons > Install from file > {}\n",
            xpi_path().display()
        ),
        BrowserFamily::Chromium => format!(
            "       Manually install: open {} > enable Developer mode > Load unpacked > {}\n",
            kind.extensions_page(),
            chromium_extension_dir().display()
        ),
        BrowserFamily::Safari => format!(
            "       Manually install: on a Mac with Xcode, convert {} with `xcrun safari-web-extension-converter`, build and open the app, then enable it in Safari > Settings > Extensions.\n",
            safari_extension_dir().display()
        ),
    }
}

pub(super) fn extension_package_present(kind: BrowserKind) -> bool {
    match kind.family() {
        BrowserFamily::Gecko => xpi_path().exists(),
        BrowserFamily::Chromium => chromium_extension_dir().join("manifest.json").exists(),
        BrowserFamily::Safari => safari_extension_dir().join("manifest.json").exists(),
    }
}

/// Whether a bridge ping came from the requested browser family. Chromium
/// browsers are interchangeable here because they share one extension build.
pub(super) fn ping_matches(info: &serde_json::Value, kind: BrowserKind) -> bool {
    match info.get("browser").and_then(|b| b.as_str()) {
        // Older extensions do not report a browser and are Firefox-only.
        None => kind.family() == BrowserFamily::Gecko,
        Some(reported) => {
            BrowserKind::parse(reported).is_some_and(|r| r == kind || r.family() == kind.family())
        }
    }
}

pub(super) fn connected_matches(status: &BrowserStatus, kind: BrowserKind) -> bool {
    match status.connected_browser.as_deref() {
        None => kind.family() == BrowserFamily::Gecko,
        Some(reported) => {
            BrowserKind::parse(reported).is_some_and(|r| r == kind || r.family() == kind.family())
        }
    }
}

pub(super) async fn download_browser_binary_for(kind: BrowserKind) -> Result<()> {
    let asset_name = get_platform_asset_name();
    let client = jcode_provider_core::shared_http_client();

    let mut request = client
        .get(GITHUB_API_LATEST)
        .header(reqwest::header::ACCEPT, "application/vnd.github+json");
    // Avoid the shared unauthenticated 60 req/h per-IP GitHub bucket when a
    // token is available (see crate::github).
    if let Some(token) = crate::github::github_public_api_token() {
        request = request.bearer_auth(token);
    }
    let release_info: serde_json::Value = request
        .send()
        .await?
        .json()
        .await
        .context("Failed to fetch latest release info")?;

    let assets = release_info["assets"]
        .as_array()
        .context("No assets in release")?;

    // Find the browser CLI binary
    let browser_asset = assets
        .iter()
        .find(|a| a["name"].as_str() == Some(&asset_name))
        .context(format!("No asset found for platform: {}", asset_name))?;

    let download_url = browser_asset["browser_download_url"]
        .as_str()
        .context("No download URL")?;

    let find_asset = |pred: &dyn Fn(&str) -> bool| {
        assets
            .iter()
            .find(|a| a["name"].as_str().is_some_and(pred))
            .and_then(|a| a["browser_download_url"].as_str())
            .map(str::to_string)
    };
    let xpi_url = find_asset(&|n| n.ends_with(".xpi"));
    let chromium_url =
        find_asset(&|n| n.starts_with("browser-agent-bridge-chrome") && n.ends_with(".zip"));
    let safari_url =
        find_asset(&|n| n.starts_with("browser-agent-bridge-safari") && n.ends_with(".zip"));
    let needed = match kind.family() {
        BrowserFamily::Gecko => xpi_url
            .as_ref()
            .map(|_| ())
            .context("No XPI asset found in release"),
        BrowserFamily::Chromium => chromium_url
            .as_ref()
            .map(|_| ())
            .context("No Chromium extension package found in the latest bridge release"),
        BrowserFamily::Safari => safari_url
            .as_ref()
            .map(|_| ())
            .context("No Safari extension package found in the latest bridge release"),
    };
    needed?;

    // Find the host binary
    let host_asset_name = get_host_asset_name();
    let host_asset = assets
        .iter()
        .find(|a| a["name"].as_str() == Some(&host_asset_name))
        .with_context(|| {
            let available = assets
                .iter()
                .filter_map(|a| a["name"].as_str())
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "No native host asset found for platform: {}. Expected release asset '{}' alongside '{}'. Available assets: {}",
                std::env::consts::OS,
                host_asset_name,
                asset_name,
                available
            )
        })?;

    // Download browser CLI
    let browser_bytes = client
        .get(download_url)
        .send()
        .await?
        .bytes()
        .await
        .context("Failed to download browser binary")?;

    let bin_path = browser_binary_path();
    write_file_atomically(&bin_path, &browser_bytes, true)?;

    // Download the extension package(s). The XPI is always fetched when
    // available so switching back to Firefox needs no extra download.
    if let Some(url) = &xpi_url {
        let bytes = client
            .get(url)
            .send()
            .await?
            .bytes()
            .await
            .context("Failed to download XPI")?;
        write_file_atomically(&xpi_path(), &bytes, false)?;
    }
    match kind.family() {
        BrowserFamily::Chromium => {
            let url = chromium_url
                .as_deref()
                .context("No Chromium extension package")?;
            let bytes = client
                .get(url)
                .send()
                .await?
                .bytes()
                .await
                .context("Failed to download Chromium extension")?;
            replace_dir_with_zip(&chromium_extension_dir(), &bytes)?;
        }
        BrowserFamily::Safari => {
            let url = safari_url
                .as_deref()
                .context("No Safari extension package")?;
            let bytes = client
                .get(url)
                .send()
                .await?
                .bytes()
                .await
                .context("Failed to download Safari extension")?;
            replace_dir_with_zip(&safari_extension_dir(), &bytes)?;
        }
        BrowserFamily::Gecko => {}
    }

    // Download host binary
    let host_url = host_asset["browser_download_url"]
        .as_str()
        .context("No host download URL")?;
    let host_bytes = client
        .get(host_url)
        .send()
        .await?
        .bytes()
        .await
        .context("Failed to download host binary")?;

    let host_path = host_binary_path();
    write_file_atomically(&host_path, &host_bytes, true)?;

    Ok(())
}

fn write_file_atomically(path: &PathBuf, bytes: &[u8], _executable: bool) -> Result<()> {
    let parent = path
        .parent()
        .context("Target file has no parent directory")?;
    std::fs::create_dir_all(parent)?;

    let ts = chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default();
    let pid = std::process::id();
    let tmp_path = parent.join(format!(
        ".{}.tmp-{}-{}",
        path.file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("download"),
        pid,
        ts
    ));

    std::fs::write(&tmp_path, bytes)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = if _executable { 0o755 } else { 0o644 };
        std::fs::set_permissions(&tmp_path, std::fs::Permissions::from_mode(mode))?;
    }

    std::fs::rename(&tmp_path, path)?;
    Ok(())
}

/// Extract `bytes` (a zip archive) into `dir`, replacing its previous
/// contents. The directory path stays stable so a browser that loaded the
/// unpacked extension from it picks up the update on reload.
fn replace_dir_with_zip(dir: &std::path::Path, bytes: &[u8]) -> Result<()> {
    let parent = dir.parent().context("extension dir has no parent")?;
    std::fs::create_dir_all(parent)?;
    let staging = parent.join(format!(
        ".{}.staging-{}",
        dir.file_name().and_then(|n| n.to_str()).unwrap_or("ext"),
        std::process::id()
    ));
    if staging.exists() {
        std::fs::remove_dir_all(&staging)?;
    }
    extract_zip(bytes, &staging)?;
    if dir.exists() {
        std::fs::remove_dir_all(dir)?;
    }
    std::fs::rename(&staging, dir)?;
    Ok(())
}

/// Minimal zip reader (stored and deflate entries) using the central
/// directory. Rejects absolute paths and `..` components.
pub(crate) fn extract_zip(bytes: &[u8], dest: &std::path::Path) -> Result<()> {
    use std::io::Read;
    let u16_at = |o: usize| -> Result<usize> {
        bytes
            .get(o..o + 2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]) as usize)
            .context("truncated zip")
    };
    let u32_at = |o: usize| -> Result<usize> {
        bytes
            .get(o..o + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize)
            .context("truncated zip")
    };
    let eocd = (0..bytes.len().saturating_sub(21))
        .rev()
        .find(|&i| bytes[i..].starts_with(&[0x50, 0x4b, 0x05, 0x06]))
        .context("not a zip archive")?;
    let entries = u16_at(eocd + 10)?;
    let mut offset = u32_at(eocd + 16)?;
    std::fs::create_dir_all(dest)?;
    for _ in 0..entries {
        anyhow::ensure!(
            bytes.get(offset..offset + 4) == Some(&[0x50, 0x4b, 0x01, 0x02][..]),
            "corrupt zip central directory"
        );
        let method = u16_at(offset + 10)?;
        let compressed = u32_at(offset + 20)?;
        let name_len = u16_at(offset + 28)?;
        let extra_len = u16_at(offset + 30)?;
        let comment_len = u16_at(offset + 32)?;
        let local = u32_at(offset + 42)?;
        let name_bytes = bytes
            .get(offset + 46..offset + 46 + name_len)
            .context("truncated zip")?;
        let name = String::from_utf8_lossy(name_bytes).replace('\\', "/");
        offset += 46 + name_len + extra_len + comment_len;

        let rel = std::path::Path::new(&name);
        anyhow::ensure!(
            !rel.is_absolute()
                && rel
                    .components()
                    .all(|c| matches!(c, std::path::Component::Normal(_))),
            "unsafe path in zip: {}",
            name
        );
        let out = dest.join(rel);
        if name.ends_with('/') {
            std::fs::create_dir_all(&out)?;
            continue;
        }
        let data_start = local + 30 + u16_at(local + 26)? + u16_at(local + 28)?;
        let data = bytes
            .get(data_start..data_start + compressed)
            .context("truncated zip entry")?;
        let contents = match method {
            0 => data.to_vec(),
            8 => {
                let mut buf = Vec::new();
                flate2::read::DeflateDecoder::new(data).read_to_end(&mut buf)?;
                buf
            }
            other => anyhow::bail!("unsupported zip compression method {}", other),
        };
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&out, contents)?;
    }
    Ok(())
}

pub(super) fn get_platform_asset_name() -> String {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        "browser-linux-x64".to_string()
    }
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    {
        "browser-linux-arm64".to_string()
    }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        "browser-macos-arm64".to_string()
    }
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    {
        "browser-macos-x64".to_string()
    }
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        "browser-windows-x64.exe".to_string()
    }
    #[cfg(not(any(
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "windows", target_arch = "x86_64"),
    )))]
    {
        format!(
            "browser-{}-{}",
            std::env::consts::OS,
            std::env::consts::ARCH
        )
    }
}

fn get_host_asset_name() -> String {
    let base = get_platform_asset_name();
    base.replace("browser-", "host-")
}

pub(super) fn native_host_manifest_json(kind: BrowserKind, host_path: &str) -> serde_json::Value {
    let mut manifest = serde_json::json!({
        "name": NATIVE_HOST_NAME,
        "description": "Native host for Browser Agent Bridge (managed by jcode)",
        "path": host_path,
        "type": "stdio",
    });
    if kind.family() == BrowserFamily::Gecko {
        manifest["allowed_extensions"] =
            serde_json::json!([EXTENSION_ID_LOCAL, EXTENSION_ID_LISTED]);
    } else {
        manifest["allowed_origins"] =
            serde_json::json!([format!("chrome-extension://{}/", CHROMIUM_EXTENSION_ID)]);
    }
    manifest
}

/// Whether an existing manifest already points at a live host and allows the
/// extension this browser uses.
pub(super) fn native_host_manifest_is_valid(
    kind: BrowserKind,
    existing: &serde_json::Value,
) -> bool {
    let host_ok = existing["path"]
        .as_str()
        .is_some_and(|p| std::path::Path::new(p).exists());
    let allowed_ok = if kind.family() == BrowserFamily::Gecko {
        existing["allowed_extensions"]
            .as_array()
            .is_some_and(|ids| {
                ids.iter()
                    .any(|id| id.as_str() == Some(EXTENSION_ID_LISTED))
            })
    } else {
        let origin = format!("chrome-extension://{}/", CHROMIUM_EXTENSION_ID);
        existing["allowed_origins"]
            .as_array()
            .is_some_and(|o| o.iter().any(|v| v.as_str() == Some(origin.as_str())))
    };
    host_ok && allowed_ok
}

pub(super) fn install_native_host_manifest_for(kind: BrowserKind) -> Result<bool> {
    let dirs = native_messaging_hosts_dirs_for(kind)?;
    let host_path = host_binary_path();
    if !host_path.exists() {
        return Err(anyhow::anyhow!(
            "Host binary not found at {}. The native messaging host is required for the {} extension to communicate with the bridge.",
            host_path.display(),
            kind.display_name()
        ));
    }
    let manifest = native_host_manifest_json(kind, &host_path.to_string_lossy());
    let mut wrote_any = false;
    for manifest_dir in dirs {
        let manifest_path = manifest_dir.join(format!("{}.json", NATIVE_HOST_NAME));
        let valid = std::fs::read_to_string(&manifest_path)
            .ok()
            .and_then(|c| serde_json::from_str::<serde_json::Value>(&c).ok())
            .is_some_and(|existing| native_host_manifest_is_valid(kind, &existing));
        if !valid {
            std::fs::create_dir_all(&manifest_dir)?;
            std::fs::write(&manifest_path, serde_json::to_string_pretty(&manifest)?)?;
            wrote_any = true;
        }
        #[cfg(target_os = "windows")]
        register_windows_native_host_manifest(kind, &manifest_path)?;
    }
    Ok(wrote_any)
}

#[cfg(target_os = "windows")]
fn register_windows_native_host_manifest(
    kind: BrowserKind,
    manifest_path: &std::path::Path,
) -> Result<()> {
    for root in kind.windows_native_host_registry_roots() {
        let key = format!(r"{}\{}", root, NATIVE_HOST_NAME);
        let output = std::process::Command::new("reg")
            .args([
                "add",
                &key,
                "/ve",
                "/t",
                "REG_SZ",
                "/d",
                &manifest_path.to_string_lossy(),
                "/f",
            ])
            .output()
            .context("Failed to register the native messaging host in the Windows registry")?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            let details = if stderr.trim().is_empty() {
                stdout.trim().to_string()
            } else {
                stderr.trim().to_string()
            };
            anyhow::bail!(
                "Failed to register the {} native messaging host in the Windows registry: {}",
                kind.display_name(),
                details
            );
        }
    }
    Ok(())
}

fn native_messaging_hosts_dirs_for(kind: BrowserKind) -> Result<Vec<PathBuf>> {
    let dirs = kind.native_messaging_dirs();
    if dirs.is_empty() {
        anyhow::bail!(
            "{} does not use native messaging on this platform",
            kind.display_name()
        );
    }
    Ok(dirs)
}
