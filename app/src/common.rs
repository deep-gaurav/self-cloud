use std::{
    str::FromStr,
    sync::{Arc, Weak},
};

use serde::{Deserialize, Deserializer, Serialize};
use unicase::UniCase;
use uuid::Uuid;

#[cfg(feature = "ssr")]
pub static PROJECTS: once_cell::sync::Lazy<
    tokio::sync::RwLock<std::collections::HashMap<Uuid, std::sync::Arc<Project>>>,
> = once_cell::sync::Lazy::new(|| {
    tracing::debug!("Creating new projects list");
    let peers = std::collections::HashMap::new();
    tokio::sync::RwLock::new(peers)
});

#[cfg(feature = "ssr")]
pub static DOMAIN_MAPPING: once_cell::sync::Lazy<
    std::sync::RwLock<std::collections::HashMap<unicase::UniCase<String>, DomainStatus>>,
> = once_cell::sync::Lazy::new(|| {
    tracing::debug!("Creating new domain mapping");
    let peers = std::collections::HashMap::new();
    std::sync::RwLock::new(peers)
});

#[derive(Serialize, Clone, PartialEq)]
pub struct Project {
    pub id: Uuid,
    pub name: String,

    pub project_type: ProjectType,
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub enum ProjectType {
    PortForward(PortForward),
    Container(Container),
}

#[derive(Serialize, Clone)]
pub struct PortForward {
    pub port: u16,

    #[cfg(feature = "ssr")]
    #[serde(skip)]
    pub peer: Box<pingora::upstreams::peer::HttpPeer>,
}

#[cfg(feature = "ssr")]
impl PortForward {
    pub fn new(port: u16) -> Self {
        Self {
            port,
            peer: Box::new(pingora::upstreams::peer::HttpPeer::new(
                format!("0.0.0.0:{}", port),
                false,
                String::new(),
            )),
        }
    }
}

impl PartialEq for PortForward {
    fn eq(&self, other: &Self) -> bool {
        self.port == other.port
    }
}

impl<'de> Deserialize<'de> for PortForward {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Clone, Deserialize)]
        pub struct TmpPortForward {
            port: u16,
        }
        let d = TmpPortForward::deserialize(deserializer)?;

        #[cfg(not(feature = "ssr"))]
        {
            Ok(Self { port: d.port })
        }

        #[cfg(feature = "ssr")]
        {
            Ok(Self::new(d.port))
        }
    }
}

impl ProjectType {
    /// Returns `true` if the project type is [`PortForward`].
    ///
    /// [`PortForward`]: ProjectType::PortForward
    #[must_use]
    pub fn is_port_forward(&self) -> bool {
        matches!(self, Self::PortForward(..))
    }

    /// Returns `true` if the project type is [`Container`].
    ///
    /// [`Container`]: ProjectType::Container
    #[must_use]
    pub fn is_container(&self) -> bool {
        matches!(self, Self::Container(..))
    }
}

#[derive(Serialize, Clone)]
pub struct Container {
    pub exposed_ports: Vec<ExposedPort>,
    #[cfg(feature = "ssr")]
    #[serde(skip)]
    pub status: ContainerStatus,
}

#[derive(Clone)]
#[cfg(feature = "ssr")]
pub enum ContainerStatus {
    None,
    Creating,
    Failed,
    Running(Arc<docker_api::api::Container>),
}

#[cfg(feature = "ssr")]
impl ContainerStatus {
    /// Returns `true` if the container status is [`None`].
    ///
    /// [`None`]: ContainerStatus::None
    #[must_use]
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }
}

