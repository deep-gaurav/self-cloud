use crate::api::AddProject;
use leptos::create_effect;
use leptos::create_server_action;
use leptos::create_signal;
use leptos::event_target_value;
use leptos::SignalGet;
use leptos::SignalSet;
use leptos::Suspense;
use leptos::{component, create_resource, view, IntoView};
use leptos_router::ActionForm;
use leptos_router::Outlet;
use leptos_router::A;

use crate::api::get_projects;

pub mod container_page;
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
    let add_new_project = create_server_action::<AddProject>();
    let (new_project_name, set_new_project_name) = create_signal(String::new());

    create_effect(move |_| {
        add_new_project.value().get();
        set_new_project_name.set(String::new());
        projects.refetch();
    });

    view! {
        <h1 class="text-4xl">"Projects"</h1>

        <div class="p-2">
            <ActionForm action=add_new_project>
                <div class="w-full rounded-md flex gap-5">
                    <input
                        name="name"
                        id="domain"
                        placeholder="New Project name"
                        class="p-2 border w-full rounded bg-white dark:bg-white/10 dark:border-white/5"
                        on:input=move |ev| {
                            set_new_project_name.set(event_target_value(&ev));
                        }

                        prop:value=new_project_name
                    />
                    <input
                        type="submit"
                        value="Add"
                        class="border p-2 px-10 rounded bg-slate-800 text-white disabled:cursor-no-drop disabled:bg-slate-200 disabled:text-black dark:disabled:bg-white/20 dark:disabled:text-white dark:border-none dark:bg-white/90 dark:text-black"
                        disabled=move || new_project_name.get().is_empty()
                        prop:disabled=move || new_project_name.get().is_empty()
                    />
                </div>
            </ActionForm>
        </div>

        <Suspense>

            {move || {
                projects
                    .get()
                    .unwrap_or_default()
                    .into_iter()
                    .map(|p| {
                        view! {
                            <div class="p-2">
                                <A
                                    href=p.id.to_string()
                                    class="w-full shadow-md rounded-md p-4 block"
                                >
                                    <div class="text-xl">{p.name}</div>
                                    <div class="text-slate-600 text-sm">{p.id.to_string()}</div>
                                    <div class="h-2"></div>
                                // <div > "Port: " {p.port} </div>
                                </A>
                            </div>
                        }
                    })
                    .collect::<Vec<_>>()
            }}

        </Suspense>
    }
}

#[component]
pub fn ProjectsHome() -> impl IntoView {
    view! { <Outlet/> }
}
