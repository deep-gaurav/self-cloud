use app::App;
use axum::Router;
use leptos::get_configuration;
use leptos_axum::{generate_route_list, LeptosRoutes};
use pingora::{
    server::ShutdownWatch,
    services::{
        background::{background_service, BackgroundService, GenBackgroundService},
        listening::Service,
    },
};

use crate::{fileserv::file_and_error_handler, PEERS};

pub struct LeptosService {}

impl LeptosService {
    pub fn to_service() -> GenBackgroundService<Self> {
        background_service("leptos_service", Self {})
    }
}

#[async_trait::async_trait]
impl BackgroundService for LeptosService {
    async fn start(&self, mut shutdown: ShutdownWatch) {
        run_main().await
    }
}

async fn run_main() {
    // Setting get_configuration(None) means we'll be using cargo-leptos's env values
    // For deployment these variables are:
    // <https://github.com/leptos-rs/start-axum#executing-a-server-on-a-remote-machine-without-the-toolchain>
    // Alternately a file can be specified such as Some("Cargo.toml")
    // The file would need to be included with the executable when moved to deployment
    let conf = get_configuration(None).await.unwrap();
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;
    let routes = generate_route_list(App);

    // build our application with a route
    let app = Router::new()
        // .route("/.well-known/acme-challenge/<TOKEN>", method_router)
        .leptos_routes(&leptos_options, routes, App)
        .fallback(file_and_error_handler)
        .with_state(leptos_options);

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    tracing::info!("listening on http://{}", &addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}
