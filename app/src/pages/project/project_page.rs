use std::collections::BinaryHeap;
use std::collections::HashMap;
use std::sync::Arc;

use leptos::create_effect;
use leptos::create_memo;
use leptos::create_server_action;
use leptos::create_signal;
use leptos::event_target_value;
use leptos::expect_context;
use leptos::provide_context;
use leptos::For;
use leptos::Params;
use leptos::Resource;
use leptos::ServerFnError;
use leptos::SignalGet;
use leptos::SignalGetUntracked;
use leptos::SignalSet;
use leptos::SignalWithUntracked;
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
use uuid::Uuid;

use crate::api::get_project;
use crate::api::get_project_domains;
use crate::api::AddProjectDomain;
use crate::api::UpdateProjectImage;
use crate::api::UpdateProjectPort;
use crate::common::Container;
use crate::common::ExposedPort;
use crate::common::PortForward;
use crate::common::Project;
use crate::common::ProjectType;
use crate::utils::random_ascii_string;
use leptos_router::Redirect;

#[derive(Params, PartialEq)]
struct ProjectParams {
    id: Option<Uuid>,
}

#[component]
pub fn ProjectPage() -> impl IntoView {
    let params = use_params::<ProjectParams>();

    let id = params.with_untracked(|params| {
        params
            .as_ref()
            .map(|param| param.id.unwrap_or_default())
            .unwrap_or_default()
    });

    let project = create_resource(|| {}, move |_| async move { get_project(id).await });

    #[derive(Clone, Copy, PartialEq)]
    struct ChildMenus<'a> {
        name: &'a str,
        path: &'a str,
    }

    provide_context(project);

    provide_context(id);

    view! {
        <Transition>
            <div class="p-4">
                {move || {
                    project
                        .get()
                        .map(|p| {
                            if let Ok(p) = p {
                                view! {
                                    <h1 class="text-4xl">{p.name}</h1>
                                    <div class="text-slate-600 dark:text-slate-400 text-sm">
                                        {p.id.to_string()}
                                    </div>
                                }
                                    .into_view()
                            } else {
                                view! { <Redirect path="../"/> }
                            }
                        })
                }}
                <hr class="my-2"/> <div class="flex flex-col gap-5 sm:flex-row">
                    <div class="w-40 flex flex-row sm:flex-col">

                        <For
                            each=move || {
                                let proj = project.get();
                                let is_project_container = proj
                                    .and_then(|p| p.ok())
                                    .map(|p| p.project_type.is_container())
                                    .unwrap_or_default();
                                let mut pages = vec![
                                    ChildMenus {
                                        name: "General",
                                        path: "",
                                    },
                                    ChildMenus {
                                        name: "Domains",
                                        path: "/domains",
                                    },
                                ];
                                if is_project_container {
                                    pages
                                        .push(ChildMenus {
                                            name: "Container",
                                            path: "/container",
                                        });
                                }
                                pages.push(
                                    ChildMenus { name: "Settings", path: "/settings" }
                                );
                                pages
                            }

                            key=|p| p.path
                            children=move |m| {
                                let is_active = create_memo(move |_| {
                                    use_route().child().map(|r| r.path()).unwrap_or_default()
                                        == format!("{}{}", use_route().path(), m.path)
                                });
                                view! {
                                    <A
                                        href=move || format!("{}{}", use_route().path(), m.path)
                                    >
                                        <span
                                        class="dark:hover:bg-white/5 hover:bg-black/5 p-3 rounded text-sm cursor-pointer text-slate-700 dark:text-white/65 block"
                                        class=(
                                            ["text-black", "dark:text-white", "font-medium","dark:bg-white/10","bg-black/10"],
                                            is_active,
                                        )>{m.name}</span>
                                    </A>
                                }
                            }
                        />

                    </div>

                    <div class="w-full">
                        <Outlet/>
                    </div>
                </div>
            </div>
        </Transition>
    }
}

