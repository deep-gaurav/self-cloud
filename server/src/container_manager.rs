use std::sync::Arc;

use app::common::{get_docker, ContainerStatus, Project, ProjectType, PROJECTS};
use docker_api::opts::{ContainerCreateOpts, ContainerRemoveOpts, ContainerStopOpts, PublishPort};
use leptos::logging::warn;
use pingora::{
    server::ShutdownWatch,
    services::background::{background_service, BackgroundService, GenBackgroundService},
    upstreams::peer::HttpPeer,
};
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
        let mut period = tokio::time::interval(std::time::Duration::from_secs(5));

        loop {
            tokio::select! {
                _ = shutdown.changed() => {
                    info!("Shutdown received");
                    break;
                }
                _ = period.tick() => {
                    let mut peers = PROJECTS.write().await;
                    for (id, project) in peers.iter_mut() {
                        if let ProjectType::Container(container) = &project.project_type {
                            if container.status.is_none(){
                                let mut project_t = project.clone().as_ref().clone();
                                if let ProjectType::Container( container) = &mut project_t.project_type{
                                    container.status = ContainerStatus::Creating;
                                }
                                *project = Arc::new(project_t);
                                let project = project.clone();
                                tokio::spawn(async move {
                                    let id = project.id;
                                    if let Err(err) = run_and_set_container(project).await {
                                        warn!("Failed to run container {err:?}");

                                        {
                                            let mut projects = PROJECTS.write().await;
                                            if let Some(project) = projects.get_mut(&id){
                                                let mut new_p = project.as_ref().clone();
                                                if let ProjectType::Container(container) = &mut new_p.project_type {
                                                    container.status = ContainerStatus::Failed;
                                                }
                                                *project = Arc::new(new_p)
                                            }
                                        }
                                    }
                                });
                            }
                        }
                    }
                }
            };
        }
    }
}

async fn run_and_set_container(project: Arc<Project>) -> anyhow::Result<()> {
    if let ProjectType::Container(container) = &project.project_type {
        if let ContainerStatus::Running(container) = &container.status {
            container
                .stop(&ContainerStopOpts::builder().build())
                .await?;
        }
        let docker = get_docker();
        let image_id = format!("selfcloud_image_{}:latest", project.id.to_string());
        info!("Running Image id {image_id}");
        let image = docker.images().get(image_id);
        let is_image_available = image.inspect().await.is_ok();
        info!("Is image available {is_image_available}");
        if is_image_available {
            let id = format!("selfcloud_container_{}_latest", project.id.to_string());
            info!("Stopping old container");
            let _ = docker
                .containers()
                .get(&id)
                .stop(&ContainerStopOpts::builder().build())
                .await;
            info!("Removing old container");

            let _ = docker
                .containers()
                .get(&id)
                .remove(&ContainerRemoveOpts::builder().volumes(true).build())
                .await;

            info!("Creating new container");

            let container = container.clone();
            let mut container_fut = tokio::spawn(async move {
                let docker = get_docker();
                let mut builder = ContainerCreateOpts::builder()
                    .auto_remove(true)
                    // .image_arch("amd64")
                    .name(id)
                    .image(image.name())
                    .publish_all_ports();

                for expose_port in container.exposed_ports.iter() {
                    builder = builder.publish(PublishPort::tcp(expose_port.port as u32));
                }
                docker.containers().create(&builder.build()).await
            });
            let container = loop {
                tokio::select! {
                    _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {
                        info!("Container creator is running...");
                    }
                    result = &mut container_fut => {
                        match result {
                            Ok(result) => match result {
                                Ok(container) => break container,
                                Err(err) => {
                                    warn!("Failed to create container {err:?}");
                                    return Err(err)?;
                                }
                            },
                            Err(err) => {
                                warn!("Failed to create container {err:?}");
                                return Err(err)?;
                            },
                        }

                    }
                }
            };
            info!("Container created, running");

            let container = docker.containers().get(container.id().clone());
            container.start().await?;
            info!("Container started");
            let inspect = container.inspect().await?;

            {
                let mut projects = PROJECTS.write().await;
                let mut project = project.as_ref().clone();
                if let ProjectType::Container(cont) = &mut project.project_type {
                    if let Some(network) = inspect.network_settings {
                        if let Some(ports) = network.ports {
                            tracing::info!("Container running with ports {ports:#?}");
                            for port in cont.exposed_ports.iter_mut() {
                                let port_q = format!("{}/tcp", port.port);
                                let exposed_port = ports.get(&port_q);
                                if let Some(host_port) = exposed_port
                                    .and_then(|p| p.to_owned())
                                    .and_then(|p| p.first().cloned())
                                    .and_then(|p| p.host_port)
                                    .and_then(|p| p.parse::<u16>().ok())
                                {
                                    port.peer = Some(Box::new(HttpPeer::new(
                                        format!("0.0.0.0:{host_port}"),
                                        false,
                                        String::new(),
                                    )))
                                }
                            }
                        }
                    }
                    cont.status = ContainerStatus::Running(Arc::new(container));
                }
                projects.insert(project.id, Arc::new(project));
            }
        } else {
            info!("No image found")
        }
    }

    Ok(())
}
