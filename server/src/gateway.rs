use axum::http::header;
use openssl::ssl::NameType;
use pingora::{
    proxy::{http_proxy_service_with_name, HttpProxy, ProxyHttp, Session},
    server::Server,
    services::listening::Service,
    upstreams::peer::HttpPeer,
    Error, Result,
};

use crate::{SSLProvisioning, PEERS};

pub struct Gateway {
    provisioning_gateway: Box<HttpPeer>,
}

impl Gateway {
    pub fn to_service(server: &Server) -> Service<HttpProxy<Self>> {
        let provisioning_gateway = Box::new(HttpPeer::new("127.0.0.1:3000", false, String::new()));
        let service = Self {
            provisioning_gateway,
        };
        let mut service =
            http_proxy_service_with_name(&server.configuration, service, "gateway_proxy");

        service.add_tcp("0.0.0.0:80");

        let mut tls_settings =
            pingora::listeners::TlsSettings::with_callbacks(Box::new(CertSolver {})).unwrap();
        // by default intermediate supports both TLS 1.2 and 1.3. We force to tls 1.2 just for the demo
        tls_settings
            .set_max_proto_version(Some(pingora::tls::ssl::SslVersion::TLS1_2))
            .unwrap();
        tls_settings.enable_h2();

        service.add_tls_with_settings("0.0.0.0:443", None, tls_settings);

        service
    }
}

#[async_trait::async_trait]
impl ProxyHttp for Gateway {
    /// The per request object to share state across the different filters
    type CTX = ();

    /// Define how the `ctx` should be created.
    fn new_ctx(&self) -> Self::CTX {
        ()
    }

    async fn upstream_peer(
        &self,
        session: &mut Session,
        ctx: &mut Self::CTX,
    ) -> Result<Box<HttpPeer>> {
        let Some(host) = session.get_header(header::HOST) else {
            return Err(Error::new(pingora::ErrorType::Custom(
                "host header missing",
            )));
        };
        let host = host.to_str().map_err(|e| {
            pingora::Error::because(pingora::ErrorType::InternalError, "host not str", e)
        })?;
        let peers = 'pe: {
            let peers = PEERS.read().unwrap();
            if let Some(peer) = peers.get(host) {
                match peer.provisioning {
                    crate::SSLProvisioning::NotProvisioned => {
                        return Err(pingora::Error::explain(
                            pingora::ErrorType::InternalError,
                            "TLS Provisioning not started",
                        ))
                    }
                    crate::SSLProvisioning::Provisioning => {
                        break 'pe self.provisioning_gateway.clone()
                    }
                    crate::SSLProvisioning::Provisioned(_, _) => break 'pe peer.peer.clone(),
                }
            }
            return Err(pingora::Error::explain(
                pingora::ErrorType::InternalError,
                "no peer for given host",
            ));
        };
        Ok(peers)
    }
}

pub async fn add_peer(domain: String, port: u32) -> anyhow::Result<()> {
    if let (Ok(cert), Ok(key)) = (
        tokio::fs::read(format!("certificates/{domain}/cert.pem")).await,
        tokio::fs::read(format!("certificates/{domain}/key.pem")).await,
    ) {
        let cert = pingora::tls::x509::X509::from_pem(&cert)?;
        let key = pingora::tls::pkey::PKey::private_key_from_pem(&key)?;
        let mut peers = PEERS.write().unwrap();

        peers.insert(
            domain.clone(),
            crate::Peer {
                peer: Box::new(HttpPeer::new(
                    format!("127.0.0.1:{port}"),
                    false,
                    String::new(),
                )),
                provisioning: crate::SSLProvisioning::Provisioned(cert, key),
            },
        );
    } else {
        let mut peers = PEERS.write().unwrap();

        peers.insert(
            domain,
            crate::Peer {
                peer: Box::new(HttpPeer::new(
                    format!("127.0.0.1:{port}"),
                    false,
                    String::new(),
                )),
                provisioning: crate::SSLProvisioning::NotProvisioned,
            },
        );
    }

    Ok(())
}

struct CertSolver {}

#[async_trait::async_trait]
impl pingora::listeners::TlsAccept for CertSolver {
    async fn certificate_callback(&self, ssl: &mut pingora::tls::ssl::SslRef) {
        use pingora::tls::ext;
        let name = ssl.servername(NameType::HOST_NAME);
        if let Some(name) = name {
            let peer = 'b: {
                let peers = PEERS.read().unwrap();
                let peer = peers.get(name);
                if let Some(peer) = peer {
                    if let SSLProvisioning::Provisioned(cert, key) = &peer.provisioning {
                        break 'b Some((cert.clone(), key.clone()));
                    }
                }
                None
            };
            if let Some((cert, key)) = peer {
                ext::ssl_use_certificate(ssl, &cert).unwrap();
                ext::ssl_use_private_key(ssl, &key).unwrap();
            }
        }
    }
}
