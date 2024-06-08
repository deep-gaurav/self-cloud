use std::{borrow::Borrow, collections::HashMap, sync::Arc};

use anyhow::anyhow;
use leptos::{expect_context, server, use_context, ServerFnError};
use uuid::Uuid;

use crate::common::{Container, DomainStatusFields, Project, ProjectType};

#[server(AddProject)]
pub async fn add_project(name: String) -> Result<Project, ServerFnError> {
    use crate::common::PROJECTS;

    user()?;
    let id = uuid::Uuid::new_v4();

    let reserve_port = 3000;

    let project = Project {
        id,
        name,
        peer: Box::new(pingora::upstreams::peer::HttpPeer::new(
            format!("0.0.0.0:{reserve_port}"),
            false,
            String::new(),
        )),
        project_type: ProjectType::PortForward(3000),
    };
    {
        let mut projects = PROJECTS
            .write()
            .map_err(|e| ServerFnError::new(anyhow!("{e:?}")))?;

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

    let projects = PROJECTS
        .read()
        .map_err(|e| ServerFnError::new(anyhow!("{e:?}")))?;

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

    let projects = PROJECTS.read().map_err(|e| anyhow!("{e:?}"))?;

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

    let domains = DOMAIN_MAPPING
        .read()
        .map_err(|e| ServerFnError::new(anyhow!("{e:?}")))?;

    let project_domains = domains
        .iter()
        .filter_map(|(domain, status)| {
            if let Some(project) = status.project.upgrade() {
                if project.id == id {
                    Some((domain.to_lowercase(), status.clone().into()))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect::<HashMap<_, _>>();
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
        project_type: ProjectType::PortForward(port),
        peer: Box::new(pingora::upstreams::peer::HttpPeer::new(
            format!("0.0.0.0:{port}"),
            false,
            String::new(),
        )),
        ..project.as_ref().clone()
    };

    {
        let mut projects = crate::common::PROJECTS
            .write()
            .map_err(|e| ServerFnError::new(anyhow!("{e:?}")))?;

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
    image: String,
    container_port: u16,
    host_port: u16,
) -> Result<(), ServerFnError> {
    user()?;

    let project = get_project_arc(id).await.map_err(ServerFnError::new)?;

    let new_project = Project {
        project_type: ProjectType::Container(Container {
            image,
            port_mapping: (container_port, host_port),
            status: None,
        }),
        peer: Box::new(pingora::upstreams::peer::HttpPeer::new(
            format!("0.0.0.0:{host_port}"),
            false,
            String::new(),
        )),
        ..project.as_ref().clone()
    };

    {
        let mut projects = crate::common::PROJECTS
            .write()
            .map_err(|e| ServerFnError::new(anyhow!("{e:?}")))?;

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
