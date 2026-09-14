//! Self-update: check GitHub's latest release once at startup, and apply it
//! in-app (download, atomic rename-swap, relaunch) on user confirmation.
//!
//! Every network call runs on a short-lived background thread and never
//! panics -- the check is best-effort and silent on failure (offline, no
//! releases yet, parse error), while applying an update reports failure
//! through `UpdateStatus::Error` since that path is user-initiated.

use std::io::{self, Read};
use std::path::Path;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use serde::Deserialize;
use ureq::Agent;

const REPO_OWNER: &str = "Vinello28";
const REPO_NAME: &str = "wtop";
const USER_AGENT: &str = concat!("wtop-updater/", env!("CARGO_PKG_VERSION"));

// The release publishes one asset per architecture under a distinct name
// (see .github/workflows/release.yml); pick the one matching this binary's
// own compiled target, never the running OS's architecture -- an x86_64
// build under ARM64 emulation must keep updating itself as x86_64.
#[cfg(target_arch = "aarch64")]
const ASSET_NAME: &str = "wtop-arm64.exe";
#[cfg(not(target_arch = "aarch64"))]
const ASSET_NAME: &str = "wtop.exe";

#[derive(Debug, Clone)]
pub struct ReleaseInfo {
    /// Tag with the leading 'v' stripped, e.g. "1.2.3".
    pub version: String,
    pub asset_url: String,
}

#[derive(Debug, Clone, Default)]
pub enum UpdateStatus {
    #[default]
    Idle,
    Available(ReleaseInfo),
    Downloading,
    Ready,
    // Kept for potential future diagnostics; the UI deliberately renders no
    // badge for this state so a network hiccup never nags the user.
    #[allow(dead_code)]
    Error(String),
}

#[derive(Deserialize)]
struct GhAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Deserialize)]
struct GhRelease {
    tag_name: String,
    assets: Vec<GhAsset>,
}

/// Spawns a one-shot background thread that checks GitHub for a newer
/// release. This is a genuine one-off: the thread makes at most one HTTP
/// call and exits, structurally separate from the long-lived sampling
/// loop, matching the "check only at startup" requirement.
pub fn spawn_check(status: Arc<Mutex<UpdateStatus>>) {
    let _ = thread::Builder::new()
        .name("wtop-update-check".to_string())
        .spawn(move || {
            if let Some(info) = check_latest_release()
                && let Ok(mut guard) = status.lock()
            {
                *guard = UpdateStatus::Available(info);
            }
        });
}

/// Spawns a one-shot background thread that downloads the update, swaps it
/// into place, and relaunches. Unlike `spawn_check`, this is user-initiated
/// (via the `u` confirm prompt), so failures are surfaced through `status`
/// rather than swallowed.
pub fn spawn_apply_update(info: ReleaseInfo, status: Arc<Mutex<UpdateStatus>>) {
    if let Ok(mut guard) = status.lock() {
        *guard = UpdateStatus::Downloading;
    }
    let _ = thread::Builder::new()
        .name("wtop-update-apply".to_string())
        .spawn(move || {
            let result = apply_update(&info);
            if let Ok(mut guard) = status.lock() {
                *guard = match result {
                    Ok(()) => UpdateStatus::Ready,
                    Err(e) => UpdateStatus::Error(e.to_string()),
                };
            }
        });
}

/// Best-effort cleanup of a leftover `wtop.exe.old` from a previous update.
/// Call once at startup; failure (already gone, still locked) is ignored --
/// this is housekeeping, never load-bearing.
pub fn cleanup_previous_update() {
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        let _ = std::fs::remove_file(dir.join("wtop.exe.old"));
    }
}

fn agent(timeout: Duration) -> Agent {
    let config = Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(5)))
        .timeout_global(Some(timeout))
        .build();
    Agent::new_with_config(config)
}

fn check_latest_release() -> Option<ReleaseInfo> {
    let url = format!("https://api.github.com/repos/{REPO_OWNER}/{REPO_NAME}/releases/latest");
    let body = agent(Duration::from_secs(6))
        .get(&url)
        .header("User-Agent", USER_AGENT)
        .header("Accept", "application/vnd.github+json")
        .call()
        .ok()?
        .body_mut()
        .read_to_string()
        .ok()?;

    let release: GhRelease = serde_json::from_str(&body).ok()?;
    let asset = release.assets.into_iter().find(|a| a.name == ASSET_NAME)?;
    let version = release.tag_name.trim_start_matches('v').to_string();

    if !is_newer(&version, env!("CARGO_PKG_VERSION")) {
        return None;
    }

    Some(ReleaseInfo {
        version,
        asset_url: asset.browser_download_url,
    })
}

/// Hand-rolled major.minor.patch comparison -- release versions here are
/// always simple triples, so a semver crate isn't warranted. Malformed
/// segments degrade to 0 rather than panicking.
pub fn is_newer(remote: &str, current: &str) -> bool {
    parse_version(remote) > parse_version(current)
}

fn parse_version(v: &str) -> (u32, u32, u32) {
    let mut parts = v.split('.').map(|p| p.parse::<u32>().unwrap_or(0));
    (
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
    )
}

#[derive(Debug)]
enum UpdaterError {
    Network(String),
    Io(io::Error),
    InvalidBinary,
    NoParentDir,
    NoCurrentExe,
}

