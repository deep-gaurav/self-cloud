use std::sync::Arc;

use app::common::{Project, ProjectType, PROJECTS};
use leptos::logging::warn;
use pingora::{
    server::ShutdownWatch,
    services::background::{background_service, BackgroundService, GenBackgroundService},
};
use rustainers::runner::{RunOption, Runner};
use tracing::{error, info};

pub struct ContainerManager {}

impl ContainerManager {
    pub fn to_service() -> GenBackgroundService<Self> {
        background_service("container_manager", Self {})
    }
}

#[async_trait::async_trait]
impl BackgroundService for ContainerManager {
    async fn start(&self, mut shutdown: ShutdownWatch) {
        let runner = match Runner::podman() {
            Ok(runner) => runner,
            Err(err) => {
                error!("Podman runner couldnt be started {err:?}");
                panic!("Podman runner couldnt be started {err:?}");
            }
        };

        let mut period = tokio::time::interval(std::time::Duration::from_secs(5));

        loop {
            tokio::select! {
                _ = shutdown.changed() => {
                    info!("Shutdown received");
                    break;
                }
                _ = period.tick() => {
                    let project = 'ba: {
                        let mut peers = PROJECTS.write().unwrap();
                        for (id, project) in peers.iter_mut() {
                            if let ProjectType::Container(container) = &project.project_type {
                                if container.status.is_none(){
                                    break 'ba Some(project.clone())
                                }
                            }
                        }
                        None
                    };
                    if let Some(project) = project {
                        let runner = runner.clone();
                        tokio::spawn(async move {
                            if let Err(err) = run_and_set_container(runner, project).await {
                                warn!("Failed to run container {err:?}");
                            }
                        });
                    }
                }
            };
        }
    }
}

async fn run_and_set_container(runner: Runner, project: Arc<Project>) -> anyhow::Result<()> {
    if let ProjectType::Container(container) = &project.project_type {
        if container.status.is_none() {
            let container = runner
                .start_with_options(
                    container.clone(),
                    RunOption::builder()
                        .with_name(format!("selfcloud_{}", project.id))
                        .build(),
                )
                .await?;
            {
                let mut projects = PROJECTS.write().unwrap();
                let mut project = project.as_ref().clone();
                if let ProjectType::Container(cont) = &mut project.project_type {
                    cont.status = Some(Arc::new(container))
                }
                projects.insert(project.id, Arc::new(project));
            }
        }
    }

    Ok(())
}
