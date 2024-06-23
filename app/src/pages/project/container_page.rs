use leptos::{
    create_effect, create_resource, create_server_action, expect_context, prelude::*, view,
    IntoView, Transition,
};
use leptos_use::{use_interval_fn, utils::Pausable};
use tracing::info;
use uuid::Uuid;

use crate::api::inspect_container;

pub fn ContainerPage() -> impl IntoView {
    let id = expect_context::<Uuid>();

    let container = create_resource(
        move || id,
        move |id| async move {
            let result = inspect_container(id).await;

            result
        },
    );

    let Pausable {
        pause,
        resume,
        is_active,
    } = use_interval_fn(
        move || {
            container.refetch();
        },
        5000,
    );

    view! {
        <div class="p-2 text-xl">"Container"</div>

        <Transition>
           {
                let container = create_memo(move |_| container.get());
                let is_running = create_memo(move |_| container.get().map(|r|r.is_ok()).unwrap_or(false));
                move || if is_running.get() {
                    let container = move || container.get().unwrap().unwrap();
                    view! {
                        <div class="p-2 flex gap-2 items-center">
                            <div class="text-lg "> "Status" </div>
                            <div class="text-sm px-6 py-1 rounded-full bg-slate-400 text-black"> {
                                move || {
                                    container()
                                            .state.and_then(|s|s.status).unwrap_or("Unknown".to_string())
                                }
                            } </div>
                        </div>
                    }.into_view()

                }else {
                    view! {
                        <div>
                            "Failed to load container status"
                        </div>
                    }.into_view()
                }

           }

        </Transition>
    }
}
