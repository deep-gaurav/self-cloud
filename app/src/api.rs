use std::{collections::HashMap, sync::Arc};

use leptos::{expect_context, server, use_context, ServerFnError};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::common::{
    Container, DomainStatusFields, EnvironmentVar, ExposedPort, PortForward, Project, ProjectType,
    SupportContainer, Token,
};

#[server(InspectContainer)]
pub async fn inspect_container(
    id: Uuid,
) -> Result<docker_api_stubs::models::ContainerInspect200Response, ServerFnError> {
    user()?;
    let context = project_context()?;
    let project = context
        .get_project(id)
        .await
        .ok_or(ServerFnError::new("Not project with given id"))?;
    if let ProjectType::Container {
        primary_container: container,
        support_containers,
        tokens,
        exposed_ports,
    } = &project.project_type
    {
        if let crate::common::ContainerStatus::Running(container) = &container.status {
            let inspect = container.inspect().await?;
            Ok(inspect)
        } else {
            Err(ServerFnError::new("container not running"))
        }
    } else {
        Err(ServerFnError::new("project doesnt have container"))
    }
}

#[server(PauseContainer)]
pub async fn pause_container(id: Uuid) -> Result<(), ServerFnError> {
    user()?;

    let context = project_context()?;
    let project = context
        .get_project(id)
        .await
        .ok_or(ServerFnError::new("Not project with given id"))?;
    if let ProjectType::Container {
        primary_container: container,
        support_containers,
        tokens,
        exposed_ports,
    } = &project.project_type
    {
        if let crate::common::ContainerStatus::Running(container) = &container.status {
            container
                .pause()
                .await
                .map_err(|e| ServerFnError::new(format!("Cannot pause container {e:#?}")))?;
            Ok(())
        } else {
            Err(ServerFnError::new("container not running"))
        }
    } else {
        Err(ServerFnError::new("project doesnt have container"))
    }
}

#[server(ResumeContainer)]
pub async fn resume_container(id: Uuid) -> Result<(), ServerFnError> {
    user()?;
    let context = project_context()?;
    let project = context
        .get_project(id)
        .await
        .ok_or(ServerFnError::new("Not project with given id"))?;
    if let ProjectType::Container {
        primary_container: container,
        support_containers,
        tokens,
        exposed_ports,
    } = &project.project_type
    {
        if let crate::common::ContainerStatus::Running(container) = &container.status {
            container
                .unpause()
                .await
                .map_err(|e| ServerFnError::new(format!("Cannot resume container {e:?}")))?;
            Ok(())
        } else {
            Err(ServerFnError::new("container not running"))
        }
    } else {
        Err(ServerFnError::new("project doesnt have container"))
    }
}

#[server(StopContainer)]
pub async fn stop_container(id: Uuid) -> Result<(), ServerFnError> {
    user()?;

    let context = project_context()?;
    let project = context
        .get_project(id)
        .await
        .ok_or(ServerFnError::new("Not project with given id"))?;
    if let ProjectType::Container {
        primary_container: container,
        support_containers,
        tokens,
        exposed_ports,
    } = &project.project_type
    {
        if let crate::common::ContainerStatus::Running(container) = &container.status {
            container
                .stop(&docker_api::opts::ContainerStopOpts::builder().build())
                .await
                .map_err(|e| ServerFnError::new(format!("Cannot stop container {e:?}")))?;
            Ok(())
        } else {
            Err(ServerFnError::new("container not running"))
        }
    } else {
        Err(ServerFnError::new("project doesnt have container"))
    }
}

#[server(StartContainer)]
pub async fn start_container(id: Uuid) -> Result<(), ServerFnError> {
    user()?;

    let context = project_context()?;
    let project = context
        .get_project(id)
        .await
        .ok_or(ServerFnError::new("Not project with given id"))?;
    if let ProjectType::Container {
        primary_container: container,
        support_containers,
        tokens,
        exposed_ports,
    } = &project.project_type
    {
        if let crate::common::ContainerStatus::Running(container) = &container.status {
            container
                .start()
                .await
                .map_err(|e| ServerFnError::new(format!("Cannot start container {e:?}")))?;
            Ok(())
        } else {
            Err(ServerFnError::new("container not running"))
        }
    } else {
        Err(ServerFnError::new("project doesnt have container"))
    }
}

#[server(AddProject)]
pub async fn add_project(name: String) -> Result<Project, ServerFnError> {
    user()?;

    let mut project_context = project_context()?;
    let project = crate::common::add_port_forward_project(&name, 3000, &mut project_context)
        .await
        .map_err(|e| ServerFnError::new(e))?;

    Ok(project.as_ref().clone())
}

