use std::{
    io::Cursor,
    sync::Arc,
};

use app::common::{get_docker, ContainerStatus, ProjectType, PROJECTS};
use axum::{
    extract::Multipart,
    response::{IntoResponse, Response},
};
use docker_api::opts::TagOpts;
use futures::stream::StreamExt;
use http::StatusCode;
use tracing::{info, warn};
use uuid::Uuid;

pub async fn push_image(mut multipart: Multipart) -> Result<(StatusCode, String), PushError> {
    let mut token = None;
    let mut project_id = None;
    while let Some(field) = multipart.next_field().await? {
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
                    let projects = PROJECTS.read().await;
                    let project = projects
                        .get(&project_id)
                        .ok_or(anyhow::anyhow!("project with given id not present"))?;

                    if let ProjectType::Container(container) = &project.project_type {
                        if let Some(token) = container.tokens.get(&token) {
                            if let Some(expiry) = &token.expiry {
                                let current_date = chrono::Utc::now().naive_utc().date();
                                if &current_date > expiry {
                                    return Err(anyhow::anyhow!("Project token not valid").into());
                                }
                            }
                        } else {
                            return Err(anyhow::anyhow!("Project token not valid").into());
                        }
                    } else {
                        return Err(anyhow::anyhow!("Project not of type container").into());
                    }
                };
                let data = field.bytes().await?;
                let id = format!("selfcloud_image_{}", project_id.to_string());
                info!("Uploading image to {id}");
                let docker = get_docker();
                let reader = Cursor::new(data);
                let image = 'ba: {
                    let images = docker.images();
                    let mut stream = images.import(reader);
                    while let Some(data) = stream.next().await {
                        match data {
                            Ok(data) => match data {
                                docker_api::models::ImageBuildChunk::Update { stream } => {
                                    let reg = regex_macro::regex!(r"(?m)Loaded image: (.*)");
                                    let capture = reg.captures(&stream).and_then(|c| c.get(1));
                                    if let Some(capture) = capture {
                                        break 'ba Ok(capture.as_str().to_string());
                                    }
                                }
                                docker_api::models::ImageBuildChunk::Error {
                                    error,
                                    error_detail,
                                } => {
                                    break 'ba Err(docker_api::Error::Any("failed".into()));
                                }
                                docker_api::models::ImageBuildChunk::Digest { aux } => {
                                    break 'ba Ok(aux.id.to_string());
                                }
                                docker_api::models::ImageBuildChunk::PullStatus {
                                    status,
                                    id,
                                    progress,
                                    progress_detail,
                                } => {}
                            },
                            Err(err) => {
                                break 'ba Err(err);
                            }
                        }
                    }
                    Err(docker_api::Error::Any("failed".into()))
                };
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
                    let mut projects = PROJECTS.write().await;
                    let project = projects.get(&project_id);
                    if let Some(project) = project {
                        let mut proj = project.as_ref().clone();
                        if let ProjectType::Container(container) = &mut proj.project_type {
                            container.status = ContainerStatus::None;
                        }

                        projects.insert(project_id, Arc::new(proj));
                    }
                }
                return Ok((StatusCode::OK, format!("Accepted")));
            }
            name => return Ok((StatusCode::BAD_REQUEST, format!("Unknown field {name:?}"))),
        }
    }
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