#[component]
pub fn GeneralSettings() -> impl IntoView {
    let id = expect_context::<Uuid>();

    let project = expect_context::<Resource<(), Result<Project, ServerFnError>>>();

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
                        Some(ProjectType::PortForward(PortForward {
                            port: 3000,
                            #[cfg(feature = "ssr")]
                            peer: Arc::new(pingora::upstreams::peer::HttpPeer::new(
                                "0.0.0.0:3000",
                                false,
                                String::new(),
                            )),
                        }))
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
                            exposed_ports: vec![].into(),
                            #[cfg(feature = "ssr")]
                            status: crate::common::ContainerStatus::None,
                            tokens: HashMap::new(),
                        }))
                    }
                })
            }),
        ),
    ];

    let update_port_action = create_server_action::<UpdateProjectPort>();
    let update_image_action = create_server_action::<UpdateProjectImage>();

    let domains = create_resource(
        move || {},
        move |_| async move {
            let result = get_project_domains(id).await;
            let result = result.unwrap_or_default();
            result
        },
    );

    create_effect(move |_| {
        update_image_action.value().get();
        update_port_action.value().get();
        project.refetch();
    });

    create_effect(move |p| {
        let new_p = project.get();
        if new_p != p.and_then(|p| p) {
            set_edited_project_type.set(None);
        }
        new_p
    });

    view! {
        <Transition>
            <div>
                <div class="text-xl ">"Project Type"</div>
                <div class="h-2"></div>
                <div class="flex gap-3">

                    {move || {
                        project_types
                            .into_iter()
                            .map(|p| {
                                view! {
                                    <div
                                        class="p-2 text-sm rounded-md hover:bg-black/20 dark:hover:bg-white/20 cursor-pointer"
                                        class=(
                                            [
                                                "dark:text-white",
                                                "text-black",
                                                "bg-black/10",
                                                "dark:bg-white/30",
                                            ],
                                            p.1,
                                        )

                                        class=(
                                            ["text-black/60", "dark:text-white/60"],
                                            move || !p.1.get(),
                                        )

                                        on:click=move |_| { set_edited_project_type.set(p.2.get()) }
                                    >

                                        {p.0}
                                    </div>
                                }
                            })
                            .collect::<Vec<_>>()
                    }}

                </div>
                <div class="h-2"></div>

                {move || match edited_project_type.get().or(project_type.get()) {
                    Some(project_type) => {
                        match project_type {
                            ProjectType::PortForward(port) => {
                                view! {
                                    <ActionForm action=update_port_action>
                                        <div class="text-md">"Port"</div>
                                        <input
                                            name="id"
                                            type="hidden"
                                            prop:value=move || {
                                                project.get().and_then(|p| p.ok()).map(|p| p.id.to_string())
                                            }
                                        />

                                        <input
                                            name="port"
                                            prop:value=port.port
                                            type="number"
                                            class="border p-2 rounded-md dark:bg-white/10 dark:border-white/5"
                                        />
                                        <div class="h-2"></div>
                                        <input
                                            type="submit"
                                            value="Update"
                                            class="cursor-pointer block border p-2 px-10 rounded bg-slate-800 text-white disabled:cursor-no-drop disabled:bg-slate-200 disabled:text-black dark:disabled:bg-white/20 dark:disabled:text-white dark:border-none dark:bg-white/90 dark:text-black"
                                        />
                                    </ActionForm>
                                }
                                    .into_view()
                            }
                            ProjectType::Container(container) => {
                                let (exposed_ports, set_exposed_ports)= create_signal(container.exposed_ports.into_iter().map(|p|(random_ascii_string(8),p)).collect::<HashMap<String, ExposedPort>>());

                                view! {
                                    <ActionForm action=update_image_action>
                                        <input
                                            name="id"
                                            type="hidden"
                                            prop:value=move || {
                                                project.get().and_then(|p| p.ok()).map(|p| p.id.to_string())
                                            }
                                        />

                                        <div class="h-4"></div>

                                        <div class="text-md">"Exposed Port"</div>

                                        <div class="">
                                            <For
                                                each=move || exposed_ports.get().into_iter()
                                                key=|p| p.0.clone()
                                                children=move |(index, exposed_port)| {
                                                    let i2 = index.clone();
                                                    let i3 = index.clone();
                                                    let i4 = index.clone();
                                                    view! {
                                                        <div class="flex flex-col gap-4 p-2 border dark:border-white/20 m-2 rounded">
                                                            <div class="flex gap-4 flex-wrap">

                                                                <div class=" flex flex-col">
                                                                    <label for="port" class="text-sm dark:text-white/50">
                                                                        "Port"
                                                                    </label>
                                                                    <input
                                                                        prop:value=exposed_port.port
                                                                        type="number"
                                                                        id="port"
                                                                        name= format!("exposed_ports[{i2}][port]")
                                                                        required
                                                                        class="border p-2 rounded-md dark:bg-white/10 dark:border-white/5"
                                                                    />
                                                                </div>

                                                                <div class=" flex flex-col">


                                                                    <label for="domain" class="text-sm dark:text-white/50">
                                                                        "Domain"
                                                                    </label>

                                                                    <select
                                                                        name=format!("exposed_ports[{i3}][domains][1][name]")
                                                                        class="p-2 bg-white border rounded-md dark:bg-white/10 dark:border-white/5"
                                                                    >

                                                                        <option value="">"None"</option>

                                                                        {move || {
                                                                            domains
                                                                                .get()
                                                                                .unwrap_or_default()
                                                                                .iter()
                                                                                .map(|domain| {
                                                                                    view! {
                                                                                        <option
                                                                                            value=domain.0
                                                                                            selected=exposed_port.domains.iter().any(|d|d.name.to_lowercase() == domain.0.to_lowercase())
                                                                                        >
                                                                                            {domain.0}
                                                                                        </option>
                                                                                    }
                                                                                })
                                                                                .collect::<Vec<_>>()
                                                                        }}

                                                                    </select>
                                                                </div>

                                                                <button
                                                                    class="p-2 rounded bg-red-700 px-6 text-white mt-5"
                                                                    on:click=move |_| {
                                                                        let mut exposed_ports = exposed_ports.get_untracked();
                                                                        exposed_ports.remove(&i4);
                                                                        set_exposed_ports.set(exposed_ports)
                                                                    }
                                                                >

                                                                    "Remove Port"
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
                                                    let new_port = ExposedPort{
                                                        port: 0,
                                                        domains: vec![].into(),
                                                        #[cfg(feature = "ssr")]
                                                        peer: unimplemented!("Cant create new exposed port in ssr"),
                                                    };
                                                    let mut ports = exposed_ports.get_untracked();
                                                    ports.insert(random_ascii_string(8), new_port);
                                                    set_exposed_ports.set(ports);
                                                }
                                            >

                                                "Add New Exposed Port"
                                            </button>
                                        </div>
                                        <div class="h-4"></div>

                                        <input
                                            type="submit"
                                            value="Update"
                                            class="cursor-pointer block border p-2 px-10 rounded bg-slate-800 text-white disabled:cursor-no-drop disabled:bg-slate-200 disabled:text-black dark:disabled:bg-white/20 dark:disabled:text-white dark:border-none dark:bg-white/90 dark:text-black"
                                        />
                                    </ActionForm>
                                }
                                    .into_view()
                            }
                        }
                    }
                    None => view! {}.into_view(),
                }}

            </div>
        </Transition>
    }
}

