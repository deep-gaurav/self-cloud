use leptos::{
    create_action, create_effect, create_resource, create_server_action, expect_context,
    prelude::*, view, IntoView, Transition,
};
use leptos_use::{use_interval_fn, utils::Pausable};
use tracing::info;
use uuid::Uuid;

use crate::api::{
    inspect_container, PauseContainer, ResumeContainer, StartContainer, StopContainer,
};

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

    let pause_container_action = create_server_action::<PauseContainer>();
    create_effect(move |_| {
        if pause_container_action.version().get() > 0 {
            container.refetch();
        }
    });

    let resume_container_action = create_server_action::<ResumeContainer>();
    create_effect(move |_| {
        if resume_container_action.version().get() > 0 {
            container.refetch();
        }
    });

    let stop_container_action = create_server_action::<StopContainer>();
    create_effect(move |_| {
        if stop_container_action.version().get() > 0 {
            container.refetch();
        }
    });

    let start_container_action = create_server_action::<StartContainer>();
    create_effect(move |_| {
        if start_container_action.version().get() > 0 {
            container.refetch();
        }
    });

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

                            <div class="flex-grow w-full" />
                            <div class="flex gap-2">
                                {
                                    move || {
                                        if container().state.and_then(|state|state.running) == Some(true) {
                                            view! {
                                                {
                                                    if container().state.and_then(|state|state.paused) == Some(true) {
                                                        view! {
                                                            <button
                                                                class="p-2 rounded bg-green-700 px-6 text-white"

                                                                on:click=move|_|{
                                                                    resume_container_action.dispatch(ResumeContainer{
                                                                        id,
                                                                    });
                                                                }
                                                            > "Resume" </button>
                                                        }
                                                    }else {
                                                        view! {
                                                            <button
                                                                class="p-2 rounded bg-yellow-700 px-6 text-white"
                                                                on:click=move|_|{
                                                                    pause_container_action.dispatch(PauseContainer{
                                                                        id,
                                                                    });
                                                                }
                                                            > "Pause" </button>
                                                        }
                                                    }
                                                }
                                                <button
                                                    class="p-2 rounded bg-red-700 px-6 text-white"
                                                    on:click=move|_|{
                                                        stop_container_action.dispatch(StopContainer{
                                                            id,
                                                        });
                                                    }
                                                > "Stop" </button>
                                            }.into_view()
                                        } else {
                                            view! {
                                                <button
                                                    class="p-2 rounded bg-green-700 px-6 text-white"
                                                    on:click=move|_|{
                                                        start_container_action.dispatch(StartContainer{
                                                            id,
                                                        });
                                                    }
                                                > "Start" </button>
                                            }.into_view()
                                        }
                                    }
                                }
                            </div>
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
