use std::sync::Arc;

use app::common::{get_podman, ContainerStatus, Project, ProjectType, PROJECTS};
use leptos::logging::warn;
use pingora::{
    server::ShutdownWatch,
    services::background::{background_service, BackgroundService, GenBackgroundService},
    upstreams::peer::HttpPeer,
};
use podman_api::{
    models::PortMapping,
    opts::{ContainerCreateOpts, ContainerDeleteOpts, ContainerStopOpts},
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
        let podman = get_podman();
        let image_id = format!("selfcloud_image_{}:latest", project.id.to_string());
        info!("Running Image id {image_id}");
        let image = podman.images().get(image_id);
        let is_image_available = match image.exists().await {
            Ok(result) => result,
            Err(err) => {
                warn!("Cant get image exists {err:?}");
                return Err(err)?;
            }
        };
        info!("Is image available {is_image_available}");
        if is_image_available {
            let id = format!("selfcloud_container_{}_latest", project.id.to_string());
            info!("Stopping old container");
            let _ = podman
                .containers()
                .get(&id)
                .stop(&ContainerStopOpts::builder().build())
                .await;
            info!("Removing old container");

            let _ = podman.containers().get(&id).remove().await;

            info!("Creating new container");

            let container = container.clone();
            let mut container_fut = tokio::spawn(async move {
                let podman = get_podman();
                podman
                    .containers()
                    .create(
                        &ContainerCreateOpts::builder()
                            .remove(true)
                            .name(id)
                            .image(image.id())
                            .publish_image_ports(true)
                            .portmappings(container.exposed_ports.iter().map(|p| PortMapping {
                                container_port: Some(p.port),
                                host_ip: None,
                                host_port: None,
                                protocol: None,
                                range: None,
                            }))
                            .build(),
                    )
                    .await
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

            let container = podman.containers().get(container.id);
            container.start(None).await?;
            let inspect = container.inspect().await?;

            {
                let mut projects = PROJECTS.write().await;
                let mut project = project.as_ref().clone();
                if let ProjectType::Container(cont) = &mut project.project_type {
                    if let Some(network) = inspect.network_settings {
                        if let Some(ports) = network.ports {
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
