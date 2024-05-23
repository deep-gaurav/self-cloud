use app::common::{DomainStatus, SSLProvisioning, DOMAIN_MAPPING};
use axum::{body::Bytes, http::header};
use openssl::ssl::NameType;
use pingora::{
    http::ResponseHeader,
    protocols::ssl::digest,
    proxy::{http_proxy_service_with_name, HttpProxy, ProxyHttp, Session},
    server::Server,
    services::listening::Service,
    upstreams::peer::HttpPeer,
    Error, Result,
};
use tracing::{info, warn};

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
    host: String,
}

fn get_session_domain(session: &mut Session) -> (String, Option<DomainStatus>) {
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
            host: String::new(),
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
                        .authority(_ctx.host.clone())
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
