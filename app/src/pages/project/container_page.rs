use leptos::{
    component, create_action, create_effect, create_resource, create_server_action, expect_context,
    prelude::*, view, IntoView, Transition,
};
use leptos_chartistry::IntoInner;
use leptos_chartistry::{
    AspectRatio, AxisMarker, Chart, Legend, Line, RotatedLabel, Series, TickLabels, Tooltip,
    XGridLine, XGuideLine, YGridLine, YGuideLine,
};
use leptos_sse::create_sse_signal;
use leptos_use::{use_interval_fn, utils::Pausable};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::info;
use uuid::Uuid;

use crate::api::{
    inspect_container, PauseContainer, ResumeContainer, StartContainer, StopContainer,
};

#[component]
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
                let is_running = create_memo(move |_| {
                    container.get().map(|r| r.is_ok()).unwrap_or(false)
                });
                move || {
                    if is_running.get() {
                        let container = move || container.get().unwrap().unwrap();
                        view! {
                            <div class="p-2 flex gap-2 items-center flex-wrap">
                                <div class="text-lg ">"Status"</div>
                                <div class="text-sm px-6 py-1 rounded-full bg-slate-400 text-black">
                                    {move || {
                                        container()
                                            .state
                                            .and_then(|s| s.status)
                                            .unwrap_or("Unknown".to_string())
                                    }}

                                </div>

                                <div class="flex-grow w-full"></div>
                                <div class="flex gap-2">

                                    {move || {
                                        if container().state.and_then(|state| state.running)
                                            == Some(true)
                                        {
                                            view! {
                                                {if container().state.and_then(|state| state.paused)
                                                    == Some(true)
                                                {
                                                    view! {
                                                        <button
                                                            class="p-2 rounded bg-green-700 px-6 text-white"
                                                            on:click=move |_| {
                                                                resume_container_action.dispatch(ResumeContainer { id });
                                                            }
                                                        >

                                                            "Resume"
                                                        </button>
                                                    }
                                                } else {
                                                    view! {
                                                        <button
                                                            class="p-2 rounded bg-yellow-700 px-6 text-white"
                                                            on:click=move |_| {
                                                                pause_container_action.dispatch(PauseContainer { id });
                                                            }
                                                        >

                                                            "Pause"
                                                        </button>
                                                    }
                                                }}

                                                <button
                                                    class="p-2 rounded bg-red-700 px-6 text-white"
                                                    on:click=move |_| {
                                                        stop_container_action.dispatch(StopContainer { id });
                                                    }
                                                >

                                                    "Stop"
                                                </button>
                                            }
                                                .into_view()
                                        } else {
                                            view! {
                                                <button
                                                    class="p-2 rounded bg-green-700 px-6 text-white"
                                                    on:click=move |_| {
                                                        start_container_action.dispatch(StartContainer { id });
                                                    }
                                                >

                                                    "Start"
                                                </button>
                                            }
                                                .into_view()
                                        }
                                    }}

                                </div>
                            </div>
                        }
                            .into_view()
                    } else {
                        view! { <div>"Failed to load container status"</div> }.into_view()
                    }
                }
            }
            <ContainerStats id=id/>

        </Transition>
    }
}

#[component]
pub fn ContainerStats(id: Uuid) -> impl IntoView {
    leptos_sse::provide_sse(&format!("/events/container/see/{id}")).unwrap();

    #[derive(Debug, Serialize, Deserialize, Clone)]
    struct CpuUsage {
        total_usage: u128,
    }

    #[derive(Debug, Serialize, Deserialize, Clone)]
    struct CpuStats {
        online_cpus: u32,
        system_cpu_usage: u128,
        cpu_usage: CpuUsage,
    }

    #[derive(Debug, Serialize, Deserialize, Clone)]
    struct MemoryStats {
        limit: u128,
        usage: u128,
    }

    #[derive(Debug, Serialize, Deserialize, Clone)]
    struct Stats {
        cpu_stats: CpuStats,
        precpu_stats: CpuStats,
        memory_stats: MemoryStats,
        read: chrono::DateTime<chrono::Utc>,
    }

    // Create server signal
    let stats = create_sse_signal::<Value>("stats");

    let (stats_vec, set_stats_vec) = create_signal(vec![]);

    create_effect(move |_| {
        let stats = stats.get();
        let stats = serde_json::from_value::<Stats>(stats);
        if let Ok(stats) = stats {
            let mut data = stats_vec.get_untracked();
            data.push(stats);
            set_stats_vec.set(data);
        }
    });

    let cpu_series = Series::new(|data: &Stats| data.read).line(Line::new(|data: &Stats| {
        let cpu_delta =
            data.cpu_stats.cpu_usage.total_usage - data.precpu_stats.cpu_usage.total_usage;
        let system_cpu_delta = data.cpu_stats.system_cpu_usage - data.precpu_stats.system_cpu_usage;

        let usage_perc =
            ((cpu_delta as f64 / system_cpu_delta as f64) as f64) * (data.cpu_stats.online_cpus as f64) * 100_f64;


        info!("CPU DELTA {cpu_delta}\nSYSTEM DELTA {system_cpu_delta}\nUSAGE {usage_perc} USAGE {}/{}", data.cpu_stats.cpu_usage.total_usage, data.cpu_stats.system_cpu_usage);
        usage_perc
    }));

    let memory_series = Series::new(|data: &Stats| data.read).line(Line::new(|data: &Stats| {
        let usage_perc = ((data.memory_stats.usage as f64/ data.memory_stats.limit as f64) as f64) * 100_f64;
        usage_perc
    }));

    view! {
        <div class="bg-white p-2 rounded-md border text-black">
            <Chart
                aspect_ratio=AspectRatio::from_env_width(300.0)
                series=cpu_series
                data=stats_vec

                // Decorate our chart
                top=RotatedLabel::middle("CPU Usage")
                left=TickLabels::aligned_floats()
                // bottom=Legend::end()
                inner=[
                    // Standard set of inner layout options
                    AxisMarker::left_edge().into_inner(),
                    AxisMarker::bottom_edge().into_inner(),
                    XGridLine::default().into_inner(),
                    YGridLine::default().into_inner(),
                    YGuideLine::over_mouse().into_inner(),
                    XGuideLine::over_data().into_inner(),
                ]
                tooltip=Tooltip::left_cursor()
            />
        </div>
        <div class="h-6" />
        <div class="bg-white p-2 rounded-md border text-black">
            <Chart
                aspect_ratio=AspectRatio::from_env_width(300.0)
                series=memory_series
                data=stats_vec

                // Decorate our chart
                top=RotatedLabel::middle("Memory Usage")
                left=TickLabels::aligned_floats()
                // bottom=Legend::end()
                inner=[
                    // Standard set of inner layout options
                    AxisMarker::left_edge().into_inner(),
                    AxisMarker::bottom_edge().into_inner(),
                    XGridLine::default().into_inner(),
                    YGridLine::default().into_inner(),
                    YGuideLine::over_mouse().into_inner(),
                    XGuideLine::over_data().into_inner(),
                ]
                tooltip=Tooltip::left_cursor()
            />
        </div>
    }
}
