use crate::{
    api::get_projects,
    auth::{get_auth, AuthType},
    error_template::{AppError, ErrorTemplate},
};

use crate::pages::dashboard::Dashboard;
use crate::pages::home::HomePage;

use leptos::*;
use leptos_meta::*;
use leptos_router::*;
use tracing::info;

pub mod api;
pub mod auth;
pub mod common;
pub mod error_template;
pub mod pages;

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    view! {
        <Stylesheet id="leptos" href="/pkg/start-axum-workspace.css"/>

        // sets the document title
        <Title text="Welcome to Leptos"/>

        // content for this welcome page
        <Router fallback=|| {
            let mut outside_errors = Errors::default();
            outside_errors.insert_with_default_key(AppError::NotFound);
            view! { <ErrorTemplate outside_errors/> }.into_view()
        }>

            <main class="h-full w-full">
                <Routes>
                    <Route ssr=SsrMode::PartiallyBlocked path="" view=|| view! {
                        <AuthCheck is_auth_required=false />
                    } >
                        <Route path="" view=HomePage/>
                    </Route>

                    <Route ssr=SsrMode::PartiallyBlocked path="dashboard" view=|| view! {
                        <AuthCheck is_auth_required=true />
                    } >
                        <Route path="" view=Dashboard/>
                    </Route>
                </Routes>
            </main>
        </Router>
    }
}

#[component]
pub fn AuthCheck(is_auth_required: bool) -> impl IntoView {
    let auth = create_blocking_resource(
        || (),
        move |_| async {
            let result = get_auth().await;
            result.unwrap_or(AuthType::UnAuthorized)
        },
    );

    view! {
        <Suspense
            fallback=move || view! { <p>"Loading..."</p> }
        >
            {
                let user = auth.get().unwrap_or(AuthType::UnAuthorized);
                if auth.loading().get() {
                    view!{}.into_view()
                }else{
                    match user {
                        AuthType::UnAuthorized => {
                            if is_auth_required {
                                view!{
                                    <Redirect path="/" />
                                }.into_view()
                            } else {
                                view!{
                                    <Outlet/>
                                }.into_view()
                            }
                        },
                        AuthType::Authorized(_) => {
                            if !is_auth_required {
                                view!{
                                    <Redirect path="/dashboard" />
                                }.into_view()
                            } else {
                                view!{
                                    <Outlet/>
                                }.into_view()
                            }
                        },
                    }
                }
            }
        </Suspense>
    }
}
