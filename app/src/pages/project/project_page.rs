use std::collections::BinaryHeap;

use leptos::create_memo;
use leptos::create_server_action;
use leptos::For;
use leptos::Params;
use leptos::SignalGet;
use leptos::SignalWith;
use leptos::Transition;
use leptos::{component, create_resource, view, IntoView};
use leptos_router::use_params;
use leptos_router::ActionForm;
use leptos_router::Params;
use leptos_use::use_interval_fn;
use leptos_use::utils::Pausable;
use uuid::Uuid;

use crate::api::get_project;
use crate::api::get_project_domains;
use crate::api::AddProjectDomain;
use leptos_router::Redirect;

#[derive(Params, PartialEq)]
struct ProjectParams {
    id: Uuid,
}

#[component]
pub fn ProjectPage() -> impl IntoView {
    let params = use_params::<ProjectParams>();

    let id =
        move || params.with(|params| params.as_ref().map(|param| param.id).unwrap_or_default());
    let add_domain_action = create_server_action::<AddProjectDomain>();

    let project = create_resource(
        move || {
            add_domain_action.version().get();
            id()
        },
        move |id| async move {
            let result = get_project(id).await;
            result
        },
    );

    let domains = create_resource(id, move |id| async move {
        let result = get_project_domains(id).await;
        let mut result = result.unwrap_or_default();

        // result.sort_by_key(|p| p.0.to_string());
        result
    });

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

    view! {

        <Transition>
            <div class="p-4">
            {
                move || project.get().map(
                    |p| if let Ok(p) = p {
                        view! {

                            <h1 class="text-4xl"> { &p.name } </h1>
                            <div class="text-slate-600 text-sm"> {p.id.to_string()} </div>

                        }.into_view()
                    }else {
                        view! {
                            <Redirect path="../" />
                        }
                    }
                )
            }
            <hr class="my-2" />

            <div class="p-2 text-xl" > "Domains" </div>

            <For
                each= move || {domains.get().unwrap_or_default().keys().cloned().collect::<BinaryHeap<_>>().into_sorted_vec()}

                key=|domain| domain.clone()
                children=move |domain| {
                    let dc = domain.clone();
                    let status = create_memo( move |_| domains.get().unwrap_or_default().get(&dc).cloned());

                    view! {
                        <div class="p-2">
                            <div class="w-full shadow-md rounded-md p-4">
                                <div class="text-xl flex items-center ">  {domain}
                                <span class="text-slate-600 text-sm p-3 bg-slate-200 rounded-full w-fit ml-2 flex items-center justify-center">

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

            <div class="p-2">
                <ActionForm action=add_domain_action>
                    <div class="w-full shadow-md rounded-md p-4 flex flex-col">
                        <label for="domain" class="text-sm p-2"> "Domain" </label>
                        <input type="hidden" name="id" prop:value=move|| id().to_string() />
                        <input name="domain" id="domain" placeholder="example.com" class="p-2 border" />
                        <input type="submit" value="Add Domain" class="border p-2"/>
                    </div>
                </ActionForm>
            </div>

            </div>
        </Transition>
    }
}
