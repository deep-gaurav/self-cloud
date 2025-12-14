use std::collections::BinaryHeap;
use std::sync::Arc;

use crate::components::toaster::{ToastVariant, ToasterContext};
use leptos::either::Either;
use leptos::prelude::*;
use leptos::server_fn::ServerFn;
use leptos_router::components::{Form, Outlet, A};
use leptos_router::hooks::{use_location, use_params};
use leptos_router::params::Params;
use leptos_use::use_interval_fn;
use leptos_use::utils::Pausable;
use std::collections::HashMap;
use uuid::Uuid;

use crate::api::get_project;
use crate::api::get_project_domains;
use crate::api::AddProjectDomain;
use crate::api::UpdateProjectImage;
use crate::api::UpdateProjectPort;
use crate::common::Container;
use crate::common::EnvironmentVar;
use crate::common::ExposedPort;
use crate::common::PortForward;
use crate::common::Project;
use crate::common::ProjectType;

#[derive(Params, PartialEq, Clone, Debug, Copy)]
struct ProjectParams {
    id: Option<Uuid>,
}

#[component]
pub fn ProjectPage() -> impl IntoView {
    let params = use_params::<ProjectParams>();
    let location = use_location();

    let id = Signal::derive(move || params.with(|p| p.as_ref().unwrap().id.unwrap()));
    let (trigger, set_trigger) = signal(());
    let project = Resource::new(
        move || trigger.get(),
        move |_| async move {
            let res: Result<Project, ServerFnError> = get_project(id.get()).await;
            res
        },
    );

    #[derive(Clone, Copy, PartialEq)]
    struct ChildMenus<'a> {
        name: &'a str,
        path: &'a str,
    }

    provide_context(project);
    provide_context(set_trigger);
    provide_context(id);

    view! {
            <div class="p-4">
                <Transition>
                {move || Suspend::new(
                    async move {
                        let project =  project.await;
                        match project {
                            Ok(project) => {
                                Either::Left(view! {
                                    <h1 class="text-4xl">{project.name}</h1>
                                    <div class="text-slate-600 dark:text-slate-400 text-sm">
                                        {project.id.to_string()}
                                    </div>
                                })
                            },
                            Err(e) => {
                                Either::Right(view! {
                                    <h1 class="text-4xl">{e.to_string()}</h1>
                                })
                            },
                        }

                    }
                )}



                <hr class="my-2"/> <div class="flex flex-col gap-5 sm:flex-row">
                    <div class="w-40 flex flex-row sm:flex-col">

                        <For
                            each=move || {
                                let location = use_location();
                                let path = Memo::new(move |_| location.pathname.get());

                                let _is_support_containers_page = location.pathname.get().ends_with("/support-containers");
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
                                    pages
                                        .push(ChildMenus {
                                            name: "Services",
                                            path: "/services",
                                        });
                                }
                                pages
                                    .push(ChildMenus {
                                        name: "Settings",
                                        path: "/settings",
                                    });
                                pages
                            }

                            key=|p| p.path
                            children=move |m| {
                                let target_path = format!("/projects/{}{}", id.get(), m.path);
                                let target_path_memo = target_path.clone();
                                let is_active = Memo::new(move |_| {
                                    location.pathname.get() == target_path_memo
                                });
                                view! {
                                    <A href=target_path.clone()>
                                        <span
                                            class="dark:hover:bg-white/5 hover:bg-black/5 p-3 rounded text-sm cursor-pointer text-slate-700 dark:text-white/65 block"
                                            class=(
                                                [
                                                    "text-black",
                                                    "dark:text-white",
                                                    "font-medium",
                                                    "dark:bg-white/10",
                                                    "bg-black/10",
                                                ],
                                                is_active,
                                            )
                                        >
                                            {m.name}
                                        </span>
                                    </A>
                                }
                            }
                        />

                    </div>

                    <div class="w-full">
                        <Outlet/>
                    </div>

                </div>
                                </Transition>

            </div>
    }
}

