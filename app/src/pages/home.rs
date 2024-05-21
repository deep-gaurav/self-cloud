use leptos::SignalGet;
use leptos::Suspense;
use leptos::{component, create_resource, view, IntoView};

use crate::api::get_projects;

/// Renders the home page of your application.
#[component]
pub fn HomePage() -> impl IntoView {
    view! {
        <div class="w-full h-full flex items-center justify-center">
            <h1 class="text-3xl"> "Welcome to Self Cloud" </h1>
        </div>
    }
}
