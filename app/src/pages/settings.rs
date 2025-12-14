use crate::components::nav_bar::NavBar;
use crate::AuthCheck;
use leptos::prelude::*;

use crate::common::UpdateStatus;
use crate::updates::{check_update, perform_update};

#[component]
pub fn Settings() -> impl IntoView {
    let check_update_action = Action::new(|_| check_update());
    let perform_update_action = Action::new(|_| perform_update());

    let update_status = check_update_action.value();
    let perform_status = perform_update_action.value();

    view! {
        <AuthCheck is_auth_required=true/>
        <NavBar/>
        <div class="p-4 max-w-4xl mx-auto text-slate-900 dark:text-slate-100">
            <h1 class="text-2xl font-bold mb-4">"Settings"</h1>

            <div class="bg-white dark:bg-zinc-900 rounded-lg p-6 shadow">
                <h2 class="text-xl font-semibold mb-4">"System Update"</h2>

                <div class="mb-4">
                     <button
                        class="bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded"
                        on:click=move |_| { check_update_action.dispatch(()); }
                    >
                        "Check for Updates"
                    </button>

                    {move || {
                        if check_update_action.pending().get() {
                           view! { <span class="ml-2">"Checking..."</span> }.into_any()
                        } else {
                            view! { }.into_any()
                        }
                    }}
                </div>

                {move || {
                    update_status.get().map(|result| {
                        match result {
                            Ok(status) => {
                                // We need to cast the opaque return type if we want field access in template,
                                // but server function returns specific struct.
                                // In Leptos isomorphism, shared types should be in shared crate.
                                // But UpdateStatus depends on env! which is fine.
                                // Wait, UpdateStatus is defined in server crate. App crate can't see it unless we re-export or move it.
                                // ISSUE: app cannot depend on server. defaults are usually app -> logic.
                                // We should move the struct UpdateStatus to `app/src/common.rs` or similar so both can see it.
                                // For now, let's assume we fix that.
                                view! {
                                    <div class="space-y-2">
                                        <p>"Current Version: " <span class="font-mono">{status.current_build_time}</span> " (" {status.current_git_hash} ")" </p>
                                        <p>"Latest Version: " <span class="font-mono">{status.remote_build_time}</span> " (" {status.remote_git_hash} ")" </p>

                                        {if status.update_available {
                                            view! {
                                                <div class="mt-4 p-4 bg-yellow-100 dark:bg-yellow-900/30 rounded border border-yellow-200 dark:border-yellow-800">
                                                    <p class="font-bold text-yellow-800 dark:text-yellow-200 mb-2">"Update Available!"</p>
                                                    <button
                                                        class="bg-green-600 hover:bg-green-700 text-white px-4 py-2 rounded disabled:opacity-50"
                                                        disabled=move || perform_update_action.pending().get()
                                                        on:click=move |_| { perform_update_action.dispatch(()); }
                                                    >
                                                        {move || if perform_update_action.pending().get() { "Updating..." } else { "Update Now" }}
                                                    </button>
                                                </div>
                                            }.into_any()
                                        } else {
                                            view! {
                                                <p class="text-green-600 dark:text-green-400">"System is up to date."</p>
                                            }.into_any()
                                        }}
                                    </div>
                                }.into_any()
                            }
                            Err(e) => view! { <div class="text-red-500">{e.to_string()}</div> }.into_any()
                        }
                    })
                }}

                 {move || {
                    perform_status.get().map(|result| {
                        match result {
                            Ok(msg) => view! { <div class="mt-4 text-green-600 font-semibold">{msg}</div> }.into_any(),
                            Err(e) => view! { <div class="mt-4 text-red-500">{e.to_string()}</div> }.into_any(),
                        }
                    })
                }}

            </div>
        </div>
    }
}