#[component]
pub fn GeneralSettings() -> impl IntoView {
    let id = expect_context::<Signal<Uuid>>();

    let project = expect_context::<Resource<Result<Project, ServerFnError>>>();

    let project_type = Memo::new(move |_| match project.get() {
        Some(Ok(p)) => Some(p.project_type),
        _ => None,
    });

    let (edited_project_type, set_edited_project_type) = signal(Option::<ProjectType>::None);

    let project_types = [
        (
            "PortForward",
            Memo::new(move |_| {
                edited_project_type
                    .get()
                    .or(project_type.get())
                    .map(|p| p.is_port_forward())
                    .unwrap_or_default()
            }),
            Memo::new(move |_| {
                project_type.get().and_then(|p: ProjectType| {
                    if p.is_port_forward() {
                        None
                    } else {
                        Some(ProjectType::PortForward(PortForward {
                            port: 3000,
                            #[cfg(feature = "ssr")]
                            peer: Arc::new({
                                let mut peer = pingora::upstreams::peer::HttpPeer::new(
                                    "127.0.0.1:3000",
                                    false,
                                    String::new(),
                                );
                                peer.options.alpn = pingora::protocols::ALPN::H2H1;
                                peer
                            }),
                        }))
                    }
                })
            }),
        ),
        (
            "Container",
            Memo::new(move |_| {
                edited_project_type
                    .get()
                    .or(project_type.get())
                    .map(|p| p.is_container())
                    .unwrap_or_default()
            }),
            Memo::new(move |_| {
                project_type.get().and_then(|p: ProjectType| {
                    if p.is_container() {
                        None
                    } else {
                        Some(ProjectType::Container {
                            exposed_ports: vec![].into(),
                            support_containers: HashMap::new(),
                            tokens: HashMap::new(),

                            primary_container: Container {
                                #[cfg(feature = "ssr")]
                                status: crate::common::ContainerStatus::None,
                                env_vars: vec![].into(),
                            },
                        })
                    }
                })
            }),
        ),
    ];

    let update_port_action = ServerAction::<UpdateProjectPort>::new();
    let update_image_action = ServerAction::<UpdateProjectImage>::new();

    let domains = Resource::new(
        move || {},
        move |_| async move {
            let result = get_project_domains(id.get()).await;
            let result = result.unwrap_or_default();
            result
        },
    );
    let toast_context = expect_context::<ToasterContext>();
    let set_trigger = expect_context::<WriteSignal<()>>();

    Effect::new(move |_| {
        if update_image_action.version().get() > 0 || update_port_action.version().get() > 0 {
            toast_context.toast("Project Updated", ToastVariant::Success);
            set_trigger.set(());
        }
    });

    Effect::new(move |_| {
        let new_p = project.get();
        // Simplified Logic: Just update if new_p is invalid or changed.
        // For now, removing the sensitive "did logic change?" check to pass compilation.
        if new_p.is_none() {
            set_edited_project_type.set(None);
        }
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
                                    <Form action=UpdateProjectPort::url()>
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
                                    </Form>
                                }
                                    .into_any()
                            }
                            ProjectType::Container {
                                primary_container: container,
                                exposed_ports,
                                tokens,
                                ..
                            } => {
                                let (exposed_ports, set_exposed_ports) = signal({
                                    let mut map = vec![];
                                    for port in exposed_ports.into_iter() {
                                        map.push((map.len(), port));
                                    }
                                    map
                                });
                                let (env_vars, set_env_vars) = signal({
                                    let mut map = vec![];
                                    for env_var in container.env_vars.into_iter() {
                                        map.push((map.len(), env_var))
                                    }
                                    map
                                });
                                view! {
                                    <Form action=UpdateProjectImage::url()>
                                        <input
                                            name="id"
                                            type="hidden"
                                            prop:value=move || {
                                                project.get().and_then(|p| p.ok()).map(|p| p.id.to_string())
                                            }
                                        />

                                        // Exposed Port
                                        <div class="h-4"></div>

                                        <div class="text-md">"Exposed Port"</div>

                                        <div class="">
                                            <For
                                                each=move || exposed_ports.get().into_iter()
                                                key=|p| p.0
                                                children=move |(index, exposed_port)| {
                                                    view! {
                                                        <div class="flex flex-col gap-4 p-2 border dark:border-white/20 m-2 rounded">
                                                            <div class="flex gap-4 flex-wrap">

                                                                <div class=" flex flex-col">
                                                                    <label for="port" class="text-sm dark:text-white/50">
                                                                        "Port"
                                                                    </label>

                                                                    <div class="flex gap-2">
                                                                        <input
                                                                            prop:value=exposed_port.port
                                                                            type="number"
                                                                            id="port"
                                                                            name=format!("exposed_ports[{index}][port]")
                                                                            required
                                                                            class="border p-2 rounded-md dark:bg-white/10 dark:border-white/5"
                                                                        />


                                                                        <input
                                                                            name=format!("exposed_ports[{index}][host_port]")
                                                                            id="host_port"
                                                                            prop:value=exposed_port.host_port
                                                                            type="number"
                                                                            class="border p-2 rounded-md dark:bg-white/10 dark:border-white/5"
                                                                        />
                                                                    </div>
                                                                </div>

                                                                <div class=" flex flex-col">

                                                                    <label for="domain" class="text-sm dark:text-white/50">
                                                                        "Domain"
                                                                    </label>

                                                                    <select
                                                                        name=format!("exposed_ports[{index}][domains][1][name]")
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
                                                                                            value=domain.0.clone()
                                                                                            selected=exposed_port
                                                                                                .domains
                                                                                                .iter()
                                                                                                .any(|d| d.name.to_lowercase() == domain.0.to_lowercase())
                                                                                        >
                                                                                            {domain.0.clone()}
                                                                                        </option>
                                                                                    }
                                                                                })
                                                                                .collect::<Vec<_>>()
                                                                        }}

                                                                    </select>
                                                                </div>

                                                                 <button
                                                                    type="button"
                                                                    class="p-2 rounded bg-red-700 px-6 text-white mt-5"
                                                                    on:click=move |_| {
                                                                        let mut exposed_ports = exposed_ports.get_untracked();
                                                                        exposed_ports.remove(index);
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
                                                    let new_port = ExposedPort {
                                                        port: 0,
                                                        host_port: None,
                                                        domains: vec![].into(),
                                                        #[cfg(feature = "ssr")]
                                                        peer: unimplemented!("Cant create new exposed port in ssr"),
                                                    };
                                                    let mut ports = exposed_ports.get_untracked();
                                                    ports
                                                        .push((
                                                            ports.last().map(|p| p.0).unwrap_or_default() + 1,
                                                            new_port,
                                                        ));
                                                    set_exposed_ports.set(ports);
                                                }
                                            >

                                                "Add New Exposed Port"
                                            </button>
                                        </div>

                                        // EnvironmentVar

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
                                                                        name=format!("env_vars[{}][key]", index)
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
                                                                        name=format!("env_vars[{}][val]", index)
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

                                        <div class="h-4"></div>
                                        <input
                                            type="submit"
                                            value="Update"
                                            class="cursor-pointer block border p-2 px-10 rounded bg-slate-800 text-white disabled:cursor-no-drop disabled:bg-slate-200 disabled:text-black dark:disabled:bg-white/20 dark:disabled:text-white dark:border-none dark:bg-white/90 dark:text-black"
                                        />
                                    </Form>
                                }
                                    .into_any()
                            }
                        }.into_any()
                    }
                    None => view! {}.into_any(),
                }}

            </div>
        </Transition>
    }
}

#[component]
pub fn DomainsList() -> impl IntoView {
    let id = expect_context::<Signal<Uuid>>();
    let add_domain_action = ServerAction::<AddProjectDomain>::new();

    let domains = Resource::new(
        move || {},
        move |_| async move {
            let result = get_project_domains(id.get()).await;
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

    let (new_domain, set_new_domain) = signal(String::new());

    Effect::new(move |_| {
        add_domain_action.value().get();
        set_new_domain.set(String::new());
        domains.refetch();
    });

    view! {
        <div class="p-2 text-xl">"Domains"</div>

        <div class="p-2">
            <Form action=AddProjectDomain::url()>
                <div class="w-full rounded-md flex gap-5">
                    <input type="hidden" name="id" prop:value=move || id.get().to_string()/>
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
            </Form>
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
                    let status = Memo::new(move |_| {
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
