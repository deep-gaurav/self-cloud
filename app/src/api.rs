use std::borrow::Borrow;

use anyhow::anyhow;
use leptos::{server, ServerFnError};

use crate::common::Project;

#[server(GetProjects)]
pub async fn get_projects() -> Result<Vec<Project>, ServerFnError> {
    use crate::common::PROJECTS;

    let projects = PROJECTS
        .read()
        .map_err(|e| ServerFnError::new(anyhow!("{e:?}")))?;

    let projects = projects
        .iter()
        .map(|e| e.1.as_ref().clone())
        .collect::<Vec<_>>();
    Ok(projects)
}
