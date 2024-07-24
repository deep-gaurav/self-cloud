use std::{io::Cursor, sync::Arc};

use app::common::{get_docker, ContainerStatus, ProjectType};
use axum::{
    extract::{Multipart, State},
    response::{IntoResponse, Response},
};
use docker_api::opts::TagOpts;
use futures::stream::StreamExt;
use http::StatusCode;
use tracing::{info, warn};
use uuid::Uuid;

use crate::leptos_service::AppState;

#[axum::debug_handler]
pub async fn push_image(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<(StatusCode, String), PushError> {
    let mut context = state.project_context;
    let mut token = None;
    let mut project_id = None;
    loop {
        let field = multipart.next_field().await?;
        if let Some(mut field) = field {
            let name = field
                .name()
                .ok_or(anyhow::anyhow!("Unnamed field"))?
                .to_string();
            match name.as_str() {
                "token" => {
                    token = Some(field.text().await?);
                }

                "project_id" => project_id = Some(Uuid::parse_str(field.text().await?.as_str())?),

                "image" => {
                    let Some(token) = token else {
                        return Ok((StatusCode::BAD_REQUEST, format!("No Upload Token")));
                    };
                    let Some(project_id) = project_id else {
                        return Ok((StatusCode::BAD_REQUEST, format!("No Project Id")));
                    };

                    {
                        let project = context
                            .get_project(project_id)
                            .await
                            .ok_or(anyhow::anyhow!("project with given id not present"))?;

                        if let ProjectType::Container { tokens, .. } = &project.project_type {
                            if let Some(token) = tokens.get(&token) {
                                if let Some(expiry) = &token.expiry {
                                    let current_date = chrono::Utc::now().naive_utc().date();
                                    if &current_date > expiry {
                                        return Err(
                                            anyhow::anyhow!("Project token not valid").into()
                                        );
                                    }
                                }
                            } else {
                                return Err(anyhow::anyhow!("Project token not valid").into());
                            }
                        } else {
                            return Err(anyhow::anyhow!("Project not of type container").into());
                        }
                    };
                    let docker = get_docker();
                    let images = docker.images();
                    let (tx, rx) = tokio::sync::mpsc::channel(5);

                    let id = format!("selfcloud_image_{}", project_id.to_string());
                    info!("Uploading image to {id}");

                    let read_field_fut = async move {
                        while let Some(val) = field.next().await {
                            if let Err(er) = tx.send(val).await {
                                tracing::warn!("Receiver ended {er:?}");
                                break;
                            }
                        }
                    };
                    let stream = tokio_stream::wrappers::ReceiverStream::new(rx);
                    let write_docker_fut = async {
                        'ba: {
                            let mut stream = images.import_from_stream(stream);
                            while let Some(data) = stream.next().await {
                                match data {
                                    Ok(data) => match data {
                                        docker_api::models::ImageBuildChunk::Update { stream } => {
                                            let reg =
                                                regex_macro::regex!(r"(?m)Loaded image: (.*)");
                                            let capture =
                                                reg.captures(&stream).and_then(|c| c.get(1));
                                            if let Some(capture) = capture {
                                                break 'ba Ok(capture.as_str().to_string());
                                            }
                                        }
                                        docker_api::models::ImageBuildChunk::Error {
                                            error: _,
                                            error_detail: _,
                                        } => {
                                            break 'ba Err(docker_api::Error::Any("failed".into()));
                                        }
                                        docker_api::models::ImageBuildChunk::Digest { aux } => {
                                            break 'ba Ok(aux.id.to_string());
                                        }
                                        docker_api::models::ImageBuildChunk::PullStatus {
                                            status: _,
                                            id: _,
                                            progress: _,
                                            progress_detail: _,
                                        } => {}
                                    },
                                    Err(err) => {
                                        break 'ba Err(err);
                                    }
                                }
                            }
                            Err(docker_api::Error::Any("failed".into()))
                        }
                    };
                    let (_, image) = tokio::join!(read_field_fut, write_docker_fut);

                    let image = match image {
                        Ok(image) => image,
                        Err(err) => {
                            tracing::error!("Failed to load image {err:?}");
                            return Err(err)?;
                        }
                    };
                    let tag = image;
                    let image = docker.images().get(tag);
                    if let Err(err) = image
                        .tag(&TagOpts::builder().repo(id).tag("latest").build())
                        .await
                    {
                        warn!("Cannot tag image {err:?}")
                    }

                    info!("Loaded docker image {image:?}");
                    {
                        let project = context.get_project(project_id).await;
                        if let Some(project) = project {
                            let mut proj = project.as_ref().clone();
                            if let ProjectType::Container {
                                primary_container: container,
                                ..
                            } = &mut proj.project_type
                            {
                                container.status = ContainerStatus::None;
                            }

                            if let Err(err) =
                                context.update_project(project_id, Arc::new(proj)).await
                            {
                                warn!("Failed to update project status {err:?}");
                            }
                        }
                    }
                    return Ok((StatusCode::OK, format!("Accepted")));
                }
                name => return Ok((StatusCode::BAD_REQUEST, format!("Unknown field {name:?}"))),
            }
        } else {
            break;
        }
    }
    // while let Some(field) = multipart.next_field().await?
    Ok((StatusCode::BAD_REQUEST, format!("No image field")))
}

// Make our own error that wraps `anyhow::Error`.
pub struct PushError(anyhow::Error);

// Tell axum how to convert `AppError` into a response.
impl IntoResponse for PushError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.0),
        )
            .into_response()
    }
}

// This enables using `?` on functions that return `Result<_, anyhow::Error>` to turn them into
// `Result<_, AppError>`. That way you don't need to do that manually.
impl<E> From<E> for PushError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}
