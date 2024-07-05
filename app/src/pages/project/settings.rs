use leptos::{
    component, create_effect, create_memo, create_node_ref, create_server_action, expect_context,
    view, IntoView, Resource, ServerFnError, SignalGet, Transition,
};
use leptos_toaster::{Toast, ToastId, ToastVariant, Toasts};
use tracing::info;
use uuid::Uuid;

use crate::api::UpdateProjectNameToken;
use crate::common::ProjectType;
use crate::{common::Project, components::input_field::InputField};
use leptos::DynAttrs;
use leptos_router::ActionForm;

use crate::api::DeleteProject;
use crate::common::Token;
use crate::utils::random_ascii_string;
use leptos::SignalGetUntracked;
use leptos::SignalSet;
use leptos::{create_signal, For};

#[component]
pub fn ProjectSettings() -> impl IntoView {
    let id = expect_context::<Uuid>();

    let project = expect_context::<Resource<(), Result<Project, ServerFnError>>>();

    let project_type =
        create_memo(move |_| project.get().and_then(|p| p.ok()).map(|p| p.project_type));

    let update_project_action = create_server_action::<UpdateProjectNameToken>();
    let toast_context = expect_context::<Toasts>();

    create_effect(move |_| {
        if update_project_action.version().get() > 0 {
            let toast_id = ToastId::new();
            toast_context.toast(
                view! {
                    <Toast
                        toast_id
                        variant=ToastVariant::Success
                        title=view! { "Update Success" }.into_view()
                    />
                },
                Some(toast_id),
                None,
            );
            project.refetch();
        }
    });

    let confirm_delete_dialog = create_node_ref::<leptos::html::Dialog>();

    let delete_project_action = create_server_action::<DeleteProject>();
    let navigate = leptos_router::use_navigate();

    create_effect(move |_| {
        if delete_project_action.version().get() > 0 {
            let toast_id = ToastId::new();
            toast_context.toast(
                view! {
                    <Toast
                        toast_id
                        variant=ToastVariant::Success
                        title=view! { "Project Deleted" }.into_view()
                    />
                },
                Some(toast_id),
                None,
            );
            navigate("/", Default::default());
        }
    });

    view! {
        <Transition>
            <ActionForm action=update_project_action>
                <input type="hidden" name="id" value=id.to_string() />
                <div class="text-xl " class=("abc", move || true)>
                    "Project Name"
                </div>
                <input
                    class="p-2 border w-full rounded bg-white dark:bg-white/10 dark:border-white/5"
                    name="project_name"
                    prop:value=move || project.get().and_then(|p| p.ok()).map(|p| p.name)
                />
                <div class="h-2"></div>

                {move || match project_type.get() {
                    Some(project_type) => {
                        match project_type {
                            ProjectType::PortForward(_) => view! {}.into_view(),
                            ProjectType::Container(container) => {
                                let (tokens, set_tokens) = create_signal(container.tokens);
                                view! {
                                    <div class="text-md">"Tokens"</div>
                                    <div class="">
                                        <For
                                            each=move || tokens.get().into_iter()
                                            key=|p| p.0.clone()
                                            children=move |(index, token)| {
                                                let token_id = index.clone();
                                                view! {
                                                    <div class="flex flex-col gap-4 p-2 border dark:border-white/20 m-2 rounded">
                                                        <div class=" flex flex-col">
                                                            <label for="token" class="text-sm dark:text-white/50">
                                                                Token
                                                            </label>
                                                            <input
                                                                prop:value=&token.token
                                                                type="hidden"
                                                                name=format!("tokens[{index}][token]")
                                                                required
                                                                class="border p-2 rounded-md dark:bg-white/10 dark:border-white/5"
                                                            />
                                                            <input
                                                                prop:value=&token.token
                                                                disabled
                                                                type="text"
                                                                id="token"
                                                                required
                                                                class="border p-2 rounded-md dark:bg-white/10 dark:border-white/5"
                                                            />
                                                        </div>
                                                        <div class="flex gap-4 flex-wrap">

                                                            <div class=" flex flex-col">
                                                                <label for="description" class="text-sm dark:text-white/50">
                                                                    Description
                                                                </label>
                                                                <input
                                                                    prop:value=&token.description
                                                                    type="text"
                                                                    id="description"
                                                                    name=format!("tokens[{index}][description]")
                                                                    required
                                                                    class="border p-2 rounded-md dark:bg-white/10 dark:border-white/5"
                                                                />
                                                            </div>

                                                            <div class=" flex flex-col">
                                                                <label for="expiry" class="text-sm dark:text-white/50">
                                                                    Expiry
                                                                </label>
                                                                <input
                                                                    type="date"
                                                                    prop:value=token
                                                                        .expiry
                                                                        .map(|e| e.to_string())
                                                                        .unwrap_or_default()
                                                                    id="expiry"
                                                                    name=format!("tokens[{index}][expiry]")
                                                                    class="border p-2 rounded-md dark:bg-white/10 dark:border-white/5"
                                                                />
                                                            </div>

                                                            <button
                                                                class="p-2 rounded bg-red-700 px-6 text-white mt-5"
                                                                on:click=move |_| {
                                                                    let mut tokens = tokens.get_untracked();
                                                                    tokens.remove(&token_id);
                                                                    set_tokens.set(tokens)
                                                                }
                                                            >

                                                                "Delete Token"
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
                                                let new_token = Token {
                                                    expiry: None,
                                                    description: String::new(),
                                                    token: random_ascii_string(20),
                                                };
                                                let mut tokens = tokens.get_untracked();
                                                tokens.insert(new_token.token.clone(), new_token);
                                                set_tokens.set(tokens);
                                            }
                                        >

                                            "Generate new Token"
                                        </button>
                                    </div>
                                }
                                    .into_view()
                            }
                        }
                    }
                    None => view! {}.into_view(),
                }}

                <div class="h-4" />
                <input
                    type="submit"
                    value="Update"
                    class="border p-2 px-10 rounded bg-slate-800 text-white disabled:cursor-no-drop disabled:bg-slate-200 disabled:text-black dark:disabled:bg-white/20 dark:disabled:text-white dark:border-none dark:bg-white/90 dark:text-black"
                />

            </ActionForm>

            <button
                class="p-2 rounded bg-red-700 px-6 text-white mt-5"
                on:click=move |_| {
                    if let Some(dialog) = confirm_delete_dialog.get_untracked(){
                        _ = dialog.show_modal();
                    }
                }
            >

                "Delete Project"
            </button>

            <dialog _ref=confirm_delete_dialog
                class="backdrop:bg-black/50 dark:backdrop:bg-white/40 bg-white dark:bg-black text-black dark:text-white p-4 rounded-md"
            >
                <div>
                    "Are you sure you want to delete Project"
                </div>

                <div class="flex gap-2">
                    <button
                        class="p-2 rounded bg-red-700 px-6 text-white mt-5 disabled:bg-slate-700"
                        disabled=delete_project_action.pending()
                        on:click=move |_| {
                            delete_project_action.dispatch(DeleteProject{id});
                        }
                    >
                        "Confirm Delete"
                    </button>

                    <button
                        class="p-2 rounded border px-6 mt-5"
                        on:click=move |_| {
                            if let Some(dialog) = confirm_delete_dialog.get_untracked(){
                                _ = dialog.close();
                            }
                        }
                    >
                        "No, Cancel Delete"
                    </button>
                </div>
            </dialog>
        </Transition>
    }
}
