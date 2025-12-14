use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use std::env;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;

use app::common::UpdateStatus;

const REPO_OWNER: &str = "deep-gaurav";
const REPO_NAME: &str = "self-cloud";

#[server(CheckUpdate, "/api")]
pub async fn check_update() -> Result<UpdateStatus, ServerFnError> {
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

    // Attempt to get the short hash from the body or target_commitish
    // But nightly tag target_commitish might be the tag itself?
    // We'll rely on timestamps mostly.
    let remote_commit = json["target_commitish"].as_str().unwrap_or("unknown");

    let current_timestamp = env!("BUILD_TIMESTAMP");
    let current_git_hash = env!("GIT_HASH");

    // Simple string comparison for ISO8601 works if timezones are same (UTC)
    let update_available = remote_published_at > current_timestamp;

    Ok(UpdateStatus {
        current_git_hash: current_git_hash.to_string(),
        current_build_time: current_timestamp.to_string(),
        remote_git_hash: remote_commit.to_string(), // This might be "nightly" or a SHA
        remote_build_time: remote_published_at.to_string(),
        update_available,
    })
}

#[server(PerformUpdate, "/api")]
pub async fn perform_update() -> Result<String, ServerFnError> {
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

    // 5. Restart
    let restart_cmd = env::var("SELF_UPDATE_CMD")
        .unwrap_or_else(|_| "systemctl --user restart selfcloud".to_string());

    // We execute the command in background or just run it?
    // If we restart immediately, this request might fail to return.
    // We'll spawn it.

    tracing::info!("Triggering restart with: {}", restart_cmd);

    tokio::spawn(async move {
        // Sleep a bit to allow response to go out
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        // Split cmd
        let parts: Vec<&str> = restart_cmd.split_whitespace().collect();
        if let Some((cmd, args)) = parts.split_first() {
            let _ = Command::new(cmd).args(args).spawn();
        }
    });

    Ok("Update initiated. Server restarting...".to_string())
}
