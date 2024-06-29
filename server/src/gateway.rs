use std::sync::Arc;

use app::common::{
    ContainerStatus, DomainStatus, Project, SSLProvisioning, DOMAIN_MAPPING, PROJECTS,
};
use axum::{body::Bytes, http::header};
use openssl::ssl::NameType;
use pingora::{
    http::ResponseHeader,
    proxy::{http_proxy_service_with_name, HttpProxy, ProxyHttp, Session},
    server::Server,
    services::listening::Service,
    upstreams::peer::HttpPeer, Result,
};
use tracing::{info, warn};
use unicase::UniCase;

pub struct Gateway {
    provisioning_gateway: Box<HttpPeer>,
}

impl Gateway {
    pub fn to_service(server: &Server) -> Service<HttpProxy<Self>> {
        let http_port = 8080;
        let https_port = 4433;

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
    host: UniCase<String>,
}

fn get_session_domain(session: &mut Session) -> (UniCase<String>, Option<DomainStatus>) {
    fn get_host(session: &mut Session) -> String {
        if let Some(host) = session.get_header(header::HOST) {
            if let Ok(host_str) = host.to_str() {
                return host_str.to_lowercase();
            }
        }

        if let Some(host) = session.req_header().uri.host() {
            return host.to_lowercase();
        }

        "".to_string()
    }

    let host = UniCase::<String>::from(get_host(session));

    let peers = DOMAIN_MAPPING.read().unwrap();
    (host.clone(), peers.get(&host).cloned())
}

#[async_trait::async_trait]
impl ProxyHttp for Gateway {
    /// The per request object to share state across the different filters
    type CTX = GatewayContext;

    /// Define how the `ctx` should be created.
    fn new_ctx(&self) -> Self::CTX {
        GatewayContext {
            domain: None,
            host: UniCase::new(String::new()),
        }
    }

    async fn request_filter(&self, _session: &mut Session, _ctx: &mut Self::CTX) -> Result<bool>
    where
        Self::CTX: Send + Sync,
    {
        if _ctx.domain.is_none() {
            (_ctx.host, _ctx.domain) = get_session_domain(_session);
        }
        if let Some(domain) = &_ctx.domain {
            if domain.ssl_provision.is_provisioned() {
                let is_tls = _session
                    .digest()
                    .map(|d| d.ssl_digest.is_some())
                    .unwrap_or(false);
                if !is_tls {
                    let uri = _session.req_header().uri.clone();
                    let new_uri = http::uri::Builder::from(uri.clone())
                        .scheme("https")
                        .authority(_ctx.host.to_lowercase())
                        .build();
                    if let Ok(new_uri) = new_uri {
                        match ResponseHeader::build_no_case(
                            http::StatusCode::PERMANENT_REDIRECT,
                            None,
                        ) {
                            Ok(mut response) => {
                                if let Err(err) =
                                    response.append_header("Location", new_uri.to_string())
                                {
                                    warn!("Cant append header {err:?}")
                                }

                                if let Err(err) =
                                    response.append_header("Content-Length", 0.to_string())
                                {
                                    warn!("Cant append header {err:?}")
                                }

                                if let Err(err) =
                                    _session.write_response_header(Box::new(response)).await
                                {
                                    warn!("Cant write response header {err:?}")
                                }

                                if let Err(err) = _session.write_response_body(Bytes::new()).await {
                                    warn!("Cant write response body {err:?}")
                                }

                                if let Err(err) = _session.finish_body().await {
                                    warn!("Cant finish body {err:?}")
                                }

                                info!("Will redirect to TLS path \n{uri} -> {new_uri}");

                                return Ok(true);
                            }
                            Err(err) => warn!("Cant create response {err:?}"),
                        }
                    } else {
                        // info!("Old uri: {uri:?}\nUri not valid {:?}", new_uri);
                    }
                } else {
                    // info!("SSL exist {:?}", _session.digest());
                }
            } else {
                // info!("SSL Not provisioned");
            }
        } else {
            // info!("No domain status");
        }
        Ok(false)
    }

    async fn upstream_peer(
        &self,
        session: &mut Session,
        ctx: &mut Self::CTX,
    ) -> Result<Box<HttpPeer>> {
        if let Some(domain) = &mut ctx.domain {
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
                    fn get_peer(
                        project: Arc<Project>,
                        host: &UniCase<String>,
                    ) -> anyhow::Result<Box<HttpPeer>> {
                        match &project.project_type {
                            app::common::ProjectType::PortForward(port) => {
                                return Ok(port.peer.clone());
                            }
                            app::common::ProjectType::Container(container) => {
                                if let ContainerStatus::Running(pod) = &container.status {
                                    let port = container.exposed_ports.iter().find(|cont| {
                                        cont.domains.iter().any(|dom| &dom.name == host)
                                    });
                                    if let Some(port) = port {
                                        if let Some(peer) = &port.peer {
                                            return Ok(peer.clone());
                                        }
                                    }
                                }
                            }
                        }
                        Err(anyhow::anyhow!("No peer in project"))
                    }
                    if let Some(project) = project {
                        let peer = get_peer(project, &ctx.host);
                        if let Ok(peer) = peer {
                            return Ok(peer);
                        }
                    } else {
                        {
                            let projects = PROJECTS.read().await;
                            if let Some(proj) = projects.get(&domain.project_id) {
                                domain.project = Arc::downgrade(proj);
                            }
                        }
                        {
                            let mut domains = DOMAIN_MAPPING.write().unwrap();
                            domains.insert(ctx.host.clone(), domain.clone());
                        }
                        if let Some(project) = domain.project.upgrade() {
                            let peer = get_peer(project, &ctx.host);
                            if let Ok(peer) = peer {
                                return Ok(peer);
                            }
                        }
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
                let peer = peers.get(&UniCase::from(name));
                if let Some(peer) = peer {
                    if let SSLProvisioning::Provisioned(data) = &peer.ssl_provision {
                        break 'b Some((data.cert.clone(), data.key.clone()));
                    }
                }
                None
            };
            if let Some((cert, key)) = peer {
                let mut cert = cert.iter();
                ext::ssl_use_private_key(ssl, &key).unwrap();
                if let Some(cert) = cert.next() {
                    ext::ssl_use_certificate(ssl, &cert).unwrap();
                }
                while let Some(chain_cert) = cert.next() {
                    if let Err(err) = ext::ssl_add_chain_cert(ssl, &chain_cert) {
                        warn!("Failed loading cert chain {err:?}");
                    }
                }
            }
        }
    }
}