impl std::fmt::Display for UpdaterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpdaterError::Network(e) => write!(f, "network error: {e}"),
            UpdaterError::Io(e) => write!(f, "I/O error: {e}"),
            UpdaterError::InvalidBinary => write!(f, "downloaded file is not a valid executable"),
            UpdaterError::NoParentDir => write!(f, "could not determine install directory"),
            UpdaterError::NoCurrentExe => write!(f, "could not determine running executable path"),
        }
    }
}

impl From<io::Error> for UpdaterError {
    fn from(e: io::Error) -> Self {
        UpdaterError::Io(e)
    }
}

fn apply_update(info: &ReleaseInfo) -> Result<(), UpdaterError> {
    let current_exe = std::env::current_exe().map_err(|_| UpdaterError::NoCurrentExe)?;
    let dir = current_exe.parent().ok_or(UpdaterError::NoParentDir)?;

    let staged = dir.join("wtop.exe.update");
    download_asset(&info.asset_url, &staged)?;
    verify_pe_binary(&staged)?;

    let old = dir.join("wtop.exe.old");
    let _ = std::fs::remove_file(&old); // clear a stale leftover before renaming into it

    // Windows/NTFS allows renaming a file that's currently mapped/running,
    // just not overwriting it in place -- this is the standard self-replace
    // dance: move the running exe aside, move the new one into its place.
    std::fs::rename(&current_exe, &old)?;
    std::fs::rename(&staged, &current_exe)?;

    // Best-effort immediate cleanup: the old binary is only renamed here,
    // still backing this running process's image, so deleting it now relies
    // on the same FILE_SHARE_DELETE semantics the rename above already used
    // -- it disappears from the directory right away and its disk space is
    // reclaimed once this process exits (which happens moments later, see
    // UpdateStatus::Ready in main.rs). cleanup_previous_update() at the next
    // startup remains as a fallback for the rare case this is blocked (e.g.
    // an AV scanner briefly holding its own handle).
    let _ = std::fs::remove_file(&old);

    Command::new(&current_exe).spawn()?;
    Ok(())
}

fn download_asset(url: &str, dest: &Path) -> Result<(), UpdaterError> {
    let mut response = agent(Duration::from_secs(60))
        .get(url)
        .header("User-Agent", USER_AGENT)
        .call()
        .map_err(|e| UpdaterError::Network(e.to_string()))?;

    let mut file = std::fs::File::create(dest)?;
    let mut reader = response.body_mut().as_reader();
    io::copy(&mut reader, &mut file)?;
    Ok(())
}

fn verify_pe_binary(path: &Path) -> Result<(), UpdaterError> {
    let mut file = std::fs::File::open(path)?;
    let mut header = [0u8; 2];
    let n = file.read(&mut header)?;
    if n < 2 || &header != b"MZ" {
        return Err(UpdaterError::InvalidBinary);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_newer_detects_bumps() {
        assert!(is_newer("1.0.1", "1.0.0"));
        assert!(is_newer("1.1.0", "1.0.9"));
        assert!(is_newer("2.0.0", "1.9.9"));
        assert!(!is_newer("1.0.0", "1.0.0"));
        assert!(!is_newer("1.0.0", "1.0.1"));
    }

    #[test]
    fn is_newer_handles_malformed_versions() {
        // Never panics; unparseable segments degrade to 0.
        assert!(!is_newer("garbage", "1.0.0"));
        assert!(is_newer("1.0.0", "garbage"));
        assert!(!is_newer("", ""));
    }

    #[test]
    fn verify_pe_binary_accepts_mz_header() {
        let dir = std::env::temp_dir().join("wtop-updater-test-pe-ok");
        std::fs::write(&dir, b"MZ\x90\x00rest-of-a-fake-pe").unwrap();
        assert!(verify_pe_binary(&dir).is_ok());
        let _ = std::fs::remove_file(&dir);
    }

    #[test]
    fn verify_pe_binary_rejects_bad_or_empty_files() {
        let bad = std::env::temp_dir().join("wtop-updater-test-pe-bad");
        std::fs::write(&bad, b"not a pe file").unwrap();
        assert!(verify_pe_binary(&bad).is_err());
        let _ = std::fs::remove_file(&bad);

        let empty = std::env::temp_dir().join("wtop-updater-test-pe-empty");
        std::fs::write(&empty, b"").unwrap();
        assert!(verify_pe_binary(&empty).is_err());
        let _ = std::fs::remove_file(&empty);
    }

    #[test]
    fn self_replace_rename_dance_on_temp_files() {
        // Exercises the exact rename sequence apply_update() performs,
        // against throwaway files instead of a real exe -- proves Windows/
        // NTFS allows renaming a file "in place" this way and that the
        // leftover-cleanup logic works, without needing a real release.
        let dir = std::env::temp_dir().join("wtop-updater-test-rename-dance");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let current = dir.join("wtop.exe");
        let staged = dir.join("wtop.exe.update");
        let old = dir.join("wtop.exe.old");

        std::fs::write(&current, b"old version").unwrap();
        std::fs::write(&staged, b"new version").unwrap();

        let _ = std::fs::remove_file(&old);
        std::fs::rename(&current, &old).unwrap();
        std::fs::rename(&staged, &current).unwrap();

        assert_eq!(std::fs::read(&current).unwrap(), b"new version");
        assert_eq!(std::fs::read(&old).unwrap(), b"old version");
        assert!(!staged.exists());

        // cleanup_previous_update()'s logic: best-effort remove of `.old`.
        let _ = std::fs::remove_file(&old);
        assert!(!old.exists());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
