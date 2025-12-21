use std::sync::Arc;

use axum::{
    extract::{
        ws::{Message, WebSocket},
        Path, State, WebSocketUpgrade,
    },
    response::Response,
};
use axum_extra::extract::cookie::CookieJar;
use docker_api::Container;
use http::StatusCode;
use tokio_stream::StreamExt;
use tracing::warn;
use uuid::Uuid;

use crate::leptos_service::AppState;

use super::ensure_authorized_user;

pub async fn container_stats_ws(
    State(app_state): State<AppState>,
    jar: CookieJar,
    Path(project_id): Path<Uuid>,
    ws: WebSocketUpgrade,
) -> Result<Response, (axum::http::StatusCode, String)> {
    ensure_authorized_user(jar)?;
    let container = {
        let project = app_state
            .project_context
            .get_project(project_id)
            .await
            .ok_or((StatusCode::BAD_REQUEST, "Project doesnt exist".to_string()))?;
        let container = project
            .project_type
            .try_get_primary()
            .ok_or((StatusCode::BAD_REQUEST, "Project not container".to_string()))?
            .status
            .as_running()
            .ok_or((StatusCode::BAD_REQUEST, "Container not running".to_string()))?;
        container.clone()
    };

    Ok(ws.on_upgrade(|socket| handle_stats_socket(socket, container)))
}

async fn handle_stats_socket(mut socket: WebSocket, container: Arc<Container>) {
    let mut stat_stream = container.stats();
    // Get the first item to initialize state
    let mut previous_value = if let Some(Ok(initial_stats)) = stat_stream.next().await {
        // Send the initial full state
        if let Ok(serialized) = serde_json::to_string(&initial_stats) {
            if let Err(err) = socket.send(Message::Text(serialized.into())).await {
                warn!("Failed to send initial stats {err:?}");
                return;
            }
        }
        initial_stats
    } else {
        return;
    };

    loop {
        tokio::select! {
            rec = socket.recv() => {
                if rec.is_none() {
                    tracing::debug!("Exiting stats socket, ws closed");
                    break;
                }
            }
            Some(item) = stat_stream.next() => {
                match item {
                    Ok(item) => {
                        let patch = json_patch::diff(&previous_value, &item);
                        if let Ok(patch_serialized) = serde_json::to_string(&patch) {
                            if let Err(err) = socket.send(Message::Text(patch_serialized.into())).await {
                                warn!("Failed to send msg {err:?}");
                            } else {
                                previous_value = item;
                            }
                        }
                    }
                    Err(err) => {
                        warn!("Stats stream gave error {err:?}")
                    }
                }
            }
        }
    }
    // while let Some(item) = stat_stream.next().await {

    // }
}
