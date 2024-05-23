use app::common::{DomainStatus, SSLProvisioning, DOMAIN_MAPPING};
use axum::http::header;
use openssl::ssl::NameType;
use pingora::{
    protocols::ssl::digest,
    proxy::{http_proxy_service_with_name, HttpProxy, ProxyHttp, Session},
    server::Server,
    services::listening::Service,
    upstreams::peer::HttpPeer,
    Error, Result,
};
use tracing::info;

pub struct Gateway {
    provisioning_gateway: Box<HttpPeer>,
}

impl Gateway {
    pub fn to_service(server: &Server) -> Service<HttpProxy<Self>> {
        let http_port;
        let https_port;
        if cfg!(debug_assertions) {
            http_port = 8080;
            https_port = 4433;
        } else {
            https_port = 443;
            http_port = 80;
        }
        let provisioning_gateway = Box::new(HttpPeer::new("127.0.0.1:3000", false, String::new()));
        let service = Self {
            provisioning_gateway,
        };
        let mut service =
            http_proxy_service_with_name(&server.configuration, service, "gateway_proxy");

        service.add_tcp(&format!("0.0.0.0:{http_port}"));

        let mut tls_settings =
            pingora::listeners::TlsSettings::with_callbacks(Box::new(CertSolver {})).unwrap();
        // by default intermediate supports both TLS 1.2 and 1.3. We force to tls 1.2 just for the demo
        tls_settings
            .set_max_proto_version(Some(pingora::tls::ssl::SslVersion::TLS1_2))
            .unwrap();
        tls_settings.enable_h2();

        service.add_tls_with_settings(&format!("0.0.0.0:{https_port}"), None, tls_settings);

        service
    }
}

pub struct GatewayContext {
    domain: Option<DomainStatus>,
}

fn get_session_domain(session: &mut Session) -> Option<DomainStatus> {
    fn get_host(session: &mut Session) -> String {
        if let Some(host) = session.get_header(header::HOST) {
            if let Ok(host_str) = host.to_str() {
                return host_str.to_string();
            }
        }

        if let Some(host) = session.req_header().uri.host() {
            return host.to_string();
        }

        "".to_string()
    }

    let host = get_host(session);

    let peers = DOMAIN_MAPPING.read().unwrap();
    peers.get(&host).cloned()
}

#[async_trait::async_trait]
impl ProxyHttp for Gateway {
    /// The per request object to share state across the different filters
    type CTX = GatewayContext;

    /// Define how the `ctx` should be created.
    fn new_ctx(&self) -> Self::CTX {
        GatewayContext { domain: None }
    }

    async fn request_filter(&self, _session: &mut Session, _ctx: &mut Self::CTX) -> Result<bool>
    where
        Self::CTX: Send + Sync,
    {
        if _ctx.domain.is_none() {
            _ctx.domain = get_session_domain(_session);
        }
        let digest = _session.digest();
        if let Some(digest) = digest {}
        Ok(false)
    }

    async fn upstream_peer(
        &self,
        session: &mut Session,
        ctx: &mut Self::CTX,
    ) -> Result<Box<HttpPeer>> {
        if let Some(domain) = &ctx.domain {
            match domain.ssl_provision {
                SSLProvisioning::NotProvisioned => {
                    return Err(pingora::Error::explain(
                        pingora::ErrorType::InternalError,
                        "TLS Provisioning not started",
                    ))
                }
                SSLProvisioning::Provisioning => return Ok(self.provisioning_gateway.clone()),
                SSLProvisioning::Provisioned(_) => {
                    let project = domain.project.upgrade();
                    if let Some(project) = project {
                        return Ok(project.peer.clone());
                    }
                }
            }
        }
        tracing::error!("No peer for host, from ip: {:?} ", session.client_addr());
        return Err(pingora::Error::explain(
            pingora::ErrorType::InternalError,
            "no peer for given host",
        ));
    }
}

struct CertSolver {}

#[async_trait::async_trait]
impl pingora::listeners::TlsAccept for CertSolver {
    async fn certificate_callback(&self, ssl: &mut pingora::tls::ssl::SslRef) {
        use pingora::tls::ext;
        let name = ssl.servername(NameType::HOST_NAME);
        if let Some(name) = name {
            let peer = 'b: {
                let peers = DOMAIN_MAPPING.read().unwrap();
                let peer = peers.get(name);
                if let Some(peer) = peer {
                    if let SSLProvisioning::Provisioned(data) = &peer.ssl_provision {
                        break 'b Some((data.cert.clone(), data.key.clone()));
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
