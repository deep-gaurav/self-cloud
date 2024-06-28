use std::sync::Arc;

use app::common::PROJECTS;
use axum::{
    extract::{
        ws::{Message, WebSocket},
        Path, WebSocketUpgrade,
    },
    response::{
        sse::{Event, KeepAlive},
        Response, Sse,
    },
};
use docker_api::Container;
use futures::{Stream, TryStreamExt};
use http::StatusCode;
use tokio_stream::StreamExt;
use tracing::warn;
use uuid::Uuid;

pub async fn container_stats_ws(
    Path(project_id): Path<Uuid>,
    ws: WebSocketUpgrade,
) -> Result<Response, (axum::http::StatusCode, String)> {
    let container = {
        let projects = PROJECTS.read().await;
        let project = projects
            .get(&project_id)
            .ok_or((StatusCode::BAD_REQUEST, "Project doesnt exist".to_string()))?;
        let container = project
            .project_type
            .as_container()
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
    let mut previous_value = serde_json::Value::Null;
    loop {
        tokio::select! {
            rec = socket.recv() => {
                if rec.is_none() {
                    tracing::debug!("Exiting stats socket, ws closed");
                    break;
                }
                //Ignore for now
            }
            Some(item) = stat_stream.next() => {
                match item {
                    Ok(item) => {
                        let patch = json_patch::diff(&previous_value, &item);
                        if let Ok(patch_serialized) = serde_json::to_string(&patch) {
                            if let Err(err) = socket.send(Message::Text(patch_serialized)).await {
                                warn!("Failed to send msg {err:?}");
                            } else {
                                previous_value = item;
                            }
                        }
                    }
                    Err(err) => {
                        // yield Err(axum::BoxError::new(anyhow::anyhow!("{err:#?}")))
                    }
                }
            }
        }
    }
    // while let Some(item) = stat_stream.next().await {

    // }
}
