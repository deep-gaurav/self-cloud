use std::sync::Arc;

use app::common::{AttachParams, TtyChunk, PROJECTS};
use axum::{
    extract::{
        ws::{Message, WebSocket},
        Path, Query, WebSocketUpgrade,
    },
    response::Response,
};
use docker_api::{
    opts::{ConsoleSize, ExecCreateOpts, ExecStartOpts},
    Container,
};
use futures::{AsyncWriteExt, SinkExt, StreamExt};
use http::StatusCode;
use tower_cookies::Cookies;
use tracing::warn;
use uuid::Uuid;

use super::ensure_authorized_user;

pub async fn container_attach_ws(
    cookies: Cookies,
    Path(project_id): Path<Uuid>,
    Query(attach_params): Query<AttachParams>,
    ws: WebSocketUpgrade,
) -> Result<Response, (axum::http::StatusCode, String)> {
    ensure_authorized_user(cookies)?;
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

    Ok(ws.on_upgrade(|socket| handle_attach_socket(socket, container, attach_params)))
}

async fn handle_attach_socket(
    socket: WebSocket,
    container: Arc<Container>,
    attach_params: AttachParams,
) {
    let exec_multiplexer = container
        .exec(
            &ExecCreateOpts::builder()
                .command(vec![attach_params.command])
                .attach_stdout(true)
                .attach_stdout(true)
                .attach_stdin(true)
                .working_dir("/")
                .tty(true)
                .console_size(ConsoleSize {
                    height: attach_params.size_height,
                    width: attach_params.size_width,
                })
                .build(),
            &ExecStartOpts::builder()
                .tty(true)
                .console_size(ConsoleSize {
                    height: attach_params.size_height,
                    width: attach_params.size_width,
                })
                .build(),
        )
        .await;
    match exec_multiplexer {
        Ok(multiplexer) => {
            let (mut receiver_out, mut input_sender) = multiplexer.split();
            let (mut sender_sock, mut receiver_sock) = socket.split();
            let (quit_sender, mut quit_receiver) = tokio::sync::broadcast::channel(1);
            let (quit_sender2, mut quit_receiver2) = tokio::sync::broadcast::channel(1);
            let ws_receiver = tokio::spawn(async move {
                loop {
                    tokio::select! {
                        _ = quit_receiver2.recv() => {
                            tracing::debug!("Sender quit, quitting");
                            break;
                        }
                        rec = receiver_sock.next() => {
                            match rec {
                                Some(data) => {
                                    match data {
                                        Ok(data) => {
                                            match data {
                                                Message::Text(text) => {
                                                    // tracing::info!("Sending {text:?}");
                                                    if let Err(err) =  input_sender.write_all(text.as_bytes()).await{
                                                        warn!("Cant send intput {err:?}");
                                                    }
                                                },
                                                Message::Binary(data) => {
                                                    if let Err(err) =  input_sender.write_all(&data).await{
                                                        warn!("Cant send intput {err:?}");
                                                    }
                                                },
                                                Message::Ping(_) |
                                                Message::Pong(_) => {
                                                    //ignore ping pong
                                                },
                                                Message::Close(_) => {
                                                    tracing::debug!("Exiting stats socket, ws requested closed");
                                                    break;
                                                },
                                            }
                                        },
                                        Err(err) => {
                                            warn!("Error receiving ws input {err:?}");
                                        },
                                    }
                                },
                                None => {
                                    if let Err(err) =  quit_sender.send(()){
                                        tracing::warn!("Cant send quit {err:?}");
                                    }
                                    tracing::debug!("Exiting stats socket, ws closed");
                                    break;
                                }
                            }
                            //Ignore for now
                        }
                    }
                }
            });
            loop {
                tokio::select! {

                    _ = quit_receiver.recv() => {
                        tracing::debug!("Receiver quit, quitting");
                        break;
                    }
                    Some(item) = receiver_out.next() => {
                        match item {
                            Ok(item) => {
                                let item = TtyChunk::from(item);
                                if let Ok(serialized_data) = bincode::serialize(&item) {
                                    if let Err(err) = sender_sock.send(Message::Binary(serialized_data)).await {
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

                    else => {
                        tracing::info!("attach out exited, quitting");
                        if let Err(err) =  quit_sender2.send(()){
                            tracing::warn!("Cant send quit {err:?}");
                        }

                        break;
                    }
                }
            }

            tracing::info!("Sender exited, waiting for ws receiver to exit");
            if let Err(err) = ws_receiver.await {
                tracing::warn!("tokio join ws receiver error {err:?}")
            }
            tracing::info!("ws receiver exited");
        }
        Err(err) => {
            warn!("Failed to start exec {err:?}")
        }
    }

    tracing::info!("Exited attach");
}
