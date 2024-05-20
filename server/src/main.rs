use std::{collections::HashMap, sync::RwLock};

use gateway::Gateway;
use leptos_service::LeptosService;
use pingora::{
    server::{configuration::Opt, Server},
    upstreams::peer::HttpPeer,
};
// use proxy::Gateway;
use once_cell::sync::Lazy;
use structopt::StructOpt;
// mod proxy;

mod fileserv;
mod gateway;
mod leptos_service;
mod tls_gen;

// main.rs
#[cfg(not(target_env = "msvc"))]
use tikv_jemallocator::Jemalloc;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

pub struct Peer {
    peer: Box<HttpPeer>,
    provisioning: SSLProvisioning,
}

pub enum SSLProvisioning {
    NotProvisioned,
    Provisioning,
    Provisioned(String, String),
}

impl SSLProvisioning {
    /// Returns `true` if the sslprovisioning is [`NotProvisioned`].
    ///
    /// [`NotProvisioned`]: SSLProvisioning::NotProvisioned
    #[must_use]
    pub fn is_not_provisioned(&self) -> bool {
        matches!(self, Self::NotProvisioned)
    }

    /// Returns `true` if the sslprovisioning is [`Provisioning`].
    ///
    /// [`Provisioning`]: SSLProvisioning::Provisioning
    #[must_use]
    pub fn is_provisioning(&self) -> bool {
        matches!(self, Self::Provisioning)
    }
}

pub static PEERS: Lazy<RwLock<HashMap<String, Peer>>> = Lazy::new(|| {
    let mut peers = HashMap::new();
    peers.insert(
        "cloud.deepwith.in".to_string(),
        Peer {
            peer: Box::new(HttpPeer::new("127.0.0.1:3000", false, String::new())),
            provisioning: SSLProvisioning::NotProvisioned,
        },
    );
    RwLock::new(peers)
});

fn main() {
    tracing_subscriber::fmt::init();

    let opt = Some(Opt::from_args());
    let mut my_server = Server::new(opt).unwrap();

    let leptos_service = LeptosService::to_service();

    let proxy_service = Gateway::to_service(&my_server);
    my_server.add_service(leptos_service);
    my_server.add_service(proxy_service);

    my_server.run_forever()
}
