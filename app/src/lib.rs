use crate::{
    auth::get_auth,
    error_template::{AppError, ErrorTemplate},
    pages::project::project_page::GeneralSettings,
};

use crate::auth::AuthType;
use crate::auth::Login;
use crate::pages::dashboard::Dashboard;
use crate::pages::home::HomePage;
use crate::pages::project::container_page::ContainerPage;
use crate::pages::project::settings::ProjectSettings;
use crate::pages::project::support_containers::SupportContainers;
use crate::pages::project::{project_page::ProjectPage, ProjectsHome, ProjectsList};
use crate::pages::settings::Settings;

use leptos::prelude::*;
use leptos_meta::*;
use leptos_router::components::*;
use leptos_router::*;

// use leptos_toaster::Toaster; (Removed)
use crate::components::toaster::Toaster;

pub mod api;
pub mod auth;
pub mod common;
pub mod components;
#[cfg(feature = "ssr")]
pub mod context;
pub mod error_template;
pub mod hooks;
pub mod pages;
pub mod updates;
pub mod utils;

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>

                // <link rel="icon" type="image/svg+xml" href={SITE_LOGO_ABS}/>
                <link rel="preconnect" href="https://fonts.googleapis.com"/>
                <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin="anonymous"/>
                {
                    leptos::tachys::html::element::link()
                        .rel("preload")
                        .href("https://fonts.googleapis.com/css2?family=Cormorant+Garamond:wght@400;600;700&family=Lato:wght@300;400;700&display=swap")
                        .r#as("style")
                        .onload("this.onload=null;this.rel='stylesheet'")
                }
                <noscript>
                    <link href="https://fonts.googleapis.com/css2?family=Cormorant+Garamond:wght@400;600;700&family=Lato:wght@300;400;700&display=swap" rel="stylesheet"/>
                </noscript>
                <link rel="preload" href="/pkg/start-axum-workspace.css" attr:as="style"/>
                <link rel="stylesheet" href="/pkg/start-axum-workspace.css"/>

                <AutoReload options=options.clone()/>
                <HydrationScripts options/>
                <MetaTags/>
            </head>
            <body class="font-display w-full bg-rooh_cream">
                <App/>
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    let login = ServerAction::<Login>::new();
    let auth = Resource::new(move || login.version().get(), move |_| get_auth());
    provide_context(auth);

    view! {
        <Stylesheet id="leptos" href="/pkg/start-axum-workspace.css"/>

        // sets the document title
        <Title text="Welcome to SelfCloud"/>

        <Toaster>

            // content for this welcome page
            <Router>

                <main class="min-h-full w-full bg-slate-100 dark:bg-black dark:text-slate-50 flex flex-col">
                    <Routes fallback=|| {
                        let mut outside_errors = Errors::default();
                        outside_errors.insert_with_default_key(AppError::NotFound);
                        view! { <ErrorTemplate outside_errors/> }.into_view()
                    }>
                        <Route
                            ssr=SsrMode::PartiallyBlocked
                            path=leptos_router::path!("")
                            view=move || view! {
                                <AuthCheck is_auth_required=false/>
                                <HomePage login=login/>
                            }
                        />

                        <Route
                            ssr=SsrMode::PartiallyBlocked
                            path=leptos_router::path!("dashboard")
                            view=move || view! {
                                <AuthCheck is_auth_required=true/>
                                <Dashboard/>
                            }
                        />

                        <ParentRoute
                            ssr=SsrMode::PartiallyBlocked
                            path=leptos_router::path!("projects")
                            view=ProjectsHome
                        >
                            <Route path=leptos_router::path!("") view=ProjectsList/>
                            <ParentRoute path=leptos_router::path!(":id") view=ProjectPage>
                                <Route path=leptos_router::path!("") view=GeneralSettings/>
                                <Route path=leptos_router::path!("container") view=ContainerPage/>
                                <Route path=leptos_router::path!("settings") view=ProjectSettings/>
                                <Route path=leptos_router::path!("support") view=SupportContainers/>
                            </ParentRoute>
                        </ParentRoute>

                    </Routes>
                </main>
            </Router>
        </Toaster>
    }
}

#[component]
pub fn AuthCheck(is_auth_required: bool) -> impl IntoView {
    let auth = expect_context::<Resource<Result<AuthType, ServerFnError>>>();

    view! {
        <Suspense fallback=move || view! { <div class="centered">"Loading..."</div> }>
            {move || {
                auth.get().map(|auth| {
                    match auth {
                        Ok(auth) => {
                            if is_auth_required && auth.is_un_authorized() {
                                view! { <Redirect path="/"/> }.into_any()
                            } else {
                                view! { }.into_any()
                            }
                        }
                        Err(_) => {
                             if is_auth_required {
                                view! { <Redirect path="/"/> }.into_any()
                             } else {
                                view! { }.into_any()
                             }
                        }
                    }
                })
            }}
        </Suspense>
    }
}

// #[component]
// pub fn AuthCheck(
//    login: ServerAction<Login>,
//    is_auth_required: bool,
// ) -> impl IntoView {
//    // ...
// }