#[server(GetProjects)]
pub async fn get_projects() -> Result<Vec<Project>, ServerFnError> {
    user()?;
    let context = project_context()?;
    let projects = context.get_projects().await;

    let projects = projects
        .iter()
        .map(|e| e.as_ref().clone())
        .collect::<Vec<_>>();
    Ok(projects)
}

#[server(GetProject)]
pub async fn get_project(id: Uuid) -> Result<Project, ServerFnError> {
    user()?;
    let context = project_context()?;

    if let Some(project) = context.get_project(id).await {
        Ok(project.as_ref().clone())
    } else {
        use http::StatusCode;
        use leptos_axum::ResponseOptions;

        let response = expect_context::<ResponseOptions>();
        response.set_status(StatusCode::BAD_REQUEST);
        Err(ServerFnError::new("No Project with given id"))
    }
}

#[server(GetProjectDomains)]
pub async fn get_project_domains(
    id: Uuid,
) -> Result<HashMap<String, DomainStatusFields>, ServerFnError> {
    user()?;

    let context = project_context()?;
    let project_domains = context
        .get_project_domains(id)
        .await
        .into_iter()
        .map(|(d, status)| (d.to_lowercase(), status.into()))
        .collect();
    Ok(project_domains)
}

#[server(AddProjectDomain)]
pub async fn add_project_domain(id: Uuid, domain: String) -> Result<(), ServerFnError> {
    user()?;

    let mut project_context = project_context()?;

    let project = project_context
        .get_project(id)
        .await
        .ok_or(ServerFnError::new("Not project with given id"))?;

    project_context
        .add_project_domain(project, domain)
        .await
        .map_err(ServerFnError::new)?;
    Ok(())
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SupportContainerFields {
    pub name: String,
    pub image: String,
    pub env_vars: Option<HashMap<String, EnvironmentVar>>,
}

#[server(SetSupportContainers)]
pub async fn set_support_containers(
    id: Uuid,
    support_containers: Option<HashMap<String, SupportContainerFields>>,
) -> Result<(), ServerFnError> {
    user()?;

    let mut project_context = project_context()?;

    let mut project = project_context
        .get_project(id)
        .await
        .ok_or(ServerFnError::new("Not project with given id"))?
        .as_ref()
        .clone();

    if let ProjectType::Container {
        support_containers: support_c,
        ..
    } = &mut project.project_type
    {
        *support_c = HashMap::new();
        if let Some(support_containers_new) = support_containers {
            for container in support_containers_new.into_values() {
                support_c.insert(
                    container.name,
                    SupportContainer {
                        image: container.image,
                        container: Container {
                            env_vars: container
                                .env_vars
                                .map(|ev| ev.into_values().collect::<Vec<_>>().into())
                                .unwrap_or_default(),
                            status: crate::common::ContainerStatus::None,
                        },
                    },
                );
            }

            project_context
                .update_project(project.id, Arc::new(project))
                .await
                .map_err(ServerFnError::new)?;
        }
    } else {
        return Err(ServerFnError::new("project not container"));
    }
    Ok(())
}

#[server(UpdateProjectPort)]
pub async fn update_project_port(id: Uuid, port: u16) -> Result<(), ServerFnError> {
    user()?;

    let mut project_context = project_context()?;

    let project = project_context
        .get_project(id)
        .await
        .ok_or(ServerFnError::new("Not project with given id"))?;

    if let Err(_err) = stop_container(id).await {
        // Ignore fail
    }

    let new_project = Project {
        project_type: ProjectType::PortForward(PortForward::new(port)),
        ..project.as_ref().clone()
    };

    project_context
        .update_project(id, Arc::new(new_project))
        .await
        .map_err(ServerFnError::new)?;

    Ok(())
}

#[server(UpdateProjectImage)]
pub async fn update_project_image(
    id: Uuid,
    exposed_ports: Option<HashMap<String, ExposedPort>>,
    env_vars: Option<HashMap<String, EnvironmentVar>>,
    // tokens: Option<HashMap<String, Token>>,
) -> Result<(), ServerFnError> {
    user()?;

    let mut project_context = project_context()?;

    let project = project_context
        .get_project(id)
        .await
        .ok_or(ServerFnError::new("Not project with given id"))?;

    let tokens = if let ProjectType::Container {
        primary_container,
        support_containers,
        tokens,
        exposed_ports,
    } = &project.project_type
    {
        Some(tokens.clone())
    } else {
        None
    };

    let new_project = Project {
        project_type: ProjectType::Container {
            support_containers: if let ProjectType::Container {
                primary_container,
                support_containers,
                tokens,
                exposed_ports,
            } = &project.project_type
            {
                support_containers.clone()
            } else {
                HashMap::new()
            },
            exposed_ports: exposed_ports
                .map(|e| {
                    e.into_values()
                        .map(|mut p| {
                            p.domains.retain_mut(|d| !d.name.is_empty());
                            p
                        })
                        .collect()
                })
                .unwrap_or_default(),
            tokens: tokens.unwrap_or_default(),
            primary_container: Container {
                env_vars: env_vars
                    .map(|ev| ev.into_values().collect::<Vec<_>>().into())
                    .unwrap_or_default(),

                status: crate::common::ContainerStatus::None,
            },
        },
        ..project.as_ref().clone()
    };

    project_context
        .update_project(id, Arc::new(new_project))
        .await
        .map_err(ServerFnError::new)?;

    Ok(())
}

#[server(DeleteProject)]
pub async fn delete_project(id: Uuid) -> Result<(), ServerFnError> {
    user()?;

    let mut project_context = project_context()?;

    let project = project_context
        .get_project(id)
        .await
        .ok_or(ServerFnError::new("Not project with given id"))?;

    if let ProjectType::Container {
        primary_container: container,
        support_containers,
        tokens,
        exposed_ports,
    } = &project.project_type
    {
        if let Some(container) = container.status.as_running() {
            use docker_api::opts::{ContainerRemoveOpts, ContainerStopOpts};

            let _ = container.stop(&ContainerStopOpts::builder().build()).await;
            container
                .remove(&ContainerRemoveOpts::builder().force(true).build())
                .await
                .map_err(ServerFnError::new)?;
        }
    }

    project_context
        .remove_project(id)
        .await
        .map_err(ServerFnError::new)?;
    Ok(())
}

#[server(GetServerVersion)]
pub async fn get_server_version() -> Result<String, ServerFnError> {
    let hash: &str = env!("GIT_HASH");
    let version = env!("CARGO_PKG_VERSION");

    Ok(format!("{}-{}", version, hash))
}

#[server(UpdateProjectNameToken)]
pub async fn update_project_name_token(
    id: Uuid,
    project_name: String,
    tokens: Option<HashMap<String, Token>>,
) -> Result<(), ServerFnError> {
    user()?;

    let mut project_context = project_context()?;

    let project = project_context
        .get_project(id)
        .await
        .ok_or(ServerFnError::new("Not project with given id"))?;

    let new_project_type = match project.project_type.clone() {
        ProjectType::PortForward(port) => {
            if Some(true) == tokens.map(|t| !t.is_empty()) {
                return Err(ServerFnError::new(
                    "Project type port forward cant have tokens",
                ));
            };
            ProjectType::PortForward(port)
        }
        ProjectType::Container {
            primary_container: container,
            support_containers,
            tokens: old_tokens,
            exposed_ports,
        } => {
            // container.tokens = tokens.unwrap_or_default();
            ProjectType::Container {
                primary_container: container,
                support_containers,
                tokens: tokens.unwrap_or_default(),
                exposed_ports,
            }
        }
    };
    let new_project = Project {
        name: project_name,
        id,
        project_type: new_project_type,
    };
    project_context
        .update_project(id, Arc::new(new_project))
        .await
        .map_err(ServerFnError::new)?;
    Ok(())
}

#[cfg(feature = "ssr")]
pub fn user() -> Result<crate::auth::User, ServerFnError> {
    use crate::auth::AuthType;
    use http::StatusCode;
    use leptos_axum::ResponseOptions;

    let auth = use_context::<AuthType>().ok_or(ServerFnError::new("User Not present"))?;

    match auth {
        AuthType::UnAuthorized => {
            let response = expect_context::<ResponseOptions>();
            response.set_status(StatusCode::UNAUTHORIZED);
            Err(ServerFnError::new("UnAuthorized"))
        }
        AuthType::Authorized(user) => Ok(user),
    }
}

#[cfg(feature = "ssr")]
pub fn project_context() -> Result<crate::context::ProjectContext, ServerFnError> {
    let context = use_context::<crate::context::ProjectContext>()
        .ok_or(ServerFnError::new("Project Context not present"))?;

    Ok(context)
}
