use std::collections::BinaryHeap;

use leptos::create_memo;
use leptos::create_server_action;
use leptos::create_signal;
use leptos::ev::InputEvent;
use leptos::event_target_value;
use leptos::expect_context;
use leptos::provide_context;
use leptos::For;
use leptos::Memo;
use leptos::Params;
use leptos::Resource;
use leptos::ServerFnError;
use leptos::SignalGet;
use leptos::SignalWith;
use leptos::Transition;
use leptos::{component, create_resource, view, IntoView};
use leptos_router::use_params;
use leptos_router::use_route;
use leptos_router::ActionForm;
use leptos_router::Outlet;
use leptos_router::Params;
use leptos_router::A;
use leptos_use::use_interval_fn;
use leptos_use::utils::Pausable;
use tracing::info;
use uuid::Uuid;

use crate::api::get_project;
use crate::api::get_project_domains;
use crate::api::AddProjectDomain;
use crate::api::UpdateProjectImage;
use crate::api::UpdateProjectPort;
use crate::common::Container;
use crate::common::Project;
use crate::common::ProjectType;
use leptos_router::Redirect;

#[derive(Params, PartialEq)]
struct ProjectParams {
    id: Uuid,
}

#[component]
pub fn ProjectPage() -> impl IntoView {
    let params = use_params::<ProjectParams>();

    let id = create_memo(move |_| {
        params.with(|params| params.as_ref().map(|param| param.id).unwrap_or_default())
    });

    let project = create_resource(id, move |id| async move { get_project(id).await });

    #[derive(Clone, Copy)]
    struct ChildMenus<'a> {
        name: &'a str,
        path: &'a str,
    }

    let menus = [
        ChildMenus {
            name: "General",
            path: "",
        },
        ChildMenus {
            name: "Domains",
            path: "/domains",
        },
    ];

    provide_context(project);

    provide_context(id);

    view! {

        <Transition>
            <div class="p-4">
                {
                    move || project.get().map(
                        |p| if let Ok(p) = p {
                            view! {

                                <h1 class="text-4xl"> { &p.name } </h1>
                                <div class="text-slate-600 dark:text-slate-400 text-sm"> {p.id.to_string()} </div>

                            }.into_view()
                        }else {
                            view! {
                                <Redirect path="../" />
                            }
                        }
                    )
                }
                <hr class="my-2" />


                <div class="flex flex-row gap-x-5">
                    <div class="w-40" >
                        {
                            menus.iter().cloned().map(|m| {

                                let is_active = create_memo(move|_|use_route().child().map(|r|r.path()).unwrap_or_default() == format!("{}{}",use_route().path(),m.path));
                                view! {
                                    <A href=move||format!("{}{}",use_route().path(),m.path)
                                        class="dark:hover:bg-white/10 hover:bg-black/10 p-3 rounded text-sm cursor-pointer text-slate-700 dark:text-white/65 block"
                                     > <span
                                     class=("text-black", move || is_active.get())
                                     class=("dark:text-white", move || is_active.get())
                                     class=("font-medium", move || is_active.get())
                                     > {m.name} </span> </A>
                                }
                            }).collect::<Vec<_>>()
                        }
                    </div>

                    <div class="w-full">
                        <Outlet />
                    </div>
                </div>
            </div>
        </Transition>
    }
}

