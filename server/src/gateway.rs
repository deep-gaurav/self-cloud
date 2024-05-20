use app::common::{SSLProvisioning, DOMAIN_MAPPING};
use axum::http::header;
use openssl::ssl::NameType;
use pingora::{
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
        info!(
            "IP: {:?}\nHanding request\n{}",
            session.client_addr(),
            session.request_summary()
        );
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

        // tracing::debug!("Read request");
        // match session.read_request().await {
        //     Ok(is_read) => {
        //         tracing::debug!("Request read {is_read}");
        //     }
        //     Err(err) => {
        //         tracing::error!("Cant read request {err:?}");
        //     }
        // };

        tracing::debug!("Get host");
        let host = get_host(session);
        tracing::debug!("Received host {host}");

        let peers = 'pe: {
            let peers = DOMAIN_MAPPING.read().unwrap();
            if let Some(peer) = peers.get(&host) {
                match peer.ssl_provision {
                    SSLProvisioning::NotProvisioned => {
                        return Err(pingora::Error::explain(
                            pingora::ErrorType::InternalError,
                            "TLS Provisioning not started",
                        ))
                    }
                    SSLProvisioning::Provisioning => break 'pe self.provisioning_gateway.clone(),
                    SSLProvisioning::Provisioned(_) => {
                        let project = peer.project.upgrade();
                        if let Some(project) = project {
                            break 'pe project.peer.clone();
                        }
                    }
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
