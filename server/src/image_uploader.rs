use std::sync::Arc;

use app::common::{get_podman, ContainerStatus, ProjectType, PROJECTS};
use axum::{
    extract::Multipart,
    response::{IntoResponse, Response},
};
use http::StatusCode;
use podman_api::opts::{ImageImportOpts, ImageTagOpts};
use tracing::{info, warn};
use uuid::Uuid;

pub async fn push_image(mut multipart: Multipart) -> Result<(StatusCode, String), PushError> {
    let mut token = None;
    let mut project_id = None;
    while let Some(mut field) = multipart.next_field().await? {
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
                if token.is_none() {
                    return Ok((StatusCode::BAD_REQUEST, format!("No Upload Token")));
                }
                let Some(project_id) = project_id else {
                    return Ok((StatusCode::BAD_REQUEST, format!("No Project Id")));
                };
                let data = field.bytes().await?;
                let id = format!("selfcloud_image_{}", project_id.to_string());
                info!("Uploading image to {id}");
                let podman = get_podman();
                let image = podman.images().load(data).await;
                let image = match image {
                    Ok(image) => image,
                    Err(err) => {
                        tracing::error!("Failed to load image {err:?}");
                        return Err(err)?;
                    }
                };
                let tag = image
                    .names
                    .and_then(|s| s.first().map(|p| p.to_string()))
                    .ok_or(anyhow::anyhow!("No imported tag"))?;
                let image = podman.images().get(tag);
                if let Err(err) = image
                    .tag(&ImageTagOpts::builder().repo(id).tag("latest").build())
                    .await
                {
                    warn!("Cannot tag image {err:?}")
                }

                info!("Loaded podman image {image:?}");
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
