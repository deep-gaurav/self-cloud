use leptos::SignalGet;
use leptos::Suspense;
use leptos::{component, create_resource, view, IntoView};
use leptos_router::Redirect;

/// Renders the home page of your application.
#[component]
pub fn Dashboard() -> impl IntoView {
    view! { <Redirect path="projects"/> }
}
