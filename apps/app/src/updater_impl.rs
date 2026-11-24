use crate::api::Result;
use serde::Serialize;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
#[cfg(target_os = "windows")]
use std::process::Command as ProcessCommand;
use std::sync::{Mutex, OnceLock};
use tauri::{Manager, Webview};
use tauri_plugin_http::reqwest::{self, header::ACCEPT};
use theseus::{
    LoadingBarType, emit_loading, init_loading, launcher_user_agent,
};
use tokio::time::Instant;

const RELEASES_URL: &str = "https://api.github.com/repos/Panda-Minecraft-Launcher/code/releases/latest";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GithubUpdate {
    pub rid: u32,
    pub version: String,
    pub current_version: String,
    pub body: Option<String>,
    pub date: Option<String>,
    pub download_url: String,
    pub size: Option<u64>,
}

#[derive(Default)]
pub struct PendingUpdateData(pub Mutex<Option<(PathBuf, GithubUpdate)>>);

static LATEST_UPDATE: OnceLock<Mutex<Option<GithubUpdate>>> = OnceLock::new();

fn latest_update() -> &'static Mutex<Option<GithubUpdate>> {
    LATEST_UPDATE.get_or_init(Default::default)
}

fn version_parts(version: &str) -> Vec<u64> {
    version
        .split('-')
        .next()
        .unwrap_or(version)
        .split('.')
        .map(|part| part.parse::<u64>().unwrap_or(0))
        .collect()
}

#[cfg(target_os = "linux")]
fn os_release_is_debian(contents: &str) -> bool {
    let mut id = None;
    let mut id_like = None;
    for line in contents.lines() {
        if let Some(value) = line.strip_prefix("ID=") {
            id = Some(value.trim().trim_matches(|c| c == '"' || c == '\''));
        } else if let Some(value) = line.strip_prefix("ID_LIKE=") {
            id_like =
                Some(value.trim().trim_matches(|c| c == '"' || c == '\''));
        }
    }
    id == Some("debian")
        || id_like.is_some_and(|value| {
            value.split_whitespace().any(|part| part == "debian")
        })
}

#[cfg(target_os = "linux")]
fn is_debian_based() -> bool {
    fs::read_to_string("/etc/os-release")
        .map(|contents| os_release_is_debian(&contents))
        .unwrap_or(false)
}

#[cfg(not(target_os = "linux"))]
fn is_debian_based() -> bool {
    false
}

fn update_file_suffix(download_url: &str) -> &'static str {
    if download_url.ends_with(".deb") {
        ".deb"
    } else {
        ""
    }
}

fn version_is_newer(latest: &str, current: &str) -> bool {
    let latest = version_parts(latest);
    let current = version_parts(current);
    for index in 0..latest.len().max(current.len()) {
        let latest = *latest.get(index).unwrap_or(&0);
        let current = *current.get(index).unwrap_or(&0);
        if latest != current {
            return latest > current;
        }
    }
    false
}

#[tauri::command]
pub async fn check_github_update() -> Result<Option<GithubUpdate>> {
    let client = reqwest::Client::builder()
        .user_agent(launcher_user_agent())
        .build()?;
    let bytes = client
        .get(RELEASES_URL)
        .header(ACCEPT, "application/vnd.github+json")
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;
    let release: serde_json::Value = serde_json::from_slice(&bytes)?;

    let tag = release
        .get("tag_name")
        .and_then(|tag| tag.as_str())
        .unwrap_or("");
    let version = tag.strip_prefix('v').unwrap_or(tag).to_string();
    let current_version = env!("CARGO_PKG_VERSION").to_string();
    if !version_is_newer(&version, &current_version) {
        return Ok(None);
    }

    let appimage_name =
        format!("Panda.Minecraft.Launcher_{version}_amd64.AppImage");
    let asset_name = if cfg!(target_os = "windows") {
        format!("Panda.Minecraft.Launcher_{version}_x64-setup.exe")
    } else if is_debian_based() {
        let deb_name = format!("Panda.Minecraft.Launcher_{version}_amd64.deb");
        if release
            .get("assets")
            .and_then(|assets| assets.as_array())
            .is_some_and(|assets| {
                assets.iter().any(|asset| {
                    asset.get("name").and_then(|name| name.as_str())
                        == Some(deb_name.as_str())
                })
            })
        {
            deb_name
        } else {
            appimage_name
        }
    } else {
        appimage_name
    };
    let Some(asset) = release
        .get("assets")
        .and_then(|assets| assets.as_array())
        .and_then(|assets| {
            assets.iter().find(|asset| {
                asset.get("name").and_then(|name| name.as_str())
                    == Some(asset_name.as_str())
            })
        })
    else {
        tracing::warn!("Release {tag} has no asset named {asset_name}");
        return Ok(None);
    };
    let Some(download_url) = asset
        .get("browser_download_url")
        .and_then(|url| url.as_str())
    else {
        return Ok(None);
    };

    let update = GithubUpdate {
        rid: 1,
        version,
        current_version,
        body: release
            .get("body")
            .and_then(|body| body.as_str())
            .map(str::to_string),
        date: release
            .get("published_at")
            .and_then(|date| date.as_str())
            .map(str::to_string),
        download_url: download_url.to_string(),
        size: asset.get("size").and_then(|size| size.as_u64()),
    };
    latest_update().lock().unwrap().replace(update.clone());
    Ok(Some(update))
}

