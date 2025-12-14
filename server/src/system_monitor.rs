use std::time::Duration;

use app::common::{DiskInfo, ProcessInfo, SystemStats};
use axum::{
    extract::{
        ws::{Message, WebSocket},
        WebSocketUpgrade,
    },
    response::Response,
};
use axum_extra::extract::cookie::CookieJar;
use sysinfo::{Disks, Networks, System};
use tracing::warn;

use crate::container_events::ensure_authorized_user;

pub async fn system_stats_ws(
    jar: CookieJar,
    ws: WebSocketUpgrade,
) -> Result<Response, (axum::http::StatusCode, String)> {
    ensure_authorized_user(jar)?;
    Ok(ws.on_upgrade(handle_system_stats_socket))
}

async fn handle_system_stats_socket(mut socket: WebSocket) {
    let mut sys = System::new_all();
    let mut disks = Disks::new_with_refreshed_list();

    loop {
        tokio::select! {
            rec = socket.recv() => {
                if rec.is_none() {
                    break;
                }
            }
            _ = tokio::time::sleep(Duration::from_secs(2)) => {
                sys.refresh_all();
                disks.refresh_list();

                let mut disk_infos = Vec::new();
                for disk in &disks {
                    disk_infos.push(DiskInfo {
                        name: disk.name().to_string_lossy().to_string(),
                        mount_point: disk.mount_point().to_string_lossy().to_string(),
                        total_space: disk.total_space(),
                        available_space: disk.available_space(),
                    });
                }

                let stats = SystemStats {
                    cpu_usage: sys.global_cpu_info().cpu_usage(),
                    total_memory: sys.total_memory(),
                    used_memory: sys.used_memory(),
                    total_swap: sys.total_swap(),
                    used_swap: sys.used_swap(),
                    disks: disk_infos,
                };

                if let Ok(serialized) = serde_json::to_string(&stats) {
                    if let Err(e) = socket.send(Message::Text(serialized.into())).await {
                        warn!("Failed to send stats: {e:?}");
                        break;
                    }
                }
            }
        }
    }
}

pub async fn process_stats_ws(
    jar: CookieJar,
    ws: WebSocketUpgrade,
) -> Result<Response, (axum::http::StatusCode, String)> {
    ensure_authorized_user(jar)?;
    Ok(ws.on_upgrade(handle_process_stats_socket))
}

async fn handle_process_stats_socket(mut socket: WebSocket) {
    let mut sys = System::new_all();

    loop {
        tokio::select! {
             msg = socket.recv() => {
                 if msg.is_none() { break; }
             }
             _ = tokio::time::sleep(Duration::from_secs(5)) => {
                 sys.refresh_processes();
                 let mut processes = Vec::new();
                 for (pid, process) in sys.processes() {
                     processes.push(ProcessInfo {
                         pid: pid.as_u32(),
                         name: process.name().to_string(),
                         cpu_usage: process.cpu_usage(),
                         memory: process.memory(),
                         user_id: process.user_id().map(|u| u.to_string()),
                         status: process.status().to_string(),
                     });
                 }

                 // Sort by CPU usage desc by default?
                 processes.sort_by(|a, b| b.cpu_usage.total_cmp(&a.cpu_usage));

                 if let Ok(serialized) = serde_json::to_string(&processes) {
                     if let Err(e) = socket.send(Message::Text(serialized.into())).await {
                         warn!("Failed to send process stats: {e:?}");
                         break;
                     }
                 }
             }
        }
    }
}