#[derive(Serialize, Clone)]
pub struct ExposedPort {
    pub port: u16,
    #[cfg(feature = "ssr")]
    #[serde(skip)]
    pub peer: Option<Box<pingora::upstreams::peer::HttpPeer>>,
    pub domains: Vec<Domain>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct Domain {
    #[serde(with = "unicase_serde::unicase")]
    pub name: UniCase<String>,
}

impl PartialEq for ExposedPort {
    fn eq(&self, other: &Self) -> bool {
        self.port == other.port && self.domains == other.domains
    }
}

impl<'de> Deserialize<'de> for ExposedPort {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Clone, Deserialize)]
        pub struct TmpExposedPort {
            pub port: u16,
            pub domains: Vec<Domain>,
        }

        let d = TmpExposedPort::deserialize(deserializer)?;

        #[cfg(not(feature = "ssr"))]
        {
            Ok(Self {
                port: d.port,
                domains: d.domains,
            })
        }

        #[cfg(feature = "ssr")]
        {
            Ok(Self {
                port: d.port,
                domains: d.domains,
                peer: None,
            })
        }
    }
}

impl PartialEq for Container {
    fn eq(&self, other: &Self) -> bool {
        self.exposed_ports == other.exposed_ports
    }
}

impl<'de> Deserialize<'de> for Container {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Clone, Deserialize)]
        pub struct TmpContainer {
            pub exposed_ports: Vec<ExposedPort>,
        }

        let d = TmpContainer::deserialize(deserializer)?;

        #[cfg(not(feature = "ssr"))]
        {
            Ok(Container {
                exposed_ports: d.exposed_ports,
            })
        }

        #[cfg(feature = "ssr")]
        {
            Ok(Container {
                exposed_ports: d.exposed_ports,
                status: ContainerStatus::None,
            })
        }
    }
}

#[cfg(feature = "ssr")]
impl Project {
    pub fn new_from_fields(fields: ProjectFields) -> Project {
        Project {
            id: fields.id,
            project_type: fields.project_type,
            name: fields.name,
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
        let mut projects = PROJECTS.write().await;
        for project in project_config.projects.into_iter() {
            let project = Arc::new(Project::new_from_fields(project));
            projects.insert(project.id, project.clone());
        }
    }

    for domain in project_config.domains.into_iter() {
        let project = {
            let projects = PROJECTS.read().await;
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
        let projects = PROJECTS.read().await;
        let projects = projects
            .values()
            .map(|p| p.as_ref().clone().into())
            .collect::<Vec<_>>();
        projects
    };

    let domains = {
        let domains = {
            DOMAIN_MAPPING
                .read()
                .map_err(|e| anyhow::anyhow!("Failed to aquire lock {e:?}"))?
                .clone()
        };
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
    pub project_id: Uuid,
    pub ssl_provision: SSLProvisioning,
}
#[cfg(feature = "ssr")]
impl DomainStatus {
    pub async fn get_project(&self) -> Option<Arc<Project>> {
        if let Some(project) = self.project.upgrade() {
            return Some(project);
        } else {
            {
                let projects = PROJECTS.read().await;
                if let Some(proj) = projects.get(&self.project_id) {
                    return Some(proj.clone());
                }
            }
        }
        None
    }

    pub async fn get_project_and_update(&mut self) -> Option<Arc<Project>> {
        if let Some(project) = self.project.upgrade() {
            return Some(project);
        } else {
            {
                let projects = PROJECTS.read().await;
                if let Some(proj) = projects.get(&self.project_id) {
                    self.project = Arc::downgrade(proj);
                    return Some(proj.clone());
                }
            }
        }
        None
    }
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
pub async fn add_port_forward_project(name: &str, port: u16) -> anyhow::Result<Arc<Project>> {
    let id = uuid::Uuid::new_v4();
    let project = Arc::new(Project {
        id,
        name: name.to_string(),
        project_type: ProjectType::PortForward(PortForward::new(port)),
    });
    let mut projects = PROJECTS.write().await;
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
                project_id: project.id,
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
                project_id: project.id,
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

#[cfg(feature = "ssr")]
pub fn get_docker() -> docker_api::Docker {
    let sock = std::env::var("DOCKER_SOCK").expect("DOCKER_SOCK var not set");

    docker_api::Docker::unix(sock)
}
