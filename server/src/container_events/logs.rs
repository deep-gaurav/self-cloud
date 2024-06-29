use std::sync::Arc;

use app::common::{TtyChunk, PROJECTS};
use axum::{
    extract::{
        ws::{Message, WebSocket},
        Path, WebSocketUpgrade,
    },
    response::Response,
};
use docker_api::{opts::LogsOpts, Container};
use http::StatusCode;
use tokio_stream::StreamExt;
use tracing::warn;
use uuid::Uuid;

pub async fn container_logs_ws(
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

    Ok(ws.on_upgrade(|socket| handle_logs_socket(socket, container)))
}

async fn handle_logs_socket(mut socket: WebSocket, container: Arc<Container>) {
    let mut logs_stream = container.logs(
        &LogsOpts::builder()
            .follow(true)
            .stderr(true)
            .stdout(true)
            .build(),
    );
    loop {
        tokio::select! {
            rec = socket.recv() => {
                if rec.is_none() {
                    tracing::debug!("Exiting stats socket, ws closed");
                    break;
                }
                //Ignore for now
            }
            Some(item) = logs_stream.next() => {
                match item {
                    Ok(item) => {
                        let item = TtyChunk::from(item);
                        if let Ok(serialized_data) = bincode::serialize(&item) {
                            if let Err(err) = socket.send(Message::Binary(serialized_data)).await {
                                warn!("Failed to send msg {err:?}");
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
