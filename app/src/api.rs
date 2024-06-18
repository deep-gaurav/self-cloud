use std::{borrow::Borrow, collections::HashMap, sync::Arc};

use anyhow::anyhow;
use leptos::{expect_context, server, use_context, ServerFnError};
use uuid::Uuid;

use crate::common::{
    Container, DomainStatusFields, ExposedPort, PortForward, Project, ProjectType,
};

#[server(InspectContainer)]
pub async fn inspect_container(
    id: Uuid,
) -> Result<docker_api_stubs::models::ContainerInspect200Response, ServerFnError> {
    user()?;
    let project = get_project_arc(id).await.map_err(ServerFnError::new)?;
    if let ProjectType::Container(container) = &project.project_type {
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

#[server(AddProject)]
pub async fn add_project(name: String) -> Result<Project, ServerFnError> {
    use crate::common::PROJECTS;

    user()?;
    let id = uuid::Uuid::new_v4();

    let reserve_port = 3000;

    let project = Project {
        id,
        name,
        project_type: ProjectType::PortForward(PortForward::new(reserve_port)),
    };
    {
        let mut projects = PROJECTS.write().await;

        projects.insert(id, Arc::new(project.clone()));
    }
    crate::common::save_project_config()
        .await
        .map_err(ServerFnError::new)?;
    Ok(project)
}

#[server(GetProjects)]
pub async fn get_projects() -> Result<Vec<Project>, ServerFnError> {
    use crate::common::PROJECTS;

    user()?;

    let projects = PROJECTS.read().await;

    let projects = projects
        .iter()
        .map(|e| e.1.as_ref().clone())
        .collect::<Vec<_>>();
    Ok(projects)
}

#[server(GetProject)]
pub async fn get_project(id: Uuid) -> Result<Project, ServerFnError> {
    user()?;

    if let Ok(project) = get_project_arc(id).await {
        Ok(project.as_ref().clone())
    } else {
        use http::StatusCode;
        use leptos_axum::ResponseOptions;

        let response = expect_context::<ResponseOptions>();
        response.set_status(StatusCode::BAD_REQUEST);
        Err(ServerFnError::new("No Project with given id"))
    }
}

#[cfg(feature = "ssr")]
pub async fn get_project_arc(id: Uuid) -> anyhow::Result<std::sync::Arc<Project>> {
    use std::sync::Arc;

    use crate::common::PROJECTS;

    let projects = PROJECTS.read().await;

    let project = projects.get(&id);
    if let Some(project) = project {
        Ok(project.clone())
    } else {
        Err(anyhow::anyhow!("No project with given id"))
    }
}

#[server(GetProjectDomains)]
pub async fn get_project_domains(
    id: Uuid,
) -> Result<HashMap<String, DomainStatusFields>, ServerFnError> {
    user()?;

    use crate::common::DOMAIN_MAPPING;

    let domains = {
        DOMAIN_MAPPING
            .read()
            .map_err(|e| ServerFnError::new(anyhow!("{e:?}")))?
            .clone()
    };

    let mut project_domains = HashMap::new();
    for (domain, status) in domains.iter() {
        if let Some(project) = status.get_project().await {
            if project.id == id {
                project_domains.insert(domain.to_lowercase(), status.clone().into());
            }
        }
    }
    Ok(project_domains)
}

#[server(AddProjectDomain)]
pub async fn add_project_domain(id: Uuid, domain: String) -> Result<(), ServerFnError> {
    user()?;

    let project = get_project_arc(id).await.map_err(ServerFnError::new)?;

    super::common::add_project_domain(project, domain)
        .await
        .map_err(ServerFnError::new)?;

    crate::common::save_project_config()
        .await
        .map_err(ServerFnError::new)?;
    Ok(())
}

#[server(UpdateProjectPort)]
pub async fn update_project_port(id: Uuid, port: u16) -> Result<(), ServerFnError> {
    user()?;

    let project = get_project_arc(id).await.map_err(ServerFnError::new)?;

    let new_project = Project {
        project_type: ProjectType::PortForward(PortForward::new(port)),
        ..project.as_ref().clone()
    };

    {
        let mut projects = crate::common::PROJECTS.write().await;

        projects.insert(project.id, Arc::new(new_project.clone()));
    }

    crate::common::save_project_config()
        .await
        .map_err(ServerFnError::new)?;
    Ok(())
}

#[server(UpdateProjectImage)]
pub async fn update_project_image(
    id: Uuid,
    container_port: u16,
    domain: String,
) -> Result<(), ServerFnError> {
    user()?;

    let project = get_project_arc(id).await.map_err(ServerFnError::new)?;

    let new_project = Project {
        project_type: ProjectType::Container(Container {
            exposed_ports: vec![ExposedPort {
                port: container_port,
                domains: vec![crate::common::Domain {
                    name: unicase::UniCase::from(domain),
                }],
                #[cfg(feature = "ssr")]
                peer: None,
            }],
            status: crate::common::ContainerStatus::None,
        }),
        ..project.as_ref().clone()
    };

    {
        let mut projects = crate::common::PROJECTS.write().await;

        projects.insert(project.id, Arc::new(new_project.clone()));
    }

    crate::common::save_project_config()
        .await
        .map_err(ServerFnError::new)?;
    Ok(())
}

#[cfg(feature = "ssr")]
pub fn user() -> Result<crate::auth::User, ServerFnError> {
    use crate::auth::AuthType;
    use http::StatusCode;
    use leptos_axum::ResponseOptions;

    let auth = expect_context::<AuthType>();

    match auth {
        AuthType::UnAuthorized => {
            let response = expect_context::<ResponseOptions>();
            response.set_status(StatusCode::UNAUTHORIZED);
            Err(ServerFnError::new("UnAuthorized"))
        }
        AuthType::Authorized(user) => Ok(user),
    }
}
