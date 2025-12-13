use crate::api::AddProject;
use leptos::prelude::*;
use leptos::server_fn::ServerFn;
use leptos_router::components::{Outlet, A};

use crate::api::get_projects;

pub mod container_page;
pub mod project_page;
pub mod settings;
pub mod support_containers;

#[component]
pub fn ProjectsList() -> impl IntoView {
    let projects = Resource::new(
        || (),
        move |_| async {
            leptos::logging::log!("ProjectsList resource fetching");
            let result = get_projects().await;
            result.unwrap_or_default()
        },
    );
    let create_project = ServerAction::<AddProject>::new();
    let (new_project_name, set_new_project_name) = signal(String::new());

    Effect::new(move |_| {
        create_project.value().get();
        set_new_project_name.set(String::new());
        projects.refetch();
    });

    view! {
        <div class="p-2">

            <h1 class="text-4xl font-bold">"Projects"</h1>
            <div class="h-4"></div>

            <div class="flex flex-col gap-4">
                <div class="p-4 border rounded shadow-sm bg-white dark:bg-zinc-900 dark:border-zinc-800">
                    <h2 class="text-xl font-semibold mb-2">"Create Project"</h2>
                    <ActionForm action=create_project attr:class="flex flex-col gap-3 max-w-sm">
                        <label class="flex flex-col gap-1">
                            <span class="text-sm font-medium">"Project Name"</span>
                            <input
                                type="text"
                                name="name"
                                class="p-2 border rounded focus:ring-2 focus:ring-blue-500 outline-none dark:bg-zinc-800 dark:border-zinc-700"
                                placeholder="My Awesome Project"
                                required
                                on:input=move |ev| {
                                    let val = event_target_value(&ev);
                                    leptos::logging::log!("Input changed: {}", val);
                                    set_new_project_name.set(val);
                                }
                                // prop:value=new_project_name
                            />
                        </label>
                        <input
                            type="submit"
                            value="Add"
                            class="border p-2 px-10 rounded bg-slate-800 text-white disabled:cursor-no-drop disabled:bg-slate-200 disabled:text-black dark:disabled:bg-white/20 dark:disabled:text-white dark:border-none dark:bg-white/90 dark:text-black"
                            disabled=move || new_project_name.get().is_empty()
                            prop:disabled=move || new_project_name.get().is_empty()
                        />
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
                                        attr:class="w-full h-full p-4 flex flex-col gap-2"
                                    >
                                        <div class="text-xl">{p.name}</div>
                                        <div class="text-slate-600 text-sm">{p.id.to_string()}</div>
                                        <div class="h-2"></div>
                                    </A>
                                </div>
                            }
                        })
                        .collect::<Vec<_>>()
                }}

            </Suspense>
        </div>
        </div>
    }
}

#[component]
pub fn ProjectsHome() -> impl IntoView {
    view! { <Outlet/> }
}
