use std::{collections::HashMap, sync::RwLock};

use app::context::ProjectContext;
use container_manager::ContainerManager;
use gateway::Gateway;
use leptos_service::LeptosService;
use pingora::server::{configuration::Opt, Server};
// use proxy::Gateway;
use clap::Parser;
// mod proxy;

mod auth;
mod container_events;
mod container_manager;
mod fileserv;
mod gateway;
mod image_uploader;
mod leptos_service;
mod system_monitor;
mod terminal;
mod tls_gen;

// main.rs
#[cfg(not(target_env = "msvc"))]
use tikv_jemallocator::Jemalloc;
use tls_gen::{TLSGenService, TLSState};
use tracing::{level_filters::LevelFilter, warn};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

fn main() {
    dotenv::dotenv().ok();

    let subscriber = tracing_subscriber::registry().with(fmt::layer()).with(
        EnvFilter::builder()
            .with_default_directive(LevelFilter::INFO.into())
            .from_env_lossy(),
    );

    if cfg!(target_os = "linux") {
        match tracing_journald::layer() {
            Ok(journald_layer) => {
                if let Err(err) = subscriber.with(journald_layer).try_init() {
                    warn!("Cannot initialize tracing {err:#?}")
                }
            }

            Err(err) => {
                warn!("Cant get journald_layer {err:#?}");
                if let Err(err) = subscriber.try_init() {
                    warn!("Cannot initialize tracing {err:#?}")
                }
            }
        }
    } else {
        subscriber.init();
    }
    let opt = Some(Opt::parse());
    let mut my_server = Server::new(opt).unwrap();

    let context = ProjectContext::new_empty();

    let tls_state = TLSState::new(RwLock::new(HashMap::new()));

    let leptos_service = LeptosService::to_service(tls_state.clone(), context.clone());
    let tls_gen_service = TLSGenService::to_service(tls_state, context.clone());
    let proxy_service = Gateway::to_service(&my_server, context.clone());
    let container_service = ContainerManager::to_service(context);

    my_server.add_service(leptos_service);
    my_server.add_service(proxy_service);
    my_server.add_service(tls_gen_service);
    my_server.add_service(container_service);

    my_server.bootstrap();
    my_server.run_forever()
}
