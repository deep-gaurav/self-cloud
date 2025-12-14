use crate::common::Project;
use crate::common::{Container, ProjectType, SupportContainer};
use leptos::prelude::*;
use leptos::server_fn::ServerFn;
use leptos_router::components::Form;
use uuid::Uuid;

use crate::api::SetSupportContainers;
use crate::common::EnvironmentVar;

#[component]
pub fn SupportContainers() -> impl IntoView {
    let id = expect_context::<Signal<Uuid>>();

    let project = expect_context::<Resource<Result<Project, ServerFnError>>>();

    let project_type =
        Memo::new(move |_| project.get().and_then(|p| p.ok()).map(|p| p.project_type));

    let update_support_container_action = ServerAction::<SetSupportContainers>::new();
    view! {
        <Transition>

            {move || {
                let project_type = project_type.get();
                match project_type {
                    Some(project_type) => {
                        match project_type {
                            ProjectType::Container { support_containers, .. } => {
                                let (support_containers, set_support_containers) = signal(
                                    support_containers,
                                );
                                view! {
                                    <input type="hidden" name="id" value=move || id.get().to_string()/>
                                    <div class="text-xl " class=("abc", move || true)>
                                        "Service Containers"
                                    </div>
                                    <div class="h-2"></div>
                                    <div class="flex gap-2">

                                        {
                                            let (new_service_name, set_new_service_name) = signal(
                                                String::new(),
                                            );
                                            view! {
                                                <input
                                                    class="p-2 border w-full rounded bg-white dark:bg-white/10 dark:border-white/5"
                                                    prop:value=new_service_name
                                                    on:input=move |ev| {
                                                        set_new_service_name.set(event_target_value(&ev));
                                                    }
                                                />

                                                <button
                                                    type="button"
                                                    class="flex-shrink-0 border p-2 px-10 rounded bg-slate-800 text-white disabled:cursor-no-drop disabled:bg-slate-200 disabled:text-black dark:disabled:bg-white/20 dark:disabled:text-white dark:border-none dark:bg-white/90 dark:text-black"
                                                    disabled=move || new_service_name.get().is_empty()
                                                    prop:disabled=move || new_service_name.get().is_empty()
                                                    on:click=move |_| {
                                                        let mut containers = support_containers.get_untracked();
                                                        containers
                                                            .insert(
                                                                new_service_name.get_untracked(),
                                                                SupportContainer {
                                                                    image: String::new(),
                                                                    container: Container {
                                                                        env_vars: vec![].into(),
                                                                        #[cfg(feature = "ssr")]
                                                                        status: crate::common::ContainerStatus::None,
                                                                    },
                                                                },
                                                            );
                                                        set_support_containers.set(containers);
                                                    }
                                                >

                                                    "Add New"
                                                </button>
                                            }
                                        }

                                    </div>
                                    <div class="h-2"></div>
                                    <ActionForm action=update_support_container_action>

                                        <input
                                            name="id"
                                            type="hidden"
                                            prop:value=move || {
                                                project.get().and_then(|p| p.ok()).map(|p| p.id.to_string())
                                            }
                                        />

                                        <For
                                            each=move || support_containers.get().into_iter()
                                            key=|p| p.0.clone()
                                            children=move |cont| {
                                                let name = StoredValue::new(cont.0.clone());
                                                let (env_vars, set_env_vars) = signal({
                                                    let mut map = Vec::with_capacity(
                                                        cont.1.container.env_vars.len(),
                                                    );
                                                    for var in cont.1.container.env_vars.into_iter() {
                                                        map.push((map.len(), var));
                                                    }
                                                    map
                                                });
                                                view! {
                                                    <div class="border p-4 dark:bg-white/10 bg-black/10 dark:border-white/20 rounded-md">

                                                        <div class="text-md">{name.get_value()}</div>
                                                        <div class="h-4"></div>
                                                        <input
                                                            name=format!(
                                                                "support_containers[{}][name]",
                                                                name.get_value(),
                                                            )

                                                            type="hidden"
                                                            prop:value=cont.0
                                                        />
                                                        <label for="port" class="text-sm dark:text-white/50">
                                                            "Image"
                                                        </label>
                                                        <input
                                                            id="image"
                                                            class="p-2 border w-full rounded bg-white dark:bg-white/10 dark:border-white/5"
                                                            name=format!(
                                                                "support_containers[{}][image]",
                                                                name.get_value(),
                                                            )

                                                            prop:value=cont.1.image
                                                        />
                                                        <div class="h-4"></div>

                                                        <div class="text-md">"Environment Variable"</div>

                                                        <div class="">
                                                            <For
                                                                each=move || env_vars.get().into_iter()
                                                                key=|p| p.0
                                                                children=move |(index, environment_var)| {
                                                                    view! {
                                                                        <div class="flex flex-col gap-4 p-2 border dark:border-white/20 m-2 rounded">
                                                                            <div class="flex gap-4 flex-wrap">

                                                                                <div class=" flex flex-col">
                                                                                    <label for="port" class="text-sm dark:text-white/50">
                                                                                        "Key"
                                                                                    </label>
                                                                                    <input
                                                                                        prop:value=environment_var.key
                                                                                        type="text"
                                                                                        id="key"
                                                                                        name=format!(
                                                                                            "support_containers[{}][env_vars][{}][key]",
                                                                                            name.get_value(),
                                                                                            index,
                                                                                        )

                                                                                        required
                                                                                        class="border p-2 rounded-md dark:bg-white/10 dark:border-white/5"
                                                                                    />
                                                                                </div>

                                                                                <div class=" flex flex-col">

                                                                                    <label for="domain" class="text-sm dark:text-white/50">
                                                                                        "Value"
                                                                                    </label>

                                                                                    <input
                                                                                        prop:value=environment_var.val
                                                                                        type="text"
                                                                                        id="val"
                                                                                        name=format!(
                                                                                            "support_containers[{}][env_vars][{}][val]",
                                                                                            name.get_value(),
                                                                                            index,
                                                                                        )

                                                                                        required
                                                                                        class="border p-2 rounded-md dark:bg-white/10 dark:border-white/5"
                                                                                    />
                                                                                </div>

                                                                                <button
                                                                                    type="button"
                                                                                    class="p-2 rounded bg-red-700 px-6 text-white mt-5"
                                                                                    on:click=move |_| {
                                                                                        let mut env_vars = env_vars.get_untracked();
                                                                                        env_vars.remove(index);
                                                                                        set_env_vars.set(env_vars)
                                                                                    }
                                                                                >

                                                                                    "Remove Variable"
                                                                                </button>
                                                                            </div>
                                                                        </div>
                                                                    }
                                                                }
                                                            />

                                                            <button
                                                                type="button"
                                                                class="p-2 rounded border bg-white/90 px-6 text-black"
                                                                on:click=move |_| {
                                                                    let new_var = EnvironmentVar {
                                                                        key: "".to_string(),
                                                                        val: "".to_string(),
                                                                    };
                                                                    let mut vars = env_vars.get_untracked();
                                                                    vars.push((
                                                                        vars.last().map(|p| p.0).unwrap_or_default() + 1,
                                                                        new_var,
                                                                    ));
                                                                    set_env_vars.set(vars);
                                                                }
                                                            >

                                                                "Add New Environment Variable"
                                                            </button>
                                                        </div>

                                                    </div>
                                                }
                                            }
                                        />

                                        <div class="h-4"></div>

                                        <input
                                            type="submit"
                                            value="Update"
                                            class="cursor-pointer block border p-2 px-10 rounded bg-slate-800 text-white disabled:cursor-no-drop disabled:bg-slate-200 disabled:text-black dark:disabled:bg-white/20 dark:disabled:text-white dark:border-none dark:bg-white/90 dark:text-black"
                                        />

                                    </ActionForm>
                                }
                                    .into_any()
                            }
                            ProjectType::PortForward(_) => view! {}.into_any(),
                        }
                    }
                    None => view! {}.into_any(),
                }
            }}

        </Transition>
    }
}