#[component]
pub fn DomainsList() -> impl IntoView {
    let id = expect_context::<Uuid>();
    let add_domain_action = create_server_action::<AddProjectDomain>();

    let domains = create_resource(
        move || {},
        move |_| async move {
            let result = get_project_domains(id).await;
            let result = result.unwrap_or_default();

            // result.sort_by_key(|p| p.0.to_string());
            result
        },
    );

    let Pausable {
        ..
        // pause,
        // resume,
        // is_active,
    } = use_interval_fn(
        move || {
            domains.refetch();
        },
        5000,
    );

    let (new_domain, set_new_domain) = create_signal(String::new());

    create_effect(move |_| {
        add_domain_action.value().get();
        set_new_domain.set(String::new());
        domains.refetch();
    });

    view! {
        <div class="p-2 text-xl">"Domains"</div>

        <div class="p-2">
            <ActionForm action=add_domain_action>
                <div class="w-full rounded-md flex gap-5">
                    <input type="hidden" name="id" prop:value=id.to_string()/>
                    <input
                        name="domain"
                        id="domain"
                        placeholder="example.com"
                        class="p-2 border w-full rounded bg-white dark:bg-white/10 dark:border-white/5"
                        on:input=move |ev| {
                            set_new_domain.set(event_target_value(&ev));
                        }

                        prop:value=new_domain
                    />
                    <input
                        type="submit"
                        value="Add"
                        class="border p-2 px-10 rounded bg-slate-800 text-white disabled:cursor-no-drop disabled:bg-slate-200 disabled:text-black dark:disabled:bg-white/20 dark:disabled:text-white dark:border-none dark:bg-white/90 dark:text-black"
                        disabled=move || new_domain.get().is_empty()
                        prop:disabled=move || new_domain.get().is_empty()
                    />
                </div>
            </ActionForm>
        </div>

        <Transition>
            <For
                each=move || {
                    domains
                        .get()
                        .unwrap_or_default()
                        .keys()
                        .cloned()
                        .collect::<BinaryHeap<_>>()
                        .into_sorted_vec()
                }

                key=|domain| domain.clone()
                children=move |domain| {
                    let dc = domain.clone();
                    let status = create_memo(move |_| {
                        domains.get().unwrap_or_default().get(&dc).cloned()
                    });
                    view! {
                        <div class="p-2">
                            <div class="w-full border bg-white dark:bg-white/10 dark:border-white/20 rounded-md p-4">
                                <div class="text-xl flex items-center ">
                                    {domain}
                                    <span class="text-slate-600 dark:text-slate-300 text-sm px-4 py-1 bg-slate-200 dark:bg-slate-700 rounded-full w-fit ml-2 flex items-center justify-center">

                                        <div
                                            class="w-2 h-2 rounded-full mr-2 inline-block"
                                            class=(
                                                "bg-green-500",
                                                move || {
                                                    status
                                                        .get()
                                                        .map(|s| s.ssl_provision.is_provisioned())
                                                        .unwrap_or_default()
                                                },
                                            )

                                            class=(
                                                "bg-yellow-500",
                                                move || {
                                                    status
                                                        .get()
                                                        .map(|s| s.ssl_provision.is_not_provisioned())
                                                        .unwrap_or_default()
                                                },
                                            )
                                        >
                                        </div>

                                        {move || match status
                                            .get()
                                            .map(|s| s.ssl_provision)
                                            .unwrap_or(crate::common::SSLProvisioning::NotProvisioned)
                                        {
                                            crate::common::SSLProvisioning::NotProvisioned => "Waiting",
                                            crate::common::SSLProvisioning::Provisioning => "Processing",
                                            crate::common::SSLProvisioning::Provisioned(_) => "Active",
                                        }}

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
