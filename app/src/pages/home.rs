use leptos::create_effect;
use leptos::create_server_action;
use leptos::expect_context;
use leptos::Action;
use leptos::Resource;
use leptos::ServerFnError;
use leptos::SignalGet;
use leptos::Suspense;
use leptos::{component, create_resource, view, IntoView};
use leptos_router::ActionForm;

use crate::api::get_projects;
use crate::auth::AuthType;
use crate::auth::Login;

/// Renders the home page of your application.
#[component]
pub fn HomePage(login: Action<Login, Result<(), ServerFnError>>) -> impl IntoView {
    let auth = expect_context::<Resource<(), AuthType>>();

    create_effect(move |_| {
        let value = login.value().get();
        if let Some(Ok(_)) = value {
            auth.refetch();
        }
    });

    view! {
        <div class="w-full min-h-full h-full flex items-center justify-center flex-col">
            <ActionForm
                action=login
                class="p-4 rounded shadow-lg flex flex-col bg-white dark:bg-white/15 dark:shadow-white/25"
            >
                <input
                    name="email"
                    class="border-solid border-0 border-b p-2 text-lg bg-transparent"
                    placeholder="Email"
                    type="email"
                />
                <div class="h-2"></div>
                <input
                    name="password"
                    class="border-solid border-0 border-b p-2 text-lg bg-transparent"
                    placeholder="Password"
                    type="password"
                />

                <div class="h-6"></div>
                <input type="submit" value="Login" class="border p-2"/>
            </ActionForm>
        </div>
    }
}