fn temp_dir() -> Result<PathBuf> {
    #[cfg(target_os = "linux")]
    if let Some(appimage) = std::env::var_os("APPIMAGE") {
        let path = PathBuf::from(appimage);
        if let Some(parent) = path.parent() {
            return Ok(parent.to_path_buf());
        }
    }
    Ok(std::env::temp_dir())
}

#[tauri::command]
pub async fn enqueue_update_for_installation<R: tauri::Runtime>(
    webview: Webview<R>,
) -> Result<()> {
    let Some(update) = latest_update().lock().unwrap().clone() else {
        return Err(anyhow::anyhow!("No update has been checked yet").into());
    };

    let progress = init_loading(
        LoadingBarType::LauncherUpdate {
            version: update.version.clone(),
            current_version: update.current_version.clone(),
        },
        1.0,
        "Downloading update...",
    )
    .await?;

    let client = reqwest::Client::builder()
        .user_agent(launcher_user_agent())
        .build()?;
    let mut response = client
        .get(&update.download_url)
        .send()
        .await?
        .error_for_status()?;
    let total = response.content_length();
    let mut file = tempfile::Builder::new()
        .prefix(".panda-update-")
        .suffix(update_file_suffix(&update.download_url))
        .tempfile_in(temp_dir()?)?;

    let download_start = Instant::now();
    while let Some(chunk) = response.chunk().await? {
        file.write_all(&chunk)?;
        if let Some(total) = total.filter(|total| *total > 0) {
            if let Err(error) =
                emit_loading(&progress, chunk.len() as f64 / total as f64, None)
            {
                tracing::error!(
                    "Failed to update download progress bar: {error}"
                );
            }
        }
    }
    file.flush()?;
    tracing::info!("Downloaded update in {:?}", download_start.elapsed());

    let (_, path) = file.keep()?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(&path)?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&path, permissions)?;
    }

    webview
        .state::<PendingUpdateData>()
        .inner()
        .0
        .lock()
        .unwrap()
        .replace((path, update));
    Ok(())
}

#[tauri::command]
pub fn remove_enqueued_update<R: tauri::Runtime>(webview: Webview<R>) {
    if let Some((path, _)) = webview
        .state::<PendingUpdateData>()
        .inner()
        .0
        .lock()
        .unwrap()
        .take()
    {
        let _ = fs::remove_file(path);
    }
}

pub fn install_github_update(path: &Path, should_restart: bool) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        use anyhow::Context;

        let _ = should_restart;
        if path.extension().and_then(|extension| extension.to_str())
            == Some("deb")
        {
            std::process::Command::new("xdg-open")
                .arg(path)
                .spawn()
                .context("failed to open downloaded Debian package")?;
            tracing::info!(
                "Opened Debian package {} for installation",
                path.display()
            );
            return Ok(());
        }
        let target = std::env::var_os("APPIMAGE")
            .map(PathBuf::from)
            .context("APPIMAGE environment variable is not set")?;
        fs::rename(path, &target).with_context(|| {
            format!("failed to replace {}", target.display())
        })?;
        tracing::info!("Replaced AppImage at {}", target.display());
    }
    #[cfg(target_os = "windows")]
    {
        let mut command = ProcessCommand::new(path);
        command.arg("/S").arg("/UPDATE");
        if should_restart {
            command.arg("/R");
        }
        command.spawn()?;
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        let _ = (path, should_restart);
        anyhow::bail!("automatic updates are not supported on this platform");
    }
    Ok(())
}

pub fn restart_target() -> Option<PathBuf> {
    #[cfg(target_os = "linux")]
    {
        std::env::var_os("APPIMAGE").map(PathBuf::from)
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

#[cfg(test)]
mod tests {
    #[cfg(target_os = "linux")]
    use super::os_release_is_debian;
    use super::update_file_suffix;
    use super::version_is_newer;

    #[test]
    fn keeps_deb_extension_on_downloaded_update() {
        assert_eq!(
            update_file_suffix(
                "https://example.com/Panda.Minecraft.Launcher_0.0.6_amd64.deb"
            ),
            ".deb"
        );
        assert_eq!(
            update_file_suffix(
                "https://example.com/Panda.Minecraft.Launcher_0.0.6_amd64.AppImage"
            ),
            ""
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn detects_debian_derivatives_from_os_release() {
        assert!(os_release_is_debian("ID=debian\n"));
        assert!(os_release_is_debian("ID=ubuntu\nID_LIKE=debian\n"));
        assert!(os_release_is_debian(
            "ID=linuxmint\nID_LIKE=\"debian ubuntu\"\n"
        ));
        assert!(!os_release_is_debian("ID=fedora\nID_LIKE=fedora\n"));
        assert!(!os_release_is_debian(""));
    }

    #[test]
    fn compares_release_versions() {
        assert!(version_is_newer("0.0.2", "0.0.1"));
        assert!(version_is_newer("1.0.0", "0.9.9"));
        assert!(!version_is_newer("0.0.1", "0.0.1"));
        assert!(!version_is_newer("0.0.1", "0.0.2"));
        assert!(!version_is_newer("0.0.1-canary+5.abc", "0.0.1"));
    }
}
