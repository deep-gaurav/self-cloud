use axum::http::header;
use pingora::{
    proxy::{http_proxy_service_with_name, HttpProxy, ProxyHttp, Session},
    server::Server,
    services::listening::Service,
    upstreams::peer::HttpPeer,
    Error, Result,
};

use crate::PEERS;

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
