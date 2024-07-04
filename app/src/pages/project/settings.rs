use leptos::{
    component, create_memo, expect_context, view, IntoView, Resource, ServerFnError, SignalGet,
    Transition,
};
use uuid::Uuid;

use crate::common::Project;

#[component]
pub fn ProjectSettings() -> impl IntoView {
    let id = expect_context::<Uuid>();

    let project = expect_context::<Resource<(), Result<Project, ServerFnError>>>();

    let project_type =
        create_memo(move |_| project.get().and_then(|p| p.ok()).map(|p| p.project_type));

    view! {
        <Transition>
            <div>
                <div class="text-xl ">"Project Name"</div>
                <input class="" prop:value=move || project.get().and_then(|p|p.ok()).map(|p|p.name.to_string()) />
                <div class="h-2"></div>
            </div>

        </Transition>
    }
}
