use std::{collections::HashMap, sync::RwLock};

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
