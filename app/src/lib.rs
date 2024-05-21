use crate::{
    api::get_projects,
    error_template::{AppError, ErrorTemplate},
};

use leptos::*;
use leptos_meta::*;
use leptos_router::*;
use tracing::info;

pub mod api;
pub mod common;
pub mod error_template;

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
            <main>
                <Routes>
                    <Route path="" view=HomePage/>
                </Routes>
            </main>
        </Router>
    }
}

/// Renders the home page of your application.
#[component]
fn HomePage() -> impl IntoView {
    let projects = create_resource(
        || (),
        move |_| async {
            let result = get_projects().await;
            result.unwrap_or_default()
        },
    );

    view! {
        <h1 class="text-4xl">"Projects"</h1>

        <Suspense>
            {
                move || projects.get().unwrap_or_default().into_iter().map(
                    |p| view! {
                        <div class="p-2">
                            <div class="w-full shadow-md rounded-md p-4">
                                <div class="text-xl"> {p.name} </div>
                                <div class="text-slate-600 text-sm"> {p.id.to_string()} </div>
                                <div class="h-2" />
                                <div > "Port: " {p.port} </div>
                            </div>
                        </div>
                    }
                ).collect::<Vec<_>>()
            }
        </Suspense>
    }
}
