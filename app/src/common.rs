use std::sync::{Arc, Weak};

use leptos::server;
use serde::{Deserialize, Deserializer, Serialize};
use uuid::Uuid;

#[derive(Serialize, Clone)]
pub struct Project {
    pub id: Uuid,
    pub port: u32,
    pub name: String,

    #[cfg(feature = "ssr")]
    #[serde(skip)]
    pub peer: Box<pingora::upstreams::peer::HttpPeer>,
}

#[cfg(not(feature = "ssr"))]
#[derive(Deserialize)]
struct ProjectFields {
    pub id: Uuid,
    pub port: u32,
    pub name: String,
}

impl<'de> Deserialize<'de> for Project {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[cfg(not(feature = "ssr"))]
        {
            let fields = ProjectFields::deserialize(deserializer)?;
            Ok(Project {
                id: fields.id,
                port: fields.port,
                name: fields.name,
                #[cfg(feature = "ssr")]
                peer: unimplemented!(),
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
}

#[cfg(feature = "ssr")]
pub struct DomainStatus {
    pub project: Weak<Project>,
    pub ssl_provision: SSLProvisioning,
}

pub struct SSlData {
    #[cfg(feature = "ssr")]
    pub cert: pingora::tls::x509::X509,

    #[cfg(feature = "ssr")]
    pub key: pingora::tls::pkey::PKey<pingora::tls::pkey::Private>,
}

#[cfg(feature = "ssr")]
pub static PROJECTS: once_cell::sync::Lazy<
    std::sync::RwLock<std::collections::HashMap<Uuid, std::sync::Arc<Project>>>,
> = once_cell::sync::Lazy::new(|| {
    let mut peers = std::collections::HashMap::new();
    std::sync::RwLock::new(peers)
});

#[cfg(feature = "ssr")]
pub static DOMAIN_MAPPING: once_cell::sync::Lazy<
    std::sync::RwLock<std::collections::HashMap<String, DomainStatus>>,
> = once_cell::sync::Lazy::new(|| {
    let mut peers = std::collections::HashMap::new();
    std::sync::RwLock::new(peers)
});

#[cfg(feature = "ssr")]
pub async fn add_project(name: &str, port: u32) -> anyhow::Result<Arc<Project>> {
    let id = uuid::Uuid::new_v4();
    let http_peer = Box::new(pingora::upstreams::peer::HttpPeer::new(
        format!("127.0.0.1:{port}"),
        false,
        String::new(),
    ));
    let project = Arc::new(Project {
        id,
        name: name.to_string(),
        port,
        peer: http_peer,
    });
    let mut projects = PROJECTS.write().map_err(|e| anyhow::anyhow!("{e:#?}"))?;
    projects.insert(id, project.clone());

    Ok(project)
}

#[cfg(feature = "ssr")]
pub async fn add_project_domain(project: Arc<Project>, domain: String) -> anyhow::Result<()> {
    if let (Ok(cert), Ok(key)) = (
        tokio::fs::read(format!("certificates/{domain}/cert.pem")).await,
        tokio::fs::read(format!("certificates/{domain}/key.pem")).await,
    ) {
        let cert = pingora::tls::x509::X509::from_pem(&cert)?;
        let key = pingora::tls::pkey::PKey::private_key_from_pem(&key)?;
        let mut peers = DOMAIN_MAPPING
            .write()
            .map_err(|e| anyhow::anyhow!("{e:?}"))?;

        peers.insert(
            domain.clone(),
            DomainStatus {
                project: Arc::downgrade(&project),
                ssl_provision: SSLProvisioning::Provisioned(SSlData {
                    cert: cert,
                    key: key,
                }),
            },
        );
    } else {
        let mut peers = DOMAIN_MAPPING
            .write()
            .map_err(|e| anyhow::anyhow!("{e:?}"))?;

        peers.insert(
            domain,
            DomainStatus {
                project: Arc::downgrade(&project),
                ssl_provision: SSLProvisioning::NotProvisioned,
            },
        );
    }

    Ok(())
}