pub fn ProjectSettings() -> impl IntoView {
    let project = expect_context::<Resource<Uuid, Result<Project, ServerFnError>>>();

    let project_type =
        create_memo(move |_| project.get().and_then(|p| p.ok()).map(|p| p.project_type));

    let (edited_project_type, set_edited_project_type) = create_signal(Option::<ProjectType>::None);

    let project_types = [
        (
            "PortForward",
            create_memo(move |_| {
                edited_project_type
                    .get()
                    .or(project_type.get())
                    .map(|p| p.is_port_forward())
                    .unwrap_or_default()
            }),
            create_memo(move |_| {
                project_type.get().and_then(|p| {
                    if p.is_port_forward() {
                        None
                    } else {
                        Some(ProjectType::PortForward(0))
                    }
                })
            }),
        ),
        (
            "Container",
            create_memo(move |_| {
                edited_project_type
                    .get()
                    .or(project_type.get())
                    .map(|p| p.is_container())
                    .unwrap_or_default()
            }),
            create_memo(move |_| {
                project_type.get().and_then(|p| {
                    if p.is_container() {
                        None
                    } else {
                        Some(ProjectType::Container(Container {
                            image: String::new(),
                            port_mapping: (0, 0),
                            #[cfg(feature = "ssr")]
                            status: None,
                        }))
                    }
                })
            }),
        ),
    ];

    let update_port_action = create_server_action::<UpdateProjectPort>();
    let update_image_action = create_server_action::<UpdateProjectImage>();

    view! {


        <Transition>
            <div>
                <div class="text-xl "> "Project Type" </div>
                <div class="h-2"/>
                <div class="flex gap-3" >
                    {
                        project_types.into_iter().map(|p| view! {
                            <div class="p-2 text-sm rounded-md hover:bg-black/20 dark:hover:bg-white/20 cursor-pointer"
                                class=("dark:text-white", p.1)
                                class=("text-black", p.1)
                                class=("bg-black/10", p.1)
                                class=("dark:bg-white/30", p.1)

                                class=("text-black/60", move || !p.1.get())
                                class=("dark:text-white/60", move || !p.1.get())

                                on:click=move|_| {
                                    set_edited_project_type(p.2.get())
                                }
                            > {p.0}  </div>
                        }).collect::<Vec<_>>()
                    }
                </div>
                <div class="h-2"/>
                {
                    move || match edited_project_type.get().or(project_type.get()) {
                        Some(project_type) => match project_type {
                            ProjectType::PortForward(port) => {
                                view! {
                                    <ActionForm action=update_port_action>
                                        <div class="text-md"> "Port" </div>
                                        <input name="id" type="hidden" prop:value=move|| project.get().and_then(|p| p.ok()).map(|p| p.id.to_string())/>
                                        <input name="port" prop:value=port type="number" class="border p-2 rounded-md dark:bg-white/10 dark:border-white/5"/>
                                        <div class="h-2" />
                                        <input type="submit" value="Update" class="cursor-pointer block border p-2 px-10 rounded bg-slate-800 text-white disabled:cursor-no-drop disabled:bg-slate-200 disabled:text-black dark:disabled:bg-white/20 dark:disabled:text-white dark:border-none dark:bg-white/90 dark:text-black"
                                        />
                                    </ActionForm>
                                }.into_view()
                            },
                            ProjectType::Container(container) => {
                                let c = container.clone();
                                view! {
                                    <ActionForm action=update_image_action>
                                        <input name="id" type="hidden" prop:value=move|| project.get().and_then(|p| p.ok()).map(|p| p.id.to_string())/>
                                        <div class="text-md"> "Image" </div>
                                        <input name="image" value={container.image} type="text" class="border p-2 rounded-md w-full dark:bg-white/10 dark:border-white/5"
                                            on:input= move |ev| {
                                                let value =  event_target_value(&ev);
                                                set_edited_project_type(
                                                    Some(ProjectType::Container(Container{
                                                        image:value,
                                                        ..c.clone()
                                                    }))
                                                );
                                            }
                                         />
                                        <div class="h-4"/>

                                        <div class="text-md"> "Port Mapping" </div>
                                        <div class="flex gap-2">
                                            <input name="container_port" value={container.port_mapping.0} type="number" class="border p-2 rounded-md dark:bg-white/10 dark:border-white/5" />
                                            <input name="host_port" value={container.port_mapping.1} type="number" class="border p-2 rounded-md dark:bg-white/10 dark:border-white/5" />
                                        </div>

                                        <div class="h-2" />
                                        <input type="submit" value="Update" class="cursor-pointer block border p-2 px-10 rounded bg-slate-800 text-white disabled:cursor-no-drop disabled:bg-slate-200 disabled:text-black dark:disabled:bg-white/20 dark:disabled:text-white dark:border-none dark:bg-white/90 dark:text-black"
                                        />
                                    </ActionForm>
                                }.into_view()
                            },
                        },
                        None => view! {}.into_view(),
                    }
                }
            </div>
        </Transition>

    }
}

