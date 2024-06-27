use app::common::PROJECTS;
use axum::{
    extract::Path,
    response::{
        sse::{Event, KeepAlive},
        Sse,
    },
};
use futures::{Stream, TryStreamExt};
use http::StatusCode;
use tokio_stream::StreamExt;
use uuid::Uuid;

pub async fn container_stats_see(
    Path(project_id): Path<Uuid>,
) -> Result<Sse<impl Stream<Item = Result<Event, axum::BoxError>>>, (axum::http::StatusCode, String)>
{
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

    let stream = async_stream::stream! {
        let mut stat_stream = container.stats();
        while let Some(item) = stat_stream.next().await {
            match item {
                Ok(item) => {
                    yield Ok(item)
                }
                Err(err) => {
                    // yield Err(axum::BoxError::new(anyhow::anyhow!("{err:#?}")))
                }
            }
        }
    };

    use leptos_sse::ServerSentEvents;
    use std::time::Duration;

    let mut value = 0;
    let stream = ServerSentEvents::new("stats", stream).unwrap();
    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}
