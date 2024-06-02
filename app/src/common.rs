use std::sync::{Arc, Weak};

use serde::{Deserialize, Deserializer, Serialize};
use tracing::{error, info};
use uuid::Uuid;

#[derive(Serialize, Clone)]
pub struct Project {
    pub id: Uuid,
    pub name: String,

    pub project_type: ProjectType,

    #[cfg(feature = "ssr")]
    #[serde(skip)]
    pub peer: Box<pingora::upstreams::peer::HttpPeer>,
}

#[derive(Serialize, Deserialize, Clone)]
pub enum ProjectType {
    PortForward(u32),
    Container(Container),
}

impl ProjectType {
    pub fn get_port(&self) -> u32 {
        match self {
            ProjectType::PortForward(port) => *port,
            ProjectType::Container(container) => container.port_mapping.1,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Container {
    image: String,
    port_mapping: (u32, u32),
}

#[cfg(feature = "ssr")]
impl Project {
    pub fn new_from_fields(fields: ProjectFields) -> Project {
        let peer = Box::new(pingora::upstreams::peer::HttpPeer::new(
            format!("127.0.0.1:{}", fields.project_type.get_port()),
            false,
            String::new(),
        ));
        Project {
            id: fields.id,
            project_type: fields.project_type,
            name: fields.name,
            peer,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct ProjectFields {
    pub id: Uuid,
    pub name: String,
    pub project_type: ProjectType,
}

impl From<Project> for ProjectFields {
    fn from(val: Project) -> Self {
        ProjectFields {
            id: val.id,
            project_type: val.project_type,
            name: val.name,
        }
    }
}

impl From<ProjectFields> for Project {
    fn from(value: ProjectFields) -> Self {
        Project {
            id: value.id,
            name: value.name,
            project_type: value.project_type,
            #[cfg(feature = "ssr")]
            peer: unimplemented!(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct DomainSerialize {
    pub domain: String,
    pub project_id: uuid::Uuid,
}

#[cfg(feature = "ssr")]
#[derive(Serialize, Deserialize)]
pub struct ProjectConfig {
    pub projects: Vec<ProjectFields>,
    pub domains: Vec<DomainSerialize>,
}

#[cfg(feature = "ssr")]
pub async fn load_projects_config() -> anyhow::Result<()> {
    let data = tokio::fs::read(get_home_path().join("projects.json")).await?;
    let project_config = serde_json::from_slice::<ProjectConfig>(&data)?;
    {
        let mut projects = PROJECTS
            .write()
            .map_err(|e| anyhow::anyhow!("Failed to aquire lock {e:?}"))?;
        for project in project_config.projects.into_iter() {
            let project = Arc::new(Project::new_from_fields(project));
            projects.insert(project.id, project.clone());
        }
    }

    for domain in project_config.domains.into_iter() {
        let project = {
            let projects = PROJECTS
                .read()
                .map_err(|e| anyhow::anyhow!("Failed to aquire lock {e:?}"))?;
            let project = projects.get(&domain.project_id).cloned();
            project
        };

        if let Some(project) = project {
            add_project_domain(project.clone(), domain.domain).await?;
        }
    }
    Ok(())
}

#[cfg(feature = "ssr")]
pub async fn save_project_config() -> anyhow::Result<()> {
    let projects = {
        let projects = PROJECTS
            .read()
            .map_err(|e| anyhow::anyhow!("Failed to aquire lock {e:?}"))?;
        let projects = projects
            .values()
            .map(|p| p.as_ref().clone().into())
            .collect::<Vec<_>>();
        projects
    };

    let domains = {
        let domains = DOMAIN_MAPPING
            .read()
            .map_err(|e| anyhow::anyhow!("Failed to aquire lock {e:?}"))?;
        domains
            .iter()
            .filter_map(|(domain, status)| {
                status.project.upgrade().map(|project| DomainSerialize {
                    domain: domain.to_string(),
                    project_id: project.id,
                })
            })
            .collect::<Vec<_>>()
    };

    let config = ProjectConfig { domains, projects };
    let data = serde_json::to_vec(&config)
        .map_err(|e| anyhow::anyhow!("Cannot serialize config {e:?}"))?;
    tokio::fs::write(get_home_path().join("projects.json"), data).await?;

    Ok(())
}

impl<'de> Deserialize<'de> for Project {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[cfg(not(feature = "ssr"))]
        {
            let fields = ProjectFields::deserialize(deserializer)?;
            Ok(fields.into())
        }

        #[cfg(feature = "ssr")]
        {
            Err(serde::de::Error::custom(
                "Deserialization is not supported with the 'ssr' feature enabled",
            ))
        }
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub enum SSLProvisioning {
    NotProvisioned,
    Provisioning,
    Provisioned(SSlData),
}

impl SSLProvisioning {
    /// Returns `true` if the sslprovisioning is [`NotProvisioned`].
    ///
    /// [`NotProvisioned`]: SSLProvisioning::NotProvisioned
    #[must_use]
    pub fn is_not_provisioned(&self) -> bool {
        matches!(self, Self::NotProvisioned)
    }

    /// Returns `true` if the sslprovisioning is [`Provisioned`].
    ///
    /// [`Provisioned`]: SSLProvisioning::Provisioned
    #[must_use]
    pub fn is_provisioned(&self) -> bool {
        matches!(self, Self::Provisioned(..))
    }

    /// Returns `true` if the sslprovisioning is [`Provisioning`].
    ///
    /// [`Provisioning`]: SSLProvisioning::Provisioning
    #[must_use]
    pub fn is_provisioning(&self) -> bool {
        matches!(self, Self::Provisioning)
    }
}

#[derive(Clone)]
#[cfg(feature = "ssr")]
pub struct DomainStatus {
    #[cfg(feature = "ssr")]
    pub project: Weak<Project>,
    pub ssl_provision: SSLProvisioning,
}

#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct DomainStatusFields {
    pub ssl_provision: SSLProvisioning,
}

#[cfg(feature = "ssr")]
impl From<DomainStatus> for DomainStatusFields {
    fn from(value: DomainStatus) -> Self {
        Self {
            ssl_provision: value.ssl_provision,
        }
    }
}

#[derive(Clone, Serialize)]
pub struct SSlData {
    #[cfg(feature = "ssr")]
    #[serde(skip)]
    pub cert: pingora::tls::x509::X509,

    #[serde(skip)]
    #[cfg(feature = "ssr")]
    pub key: pingora::tls::pkey::PKey<pingora::tls::pkey::Private>,

    pub is_active: bool,
}

impl PartialEq for SSlData {
    fn eq(&self, other: &Self) -> bool {
        #[cfg(not(feature = "ssr"))]
        {
            self.is_active == other.is_active
        }

        #[cfg(feature = "ssr")]
        {
            self.cert == other.cert && self.is_active == other.is_active
        }
    }
}

impl<'de> Deserialize<'de> for SSlData {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[cfg(not(feature = "ssr"))]
        {
            #[derive(Clone, Deserialize)]
            pub struct TmpSlData {
                pub is_active: bool,
            }

            let d = TmpSlData::deserialize(deserializer)?;

            Ok(SSlData {
                is_active: d.is_active,
            })
        }

        #[cfg(feature = "ssr")]
        {
            Err(serde::de::Error::custom(
                "Deserialization is not supported with the 'ssr' feature enabled",
            ))
        }
    }
}

#[cfg(feature = "ssr")]
pub static PROJECTS: once_cell::sync::Lazy<
    std::sync::RwLock<std::collections::HashMap<Uuid, std::sync::Arc<Project>>>,
> = once_cell::sync::Lazy::new(|| {
    tracing::debug!("Creating new projects list");
    let peers = std::collections::HashMap::new();
    std::sync::RwLock::new(peers)
});

#[cfg(feature = "ssr")]
pub static DOMAIN_MAPPING: once_cell::sync::Lazy<
    std::sync::RwLock<std::collections::HashMap<unicase::UniCase<String>, DomainStatus>>,
> = once_cell::sync::Lazy::new(|| {
    tracing::debug!("Creating new domain mapping");
    let peers = std::collections::HashMap::new();
    std::sync::RwLock::new(peers)
});

#[cfg(feature = "ssr")]
pub async fn add_port_forward_project(name: &str, port: u32) -> anyhow::Result<Arc<Project>> {
    let id = uuid::Uuid::new_v4();
    let http_peer = Box::new(pingora::upstreams::peer::HttpPeer::new(
        format!("127.0.0.1:{port}"),
        false,
        String::new(),
    ));
    let project = Arc::new(Project {
        id,
        name: name.to_string(),
        project_type: ProjectType::PortForward(port),
        peer: http_peer,
    });
    let mut projects = PROJECTS.write().map_err(|e| anyhow::anyhow!("{e:#?}"))?;
    projects.insert(id, project.clone());

    Ok(project)
}

#[cfg(feature = "ssr")]
pub async fn add_project_domain(project: Arc<Project>, domain: String) -> anyhow::Result<()> {
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
        let cert = pingora::tls::x509::X509::from_pem(&cert)?;
        let key = pingora::tls::pkey::PKey::private_key_from_pem(&key)?;
        let mut peers = DOMAIN_MAPPING
            .write()
            .map_err(|e| anyhow::anyhow!("{e:?}"))?;

        peers.insert(
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
        let mut peers = DOMAIN_MAPPING
            .write()
            .map_err(|e| anyhow::anyhow!("{e:?}"))?;

        peers.insert(
            UniCase::from(domain),
            DomainStatus {
                project: Arc::downgrade(&project),
                ssl_provision: SSLProvisioning::NotProvisioned,
            },
        );
    }

    Ok(())
}

#[cfg(feature = "ssr")]
pub fn get_home_path() -> std::path::PathBuf {
    use std::path::PathBuf;

    let home = std::env::var("SELF_CLOUD_HOME").expect("SELF_CLOUD_HOME var not set");
    PathBuf::from(home)
}