pub fn DomainsList() -> impl IntoView {
    let id = expect_context::<Memo<Uuid>>();
    let add_domain_action = create_server_action::<AddProjectDomain>();

    let domains = create_resource(
        move || {
            // add_domain_action.version().get();
            id.get()
        },
        move |id| async move {
            let result = get_project_domains(id).await;
            let mut result = result.unwrap_or_default();

            // result.sort_by_key(|p| p.0.to_string());
            result
        },
    );

    let Pausable {
        pause,
        resume,
        is_active,
    } = use_interval_fn(
        move || {
            domains.refetch();
        },
        5000,
    );

    let (new_domain, set_new_domain) = create_signal(String::new());

    view! {

        <div class="p-2 text-xl" > "Domains" </div>

        <div class="p-2">
            <ActionForm action=add_domain_action>
                <div class="w-full rounded-md flex gap-5">
                    <input type="hidden" name="id" prop:value= move || id.get().to_string() />
                    <input name="domain" id="domain" placeholder="example.com" class="p-2 border w-full rounded bg-white dark:bg-white/10 dark:border-white/5"
                        on:input=move |ev| {
                            set_new_domain(event_target_value(&ev));
                        }
                        prop:value=new_domain
                    />
                    <input type="submit" value="Add" class="border p-2 px-10 rounded bg-slate-800 text-white disabled:cursor-no-drop disabled:bg-slate-200 disabled:text-black dark:disabled:bg-white/20 dark:disabled:text-white dark:border-none dark:bg-white/90 dark:text-black"
                        disabled=move || new_domain.get().is_empty()
                        prop:disabled=move || new_domain.get().is_empty()
                    />
                </div>
            </ActionForm>
        </div>

        <Transition>
            <For
                each= move || {domains.get().unwrap_or_default().keys().cloned().collect::<BinaryHeap<_>>().into_sorted_vec()}

                key=|domain| domain.clone()
                children=move |domain| {
                    let dc = domain.clone();
                    let status = create_memo( move |_| domains.get().unwrap_or_default().get(&dc).cloned());

                    view! {
                        <div class="p-2">
                            <div class="w-full border bg-white dark:bg-white/10 dark:border-white/20 rounded-md p-4">
                                <div class="text-xl flex items-center ">  {domain}
                                <span class="text-slate-600 dark:text-slate-300 text-sm px-4 py-1 bg-slate-200 dark:bg-slate-700 rounded-full w-fit ml-2 flex items-center justify-center">

                                <div class="w-2 h-2 rounded-full mr-2 inline-block"
                                    class=("bg-green-500", move || status.get().map(|s|s.ssl_provision.is_provisioned()).unwrap_or_default())
                                    class=("bg-yellow-500", move || status.get().map(|s|s.ssl_provision.is_not_provisioned()).unwrap_or_default())
                                >
                                </div>
                                {
                                move || match status.get().map(|s|s.ssl_provision).unwrap_or(crate::common::SSLProvisioning::NotProvisioned) {
                                        crate::common::SSLProvisioning::NotProvisioned => "Waiting",
                                        crate::common::SSLProvisioning::Provisioning => "Processing",
                                        crate::common::SSLProvisioning::Provisioned(_) => "Active",
                                    }
                                }
                                </span>
                                </div>

                            </div>
                        </div>
                    }
                }
            />
        </Transition>

    }
}