use std::sync::Arc;

use app::common::TtyChunk;
use axum::{
    extract::{
        ws::{Message, WebSocket},
        Path, State, WebSocketUpgrade,
    },
    response::Response,
};
use axum_extra::extract::cookie::CookieJar;
use docker_api::{opts::LogsOpts, Container};
use http::StatusCode;
use tokio_stream::StreamExt;
use tracing::warn;
use uuid::Uuid;

use crate::leptos_service::AppState;

use super::ensure_authorized_user;

pub async fn container_logs_ws(
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
                            if let Err(err) = socket.send(Message::Binary(serialized_data.into())).await {
                                warn!("Failed to send msg {err:?}");
                            }
                        }
                    }
                    Err(err) => {
                        warn!("Log Stream gave error {err:?}");
                        // yield Err(axum::BoxError::new(anyhow::anyhow!("{err:#?}")))
                    }
                }
            }
        }
    }
    // while let Some(item) = stat_stream.next().await {

    // }
}
