use std::{
    collections::HashMap,
    sync::{Arc, Weak},
};

use serde::{Deserialize, Deserializer, Serialize};
use unicase::UniCase;
use uuid::Uuid;

#[derive(Serialize, Clone, PartialEq, Debug)]
pub struct Project {
    pub id: Uuid,
    pub name: String,

    pub project_type: ProjectType,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum ProjectType {
    PortForward(PortForward),
    Container {
        primary_container: Container,
        support_containers: HashMap<String, SupportContainer>,

        tokens: HashMap<String, Token>,
        exposed_ports: smallvec::SmallVec<[ExposedPort; 4]>,
    },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct SupportContainer {
    pub image: String,
    pub container: Container,
}

#[derive(Serialize, Clone, Debug)]
pub struct PortForward {
    pub port: u16,

    #[cfg(feature = "ssr")]
    #[serde(skip)]
    pub peer: Arc<pingora::upstreams::peer::HttpPeer>,
}

#[cfg(feature = "ssr")]
impl PortForward {
    pub fn new(port: u16) -> Self {
        Self {
            port,
            peer: Arc::new({
                let mut peer = pingora::upstreams::peer::HttpPeer::new(
                    format!("127.0.0.1:{}", port),
                    false,
                    String::new(),
                );
                peer.options.alpn = pingora::protocols::ALPN::H2H1;
                peer
            }),
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
        matches!(self, Self::Container { .. })
    }

    pub fn try_get_primary(&self) -> Option<&Container> {
        match &self {
            ProjectType::PortForward(_) => None,
            ProjectType::Container {
                primary_container,
                support_containers: _,
                tokens: _,
                exposed_ports: _,
            } => Some(primary_container),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct EnvironmentVar {
    pub key: String,
    pub val: String,
}

#[derive(Serialize, Clone, Debug)]
pub struct Container {
    pub env_vars: smallvec::SmallVec<[EnvironmentVar; 4]>,
    #[cfg(feature = "ssr")]
    #[serde(skip)]
    pub status: ContainerStatus,
}

#[derive(Serialize, Clone, PartialEq, Deserialize, Debug)]
pub struct Token {
    pub token: String,
    pub expiry: Option<chrono::NaiveDate>,
    pub description: String,
}

#[derive(Clone, Debug)]
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

    pub fn as_running(&self) -> Option<&Arc<docker_api::api::Container>> {
        if let Self::Running(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// Returns `true` if the container status is [`Running`].
    ///
    /// [`Running`]: ContainerStatus::Running
    #[must_use]
    pub fn is_running(&self) -> bool {
        matches!(self, Self::Running(..))
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct ExposedPort {
    pub port: u16,
    pub host_port: Option<u16>,
    #[cfg(feature = "ssr")]
    #[serde(skip)]
    pub peer: Option<Arc<pingora::upstreams::peer::HttpPeer>>,
    pub domains: smallvec::SmallVec<[Domain; 2]>,
}

#[derive(Serialize, Clone, Debug, Deserialize)]
pub struct ExposedPortArg {
    pub port: String,
    pub domain: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
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
            pub host_port: Option<u16>,
            pub domains: smallvec::SmallVec<[Domain; 2]>,
        }

        let d = TmpExposedPort::deserialize(deserializer)?;

        #[cfg(not(feature = "ssr"))]
        {
            Ok(Self {
                port: d.port,
                host_port: d.host_port,
                domains: d.domains,
            })
        }

        #[cfg(feature = "ssr")]
        {
            Ok(Self {
                port: d.port,
                host_port: d.host_port,
                domains: d.domains,
                peer: None,
            })
        }
    }
}

impl PartialEq for Container {
    fn eq(&self, other: &Self) -> bool {
        self.env_vars == other.env_vars
    }
}

impl<'de> Deserialize<'de> for Container {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Clone, Deserialize)]
        pub struct TmpContainer {
            pub env_vars: smallvec::SmallVec<[EnvironmentVar; 4]>,
        }

        let d = TmpContainer::deserialize(deserializer)?;

        #[cfg(not(feature = "ssr"))]
        {
            Ok(Container {
                // exposed_ports: d.exposed_ports,
                // tokens: d.tokens,
                env_vars: d.env_vars,
            })
        }

        #[cfg(feature = "ssr")]
        {
            Ok(Container {
                // exposed_ports: d.exposed_ports,
                status: ContainerStatus::None,
                // tokens: d.tokens,
                env_vars: d.env_vars,
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

impl<'de> Deserialize<'de> for Project {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[cfg(not(feature = "ssr"))]
        {
            let fields = ProjectFields::deserialize(_deserializer)?;
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

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
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
#[cfg(feature = "ssr")]
impl DomainStatus {
    pub async fn get_project(&self) -> Option<Arc<Project>> {
        self.project.upgrade()
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
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

#[derive(Clone, Serialize, Debug)]
pub struct SSlData {
    #[cfg(feature = "ssr")]
    #[serde(skip)]
    pub cert: Vec<pingora::tls::x509::X509>,

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
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[cfg(not(feature = "ssr"))]
        {
            #[derive(Clone, Deserialize)]
            pub struct TmpSlData {
                pub is_active: bool,
            }

            let d = TmpSlData::deserialize(_deserializer)?;

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
pub async fn add_port_forward_project(
    name: &str,
    port: u16,
    context: &mut crate::context::ProjectContext,
) -> anyhow::Result<Arc<Project>> {
    let id = uuid::Uuid::new_v4();
    let project = Arc::new(Project {
        id,
        name: name.to_string(),
        project_type: ProjectType::PortForward(PortForward::new(port)),
    });
    context.update_project(id, project.clone()).await?;
    Ok(project)
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

#[derive(Serialize, Deserialize, Clone)]
pub enum TtyChunk {
    StdIn(Vec<u8>),
    StdOut(Vec<u8>),
    StdErr(Vec<u8>),
}

#[cfg(feature = "ssr")]
impl From<docker_api::conn::TtyChunk> for TtyChunk {
    fn from(value: docker_api::conn::TtyChunk) -> Self {
        match value {
            docker_api::conn::TtyChunk::StdIn(b) => Self::StdIn(b),
            docker_api::conn::TtyChunk::StdOut(b) => Self::StdOut(b),
            docker_api::conn::TtyChunk::StdErr(b) => Self::StdErr(b),
        }
    }
}

impl AsRef<[u8]> for TtyChunk {
    fn as_ref(&self) -> &[u8] {
        match &self {
            Self::StdIn(b) | Self::StdErr(b) | Self::StdOut(b) => b,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct AttachParams {
    pub command: String,
    pub size_width: u64,
    pub size_height: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UpdateStatus {
    pub current_git_hash: String,
    pub current_build_time: String,
    pub remote_git_hash: String,
    pub remote_build_time: String,
    pub update_available: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SystemStats {
    pub cpu_usage: f32,
    pub total_memory: u64,
    pub used_memory: u64,
    pub total_swap: u64,
    pub used_swap: u64,
    pub disks: Vec<DiskInfo>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DiskInfo {
    pub name: String,
    pub mount_point: String,
    pub total_space: u64,
    pub available_space: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cpu_usage: f32,
    pub memory: u64,
    pub user_id: Option<String>,
    pub status: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct FileInfo {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: Option<u64>,
}
