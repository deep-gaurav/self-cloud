use leptos::SignalGet;
use leptos::Suspense;
use leptos::{component, create_resource, view, IntoView};
use leptos_router::Outlet;
use leptos_router::A;

use crate::api::get_projects;

pub mod project_page;

#[component]
pub fn ProjectsList() -> impl IntoView {
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
                            <A href={p.id.to_string()} class="w-full shadow-md rounded-md p-4 block">
                                <div class="text-xl"> {p.name} </div>
                                <div class="text-slate-600 text-sm"> {p.id.to_string()} </div>
                                <div class="h-2" />
                                <div > "Port: " {p.port} </div>
                            </A>
                        </div>
                    }
                ).collect::<Vec<_>>()
            }
        </Suspense>
    }
}

#[component]
pub fn ProjectsHome() -> impl IntoView {
    view! {
        <Outlet />
    }
}
