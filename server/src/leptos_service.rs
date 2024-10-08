use app::{
    auth::{server::get_user_from_cookie, AuthType, AuthorizedUsers},
    common::add_port_forward_project,
    context::ProjectContext,
    App,
};
use axum::{body::Body as AxumBody, extract::DefaultBodyLimit, Router};
use axum::{
    extract::{FromRef, Request, State},
    routing::get,
};
use axum::{
    response::{IntoResponse, Response},
    routing::post,
};
use leptos::{get_configuration, provide_context, LeptosOptions};
use leptos_axum::{generate_route_list, handle_server_fns_with_context, LeptosRoutes};
use leptos_router::RouteListing;

use pingora::{
    server::ShutdownWatch,
    services::background::{background_service, BackgroundService, GenBackgroundService},
};
use tower_cookies::{CookieManagerLayer, Cookies};
use tracing::info;

use crate::{
    auth::get_authorized_users,
    container_events::{
        attach::container_attach_ws, logs::container_logs_ws, stats::container_stats_ws,
    },
    fileserv::file_and_error_handler,
    image_uploader,
    tls_gen::{acme_handler, TLSState},
};

pub struct LeptosService {
    tls_state: TLSState,
    context: ProjectContext,
}

impl LeptosService {
    pub fn to_service(tls_state: TLSState, context: ProjectContext) -> GenBackgroundService<Self> {
        background_service("leptos_service", Self { tls_state, context })
    }
}

#[async_trait::async_trait]
impl BackgroundService for LeptosService {
    async fn start(&self, mut shutdown: ShutdownWatch) {
        tokio::select! {
            _ = shutdown.changed() => {
                info!("Shutdown received");
            }
            _ = run_main(self.tls_state.clone(), self.context.clone()) => {
                info!("Leptos ended");
            }
        };
    }
}

#[derive(FromRef, Clone)]
pub struct AppState {
    leptos_options: LeptosOptions,
    routes: Vec<RouteListing>,
    pub tls_state: TLSState,
    pub authorized_users: AuthorizedUsers,
    pub project_context: ProjectContext,
}

async fn run_main(tls_state: TLSState, mut context: ProjectContext) {
    // Setting get_configuration(None) means we'll be using cargo-leptos's env values
    // For deployment these variables are:
    // <https://github.com/leptos-rs/start-axum#executing-a-server-on-a-remote-machine-without-the-toolchain>
    // Alternately a file can be specified such as Some("Cargo.toml")
    // The file would need to be included with the executable when moved to deployment
    tracing::info!("Starting leptos service");
    let conf = get_configuration(None).await.unwrap();

    let leptos_options = conf.leptos_options;

    let addr = leptos_options.site_addr;
    let routes = generate_route_list(App);

    tracing::info!("Getting authorized users");
    let users = match get_authorized_users().await {
        Ok(users) => users,
        Err(err) => {
            eprintln!("Users get failed {err:?}");

            tracing::error!("Cant get authorized users");
            return;
        }
    };

    tracing::info!("Create app state");

    let app_state = AppState {
        routes: routes.clone(),
        leptos_options,
        tls_state,
        authorized_users: users,
        project_context: context.clone(),
    };

    let compression = tower_http::compression::CompressionLayer::new()
        .gzip(true)
        .br(true)
        .zstd(true)
        .deflate(true);

    // build our application with a route
    let app = Router::new()
        .route(
            "/cloud/image/push",
            post(image_uploader::push_image).layer(DefaultBodyLimit::max(1024 * 1024 * 400)),
        )
        .route(
            "/api/*fn_name",
            get(server_fn_handler).post(server_fn_handler),
        )
        .leptos_routes_with_handler(routes, get(leptos_routes_handler))
        .route("/.well-known/acme-challenge/:token", get(acme_handler))
        .route("/events/container/:id/stats/ws", get(container_stats_ws))
        .route("/events/container/:id/logs/ws", get(container_logs_ws))
        .route("/events/container/:id/attach/ws", get(container_attach_ws))
        .fallback(file_and_error_handler)
        .with_state(app_state)
        .layer(compression)
        .layer(CookieManagerLayer::new());

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    tracing::info!("listening on http://{}", &addr);

    let listener = loop {
        if let Ok(bind) = tokio::net::TcpListener::bind(addr).await {
            break bind;
        }
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    };

    tracing::info!("Load project config");
    if let Err(err) = context.load_from_config().await {
        tracing::error!("Failed to load config {err:?}");

        tracing::info!("Adding project");
        let project = match add_port_forward_project("cloud-panel", 3000, &mut context).await {
            Ok(project) => project,
            Err(err) => {
                tracing::error!("Cant create cloud-panel project {err:#?}");
                return;
            }
        };

        if let Err(err) = context
            .add_project_domain(project, "cloud.deepwith.in".to_string())
            .await
        {
            tracing::error!("Cant add panel domain {err:#?}");
            return;
        }
    }

    tracing::info!("Running server");

    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

async fn leptos_routes_handler(
    State(app_state): State<AppState>,
    // auth_session: AuthSession,
    // path: Path<String>,
    cookies: Cookies,
    request: Request<AxumBody>,
) -> Response {
    // info!("Handling request {:?}", request.uri());
    let auth = if let Some(cookie) = cookies.get("sessionId") {
        if let Ok(user) = get_user_from_cookie(cookie) {
            AuthType::Authorized(user)
        } else {
            AuthType::UnAuthorized
        }
    } else {
        AuthType::UnAuthorized
    };

    let handler = leptos_axum::render_route_with_context(
        app_state.leptos_options.clone(),
        app_state.routes.clone(),
        move || {
            provide_context(auth.clone());
            provide_context(app_state.project_context.clone());
            // provide_context(app_state.otp_map.clone());
        },
        App,
    );
    handler(request).await.into_response()
}

#[axum::debug_handler]
async fn server_fn_handler(
    State(app_state): State<AppState>,
    // auth_session: AuthSession,
    // path: Path<String>,
    cookies: Cookies,
    request: Request<AxumBody>,
) -> impl IntoResponse {
    // log!("{:?}", path);
    let auth = if let Some(cookie) = cookies.get("sessionId") {
        if let Ok(user) = get_user_from_cookie(cookie) {
            AuthType::Authorized(user)
        } else {
            AuthType::UnAuthorized
        }
    } else {
        AuthType::UnAuthorized
    };

    handle_server_fns_with_context(
        move || {
            provide_context(auth.clone());
            provide_context(app_state.authorized_users.clone());
            provide_context(cookies.clone());
            provide_context(app_state.project_context.clone());
            // provide_context(app_state.otp_map.clone());
        },
        request,
    )
    .await
}
