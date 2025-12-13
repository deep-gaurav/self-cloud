use leptos::prelude::*;
// use leptos_router::components::ActionForm;
// use leptos_router::hooks::use_query_map;
use crate::api::get_server_version;
use crate::auth::AuthType;
use crate::auth::Login;

/// Renders the home page of your application.
#[component]
pub fn HomePage(login: ServerAction<Login>) -> impl IntoView {
    let auth = expect_context::<Resource<Result<AuthType, ServerFnError>>>();

    Effect::new(move |_| {
        let value = login.value().get();
        if let Some(Ok(_)) = value {
            auth.refetch();
        }
    });

    let version = Resource::new(|| (), move |_| get_server_version());
    let login = ServerAction::<Login>::new();
    let (email, set_email) = signal(String::new());
    let (password, set_password) = signal(String::new());
    view! {
        <div class="w-full h-full flex items-center justify-center flex-col flex-grow 1">
            <ActionForm action=login
                attr:class="p-4 rounded shadow-lg flex flex-col bg-white dark:bg-white/15 dark:shadow-white/25"
            >
                <input
                    name="email"
                    class="border-solid border-0 border-b p-2 text-lg bg-transparent"
                    placeholder="Email"
                    type="email"
                    prop:value=email
                    on:input=move |ev| set_email.set(event_target_value(&ev))
                />
                <div class="h-2"></div>
                <input
                    name="password"
                    class="border-solid border-0 border-b p-2 text-lg bg-transparent"
                    placeholder="Password"
                    type="password"
                    prop:value=password
                    on:input=move |ev| set_password.set(event_target_value(&ev))
                />

                <div class="h-6"></div>
                <input type="submit" value="Login" class="border p-2"/>
            </ActionForm>

            <div class="h-2"/>
            <div class="text-xs text-slate-600">
            <Suspense>
                {move || if let Some(Ok(version)) = version.get() {
                   version
                } else {
                    "Unknown version".to_string()
                }}
            </Suspense>
            </div>
        </div>
    }
}
