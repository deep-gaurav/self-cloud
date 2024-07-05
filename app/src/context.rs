use std::{collections::HashMap, sync::Arc};

use unicase::UniCase;
use uuid::Uuid;

use crate::common::{
    get_home_path, DomainSerialize, DomainStatus, Project, ProjectConfig, SSLProvisioning, SSlData,
};

#[derive(Clone)]
pub struct ProjectContext {
    projects: Arc<tokio::sync::RwLock<std::collections::HashMap<Uuid, std::sync::Arc<Project>>>>,
    domains:
        Arc<tokio::sync::RwLock<std::collections::HashMap<unicase::UniCase<String>, DomainStatus>>>,
}

impl ProjectContext {
    pub fn new_empty() -> Self {
        Self {
            projects: Arc::new(tokio::sync::RwLock::const_new(HashMap::new())),
            domains: Arc::new(tokio::sync::RwLock::const_new(HashMap::new())),
        }
    }

    pub async fn load_from_config(&mut self) -> anyhow::Result<()> {
        let path = get_home_path().join("projects.json");
        tracing::info!("Loading path {path:?}");
        let data = tokio::fs::read(path).await?;
        tracing::debug!("Loaded data");
        let project_config = serde_json::from_slice::<ProjectConfig>(&data)?;
        tracing::debug!("Parsed config");

        tracing::debug!("Updating projects");
        {
            let mut projects = self.projects.write().await;
            for project in project_config.projects.into_iter() {
                let project = Arc::new(Project::new_from_fields(project));
                projects.insert(project.id, project.clone());
            }
        }

        tracing::debug!("Updating domains");
        let mut domains = self.domains.write().await;
        let projects = self.projects.read().await;
        for domain in project_config.domains.into_iter() {
            tracing::trace!("Get project for {}", domain.domain);
            let project = {
                let project = projects.get(&domain.project_id).cloned();
                project
            };
            tracing::trace!(
                "Add domain to {:?} ",
                project.as_ref().map(|p| p.name.clone())
            );
            if let Some(project) = project {
                Self::add_project_domain_self(project.clone(), domain.domain, &mut domains).await?;
            }
            tracing::trace!("Added domain");
        }

        tracing::debug!("Load success");
        Ok(())
    }

    pub async fn save_to_config(&self) -> anyhow::Result<()> {
        let projects = {
            let projects = self.projects.read().await;
            let projects = projects
                .values()
                .map(|p| p.as_ref().clone().into())
                .collect::<Vec<_>>();
            projects
        };

        let domains = {
            let domains = { self.domains.read().await };
            let mut dom = vec![];
            for (domain, status) in domains.iter() {
                if let Some(project) = status.get_project().await {
                    dom.push(DomainSerialize {
                        domain: domain.to_string(),
                        project_id: project.id,
                    })
                }
            }
            dom
        };

        let config = ProjectConfig { domains, projects };
        let data = serde_json::to_vec(&config)
            .map_err(|e| anyhow::anyhow!("Cannot serialize config {e:?}"))?;
        tokio::fs::write(get_home_path().join("projects.json"), data).await?;

        Ok(())
    }

    pub async fn update_project(
        &mut self,
        id: Uuid,
        new_project: Arc<Project>,
    ) -> anyhow::Result<()> {
        {
            let mut domains = self.domains.write().await;
            for domain in domains.iter_mut() {
                if let Some(project) = domain.1.project.upgrade() {
                    if project.id == id {
                        domain.1.project = Arc::downgrade(&new_project);
                    }
                }
            }
            let mut projects = self.projects.write().await;
            projects.insert(id, new_project);
        }
        self.save_to_config().await?;
        Ok(())
    }

    pub async fn remove_project(&mut self, id: Uuid) -> anyhow::Result<()> {
        {
            let mut projects = self.projects.write().await;
            projects.remove(&id);
        }
        self.save_to_config().await?;
        Ok(())
    }

    pub async fn add_project_domain(
        &mut self,
        project: Arc<Project>,
        domain: String,
    ) -> anyhow::Result<()> {
        {
            let mut domains = self.domains.write().await;
            Self::add_project_domain_self(project, domain, &mut domains).await?;
        }
        self.save_to_config().await?;
        Ok(())
    }

    async fn add_project_domain_self<'a>(
        project: Arc<Project>,
        domain: String,
        domains: &mut tokio::sync::RwLockWriteGuard<'a, HashMap<UniCase<String>, DomainStatus>>,
    ) -> anyhow::Result<()> {
        use unicase::UniCase;

        let domain = domain.to_ascii_lowercase();
        if let (Ok(cert), Ok(key)) = (
            tokio::fs::read(
                get_home_path()
                    .join("certificates")
                    .join(&domain)
                    .join("cert.pem"),
            )
            .await,
            tokio::fs::read(
                get_home_path()
                    .join("certificates")
                    .join(&domain)
                    .join("key.pem"),
            )
            .await,
        ) {
            let cert = pingora::tls::x509::X509::stack_from_pem(&cert)?;
            if cert.len() < 1 {
                anyhow::bail!("Should have atleast 1 certificate")
            }
            let key = pingora::tls::pkey::PKey::private_key_from_pem(&key)?;

            domains.insert(
                UniCase::from(domain),
                DomainStatus {
                    project: Arc::downgrade(&project),
                    ssl_provision: SSLProvisioning::Provisioned(SSlData {
                        cert,
                        key,
                        is_active: true,
                    }),
                },
            );
        } else {
            domains.insert(
                UniCase::from(domain),
                DomainStatus {
                    project: Arc::downgrade(&project),
                    ssl_provision: SSLProvisioning::NotProvisioned,
                },
            );
        }

        Ok(())
    }

    pub async fn get_projects(&self) -> Vec<Arc<Project>> {
        let projects = self.projects.read().await;
        let projects = projects.iter().map(|p| p.1.clone()).collect();
        projects
    }

    pub async fn get_project(&self, id: Uuid) -> Option<Arc<Project>> {
        let projects = self.projects.read().await;
        projects.get(&id).cloned()
    }

    pub async fn get_project_domains(&self, id: Uuid) -> HashMap<UniCase<String>, DomainStatus> {
        let domains = self.domains.read().await;
        let domains = domains.iter().filter_map(|(domain, status)| {
            if let Some(proj) = status.project.upgrade() {
                if proj.id == id {
                    Some((domain.clone(), status.clone()))
                } else {
                    None
                }
            } else {
                None
            }
        });
        domains.collect()
    }

    pub async fn get_domain(&self, domain: &UniCase<String>) -> Option<DomainStatus> {
        let domains = self.domains.read().await;
        domains.get(domain).cloned()
    }

    pub async fn get_all_domains(&self) -> HashMap<UniCase<String>, DomainStatus> {
        let domains = self.domains.read().await;
        let domains = domains.clone();
        domains
    }

    pub async fn update_domain(&mut self, domain: UniCase<String>, status: DomainStatus) {
        let mut domains = self.domains.write().await;
        domains.insert(domain, status);
    }
}
