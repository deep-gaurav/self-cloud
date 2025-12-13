use leptos::prelude::*;
use leptos_router::components::Redirect;

/// Renders the home page of your application.
#[component]
pub fn Dashboard() -> impl IntoView {
    view! { <Redirect path="/projects"/> }
}
