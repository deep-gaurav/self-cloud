use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use app::{
    common::{add_project, add_project_domain},
    App,
};
use axum::response::{IntoResponse, Response};
use axum::{body::Body as AxumBody, Router};
use axum::{
    extract::{FromRef, Request, State},
    routing::get,
};
use leptos::{get_configuration, LeptosOptions};
use leptos_axum::{generate_route_list, LeptosRoutes};
use leptos_router::RouteListing;

use pingora::{
    server::ShutdownWatch,
    services::{
        background::{background_service, BackgroundService, GenBackgroundService},
        listening::Service,
    },
};

use crate::{
    fileserv::file_and_error_handler,
    tls_gen::{acme_handler, tls_generator, TLSState},
};

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

#[derive(FromRef, Clone)]
pub struct AppState {
    leptos_options: LeptosOptions,
    routes: Vec<RouteListing>,
    pub tls_state: TLSState,
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

    let acme: TLSState = Arc::new(RwLock::new(HashMap::new()));

    let acme_c = acme.clone();
    tokio::spawn(async move {
        if let Err(err) = tls_generator(acme_c).await {
            tracing::error!("TLs Generator erroed {err:#?}");
        }
    });

    let addr = leptos_options.site_addr;
    let routes = generate_route_list(App);
    let app_state = AppState {
        routes: routes.clone(),
        leptos_options,
        tls_state: acme,
    };

    // build our application with a route
    let app = Router::new()
        .leptos_routes_with_handler(routes, get(leptos_routes_handler))
        .route("/.well-known/acme-challenge/:token", get(acme_handler))
        .fallback(file_and_error_handler)
        .with_state(app_state);

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    tracing::info!("listening on http://{}", &addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    let project = match add_project("cloud-panel", 3000).await {
        Ok(project) => project,
        Err(err) => {
            tracing::error!("Cant create cloud-panel project {err:#?}");
            return;
        }
    };
    if let Err(err) = add_project_domain(project, "cloud.deepwith.in".to_string()).await {
        tracing::error!("Cant add panel domain {err:#?}");
        return;
    }

    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

async fn leptos_routes_handler(
    State(app_state): State<AppState>,
    // auth_session: AuthSession,
    // path: Path<String>,
    // cookies: Cookies,
    request: Request<AxumBody>,
) -> Response {
    // info!("Handling request {:?}", request.uri());
    // let auth = if let Some(cookie) = cookies.get("sessionId") {
    //     if let Ok(user) = get_user_from_cookie(cookie) {
    //         AuthType::Authorized(user)
    //     } else {
    //         AuthType::UnAuthorized
    //     }
    // } else {
    //     AuthType::UnAuthorized
    // };

    let handler = leptos_axum::render_route_with_context(
        app_state.leptos_options.clone(),
        app_state.routes.clone(),
        move || {
            // provide_context(auth.clone());
            // provide_context(app_state.otp_map.clone());
        },
        App,
    );
    handler(request).await.into_response()
}
