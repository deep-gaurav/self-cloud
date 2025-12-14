use crate::common::UpdateStatus;
use leptos::prelude::*;

#[cfg(feature = "ssr")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "ssr")]
const REPO_OWNER: &str = "deep-gaurav";
#[cfg(feature = "ssr")]
const REPO_NAME: &str = "self-cloud";

#[server(CheckUpdate, "/api")]
pub async fn check_update() -> Result<UpdateStatus, ServerFnError> {
    use std::env;
    let client = reqwest::Client::new();
    let url = format!(
        "https://api.github.com/repos/{}/{}/releases/tags/nightly",
        REPO_OWNER, REPO_NAME
    );

    let resp = client
        .get(&url)
        .header("User-Agent", "SelfCloud-Server")
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    if !resp.status().is_success() {
        return Err(ServerFnError::new(format!(
            "Failed to fetch release info: {}",
            resp.status()
        )));
    }

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let remote_published_at = json["published_at"]
        .as_str()
        .ok_or_else(|| ServerFnError::new("Missing published_at"))?;

    let remote_commit = json["target_commitish"].as_str().unwrap_or("unknown");

    // These usages of env! will be compiled on the server side (ssr feature).
    // Note: env! reads at compile time. So the server binary must be compiled with these vars.
    // Which happens in build.rs of server?
    // Wait, this code is in `app`. `app` is compiled as library.
    // When `server` compiles, it compiles `app`. The `build.rs` of `server` runs for `server` crate.
    // It does NOT set env vars for `app` crate unless passed.
    // `app` has its own `build.rs`? Yes, we saw it.
    // We need to modify `app/build.rs` to include timestamp as well if we want `env!` to work here.
    // OR we pass them from server at runtime? No `env!` is compile time.
    // Better: `app/build.rs` should also set these.

    let current_timestamp = env!("BUILD_TIMESTAMP");
    let current_git_hash = env!("GIT_HASH");

    // Simple string comparison works for ISO8601
    let update_available = remote_published_at > current_timestamp;

    Ok(UpdateStatus {
        current_git_hash: current_git_hash.to_string(),
        current_build_time: current_timestamp.to_string(),
        remote_git_hash: remote_commit.to_string(),
        remote_build_time: remote_published_at.to_string(),
        update_available,
    })
}

#[server(PerformUpdate, "/api")]
pub async fn perform_update() -> Result<String, ServerFnError> {
    use std::env;
    use std::os::unix::fs::PermissionsExt;
    use std::process::Command;

    // 1. Identify architecture
    let arch = env::consts::ARCH;
    let asset_name = match arch {
        "x86_64" => "server-x64",
        "aarch64" => "server-arm64",
        _ => {
            return Err(ServerFnError::new(format!(
                "Unsupported architecture: {}",
                arch
            )))
        }
    };

    // 2. Get asset URL
    let client = reqwest::Client::new();
    let url = format!(
        "https://api.github.com/repos/{}/{}/releases/tags/nightly",
        REPO_OWNER, REPO_NAME
    );

    let resp = client
        .get(&url)
        .header("User-Agent", "SelfCloud-Server")
        .send()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let assets = json["assets"]
        .as_array()
        .ok_or(ServerFnError::new("No assets found"))?;
    let download_url = assets
        .iter()
        .find(|a| a["name"].as_str() == Some(asset_name))
        .and_then(|a| a["browser_download_url"].as_str())
        .ok_or_else(|| ServerFnError::new(format!("Asset {} not found", asset_name)))?;

    // 3. Download
    let bin_bytes = client
        .get(download_url)
        .header("User-Agent", "SelfCloud-Server")
        .send()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .bytes()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    // 4. Replace binary
    let current_exe = env::current_exe().map_err(|e| ServerFnError::new(e.to_string()))?;
    let tmp_exe = current_exe.with_extension("tmp");

    tokio::fs::write(&tmp_exe, bin_bytes)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let mut perms = tokio::fs::metadata(&tmp_exe).await.unwrap().permissions();
    perms.set_mode(0o755);
    tokio::fs::set_permissions(&tmp_exe, perms).await.unwrap();

    tokio::fs::rename(&tmp_exe, &current_exe)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    // 4.5. Update Site Assets
    let site_asset_name = match arch {
        "x86_64" => "site-x64.tar",
        "aarch64" => "site-arm64.tar",
        _ => {
            return Err(ServerFnError::new(
                "Unsupported architecture for site assets",
            ))
        }
    };

    let site_download_url = assets
        .iter()
        .find(|a| a["name"].as_str() == Some(site_asset_name))
        .and_then(|a| a["browser_download_url"].as_str())
        .ok_or_else(|| ServerFnError::new(format!("Asset {} not found", site_asset_name)))?;

    let site_bytes = client
        .get(site_download_url)
        .header("User-Agent", "SelfCloud-Server")
        .send()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .bytes()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let site_tar_path = std::path::PathBuf::from("site_update.tar");
    tokio::fs::write(&site_tar_path, site_bytes)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    // Extract tar
    // Assuming structure: target/site based on user script
    let status = tokio::process::Command::new("tar")
        .arg("-xf")
        .arg(&site_tar_path)
        .status()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to run tar: {}", e)))?;

    if !status.success() {
        return Err(ServerFnError::new("Failed to extract site assets"));
    }

    // Replace site dir
    // rm -rf site
    if tokio::fs::try_exists("site").await.unwrap_or(false) {
        tokio::fs::remove_dir_all("site")
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
    }

    // mv target/site site
    // We need to check if target/site exists.
    if tokio::fs::try_exists("target/site").await.unwrap_or(false) {
        tokio::fs::rename("target/site", "site")
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        let _ = tokio::fs::remove_dir_all("target").await;
    } else {
        // Fallback: maybe it extracts directly to site? Or site-arch?
        // If target/site doesn't exist, log warning but continue
        tracing::warn!("target/site not found after extraction. Check extracted structure.");
    }

    let _ = tokio::fs::remove_file(site_tar_path).await;

    // 5. Restart
    let restart_cmd = env::var("SELF_UPDATE_CMD")
        .unwrap_or_else(|_| "systemctl --user restart selfcloud".to_string());

    tracing::info!("Triggering restart with: {}", restart_cmd);

    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        // Split cmd - naive splitting.
        // For simple commands this works.
        let parts: Vec<&str> = restart_cmd.split_whitespace().collect();
        if let Some((cmd, args)) = parts.split_first() {
            let _ = Command::new(cmd).args(args).spawn();
        }
    });

    Ok("Update initiated. Server restarting...".to_string())
}
