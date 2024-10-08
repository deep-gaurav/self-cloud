use crate::{
    auth::{get_auth, AuthType},
    error_template::{AppError, ErrorTemplate},
    pages::project::project_page::{DomainsList, GeneralSettings},
};

use crate::auth::Login;
use crate::components::nav_bar::NavBar;
use crate::pages::dashboard::Dashboard;
use crate::pages::home::HomePage;
use crate::pages::project::container_page::ContainerPage;
use crate::pages::project::settings::ProjectSettings;
use crate::pages::project::support_containers::SupportContainers;
use crate::pages::project::{project_page::ProjectPage, ProjectsHome, ProjectsList};

use leptos::*;
use leptos_meta::*;
use leptos_router::*;

use leptos_toaster::Toaster;

pub mod api;
pub mod auth;
pub mod common;
pub mod components;
#[cfg(feature = "ssr")]
pub mod context;
pub mod error_template;
pub mod pages;
pub mod utils;

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    let login = create_server_action::<Login>();

    view! {
        <Stylesheet id="leptos" href="/pkg/start-axum-workspace.css"/>

        // sets the document title
        <Title text="Welcome to SelfCloud"/>

        <Toaster position=leptos_toaster::ToasterPosition::BottomRight>

            // content for this welcome page
            <Router fallback=|| {
                let mut outside_errors = Errors::default();
                outside_errors.insert_with_default_key(AppError::NotFound);
                view! { <ErrorTemplate outside_errors/> }.into_view()
            }>

                <main class="min-h-full w-full bg-slate-100 dark:bg-black dark:text-slate-50 flex flex-col">
                    <Routes>
                        <Route
                            ssr=SsrMode::PartiallyBlocked
                            path=""
                            view=move || view! { <AuthCheck login=login is_auth_required=false/> }
                        >
                            <Route path="" view=move || view! { <HomePage login=login/> }/>
                        </Route>

                        <Route
                            ssr=SsrMode::PartiallyBlocked
                            path="dashboard"
                            view=move || view! { <AuthCheck login=login is_auth_required=true/> }
                        >
                            <Route path="" view=Dashboard/>
                            <Route path="projects" view=ProjectsHome>
                                <Route path="" view=ProjectsList/>
                                <Route path=":id" view=ProjectPage>
                                    <Route path="" view=GeneralSettings/>
                                    <Route path="domains" view=DomainsList/>
                                    <Route path="container" view=ContainerPage/>
                                    <Route path="settings" view=ProjectSettings/>
                                    <Route path="services" view=SupportContainers/>
                                </Route>
                            </Route>
                        </Route>
                    </Routes>
                </main>
            </Router>
        </Toaster>
    }
}

#[component]
pub fn AuthCheck(
    is_auth_required: bool,
    login: Action<Login, Result<(), ServerFnError>>,
) -> impl IntoView {
    let auth = create_blocking_resource(
        move || {
            login.version().get();
        },
        move |_| async {
            let result = get_auth().await;
            result.unwrap_or(AuthType::UnAuthorized)
        },
    );

    provide_context(auth);

    view! {
        <Suspense fallback=move || {
            view! { <p>"Loading..."</p> }
        }>

            {
                let user = auth.get().unwrap_or(AuthType::UnAuthorized);
                if auth.loading().get() {
                    view! {}.into_view()
                } else {
                    match user {
                        AuthType::UnAuthorized => {
                            if is_auth_required {
                                view! { <Redirect path="/"/> }.into_view()
                            } else {
                                view! { <Outlet/> }.into_view()
                            }
                        }
                        AuthType::Authorized(_) => {
                            if !is_auth_required {
                                view! { <Redirect path="/dashboard"/> }.into_view()
                            } else {
                                view! {
                                    <NavBar/>
                                    <Outlet/>
                                }
                                    .into_view()
                            }
                        }
                    }
                }
            }

        </Suspense>
    }
}
