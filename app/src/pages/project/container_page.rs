use leptos::{
    create_effect, create_resource, create_server_action, expect_context, prelude::*, view,
    IntoView, Transition,
};
use leptos_use::{use_interval_fn, utils::Pausable};
use uuid::Uuid;

use crate::api::inspect_container;

pub fn ContainerPage() -> impl IntoView {
    let id = expect_context::<Memo<Uuid>>();

    let container = create_resource(
        move || id.get(),
        move |id| async move {
            let result = inspect_container(id).await;
            let mut result = result.ok();
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
                view! {
                    <div> Running { move || container.get().and_then(|p|p).is_some()} </div>
                }
           }

        </Transition>
    }
}
