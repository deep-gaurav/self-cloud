use leptos::SignalGet;
use leptos::Suspense;
use leptos::{component, create_resource, view, IntoView};

use crate::api::get_projects;

/// Renders the home page of your application.
#[component]
pub fn Dashboard() -> impl IntoView {
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
