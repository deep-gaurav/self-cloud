use std::{collections::HashMap, sync::RwLock};

use container_manager::ContainerManager;
use gateway::Gateway;
use leptos_service::LeptosService;
use openssl::pkey::Private;
use pingora::{
    server::{configuration::Opt, Server},
    upstreams::peer::HttpPeer,
};
// use proxy::Gateway;
use once_cell::sync::Lazy;
use structopt::StructOpt;
// mod proxy;

mod auth;
mod container_manager;
mod fileserv;
mod gateway;
mod image_uploader;
mod leptos_service;
mod tls_gen;

// main.rs
#[cfg(not(target_env = "msvc"))]
use tikv_jemallocator::Jemalloc;
use tls_gen::{TLSGenService, TLSState};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

fn main() {
    let subscriber = tracing_subscriber::registry().with(fmt::layer()).with(
        EnvFilter::builder()
            .with_default_directive(LevelFilter::INFO.into())
            .from_env_lossy(),
    );

    if cfg!(target_os = "linux") {
        subscriber
            .with(tracing_journald::layer().expect("Cannot initialize journald"))
            .init();
    } else {
        subscriber.init();
    }
    let opt = Some(Opt::from_args());
    let mut my_server = Server::new(opt).unwrap();

    let tls_state = TLSState::new(RwLock::new(HashMap::new()));

    let leptos_service = LeptosService::to_service(tls_state.clone());
    let tls_gen_service = TLSGenService::to_service(tls_state);
    let proxy_service = Gateway::to_service(&my_server);
    let container_service = ContainerManager::to_service();
    my_server.add_service(leptos_service);
    my_server.add_service(proxy_service);
    my_server.add_service(tls_gen_service);
    my_server.add_service(container_service);

    my_server.bootstrap();
    my_server.run_forever()
}
