use crate::components::toaster::{ToastVariant, ToasterContext};

use leptos::prelude::*;
use leptos_use::{use_interval_fn, utils::Pausable};
use uuid::Uuid;

use crate::api::{
    inspect_container, PauseContainer, ResumeContainer, StartContainer, StopContainer,
};
use crate::common::TtyChunk;
use leptos_router::hooks::use_query_map;
// use leptos_icons::Icon;
use crate::hooks::use_socket::{use_socket, WsMessage};
// use leptos::signal::{SignalGet, SignalWith};

// BincodeCodec removed

#[component]
pub fn ContainerPage() -> impl IntoView {
    let id = expect_context::<Signal<Uuid>>();

    let container = Resource::new(
        move || id.get(),
        move |id| async move {
            let result = inspect_container(id).await;

            result
        },
    );

    let Pausable {
        // pause,
        // resume,
        // is_active,
        ..
    } = use_interval_fn(
        move || {
            container.refetch();
        },
        5000,
    );
    let toast_context = expect_context::<ToasterContext>();
    let pause_container_action = ServerAction::<PauseContainer>::new();
    Effect::new({
        let toast_context = toast_context.clone();
        move |_| {
            if pause_container_action.version().get() > 0 {
                toast_context.toast("Container Paused", ToastVariant::Info);
                container.refetch();
            }
        }
    });

    let resume_container_action = ServerAction::<ResumeContainer>::new();
    Effect::new({
        let toast_context = toast_context.clone();
        move |_| {
            if resume_container_action.version().get() > 0 {
                toast_context.toast("Container Resumed", ToastVariant::Info);
                container.refetch();
            }
        }
    });

    let stop_container_action = ServerAction::<StopContainer>::new();
    Effect::new({
        let toast_context = toast_context.clone();
        move |_| {
            if stop_container_action.version().get() > 0 {
                toast_context.toast("Container Stopped", ToastVariant::Info);
                container.refetch();
            }
        }
    });

    let start_container_action = ServerAction::<StartContainer>::new();
    Effect::new({
        let toast_context = toast_context.clone();
        move |_| {
            if start_container_action.version().get() > 0 {
                toast_context.toast("Container Started", ToastVariant::Info);
                container.refetch();
            }
        }
    });
    let query = use_query_map();
    let sub_page = Memo::new(move |_| query.get().get("page").map(|s| s.clone()));
    let id_val = id.get();

    view! {
        <div class="h-full w-full flex flex-col">
            <div class="flex-none p-4 border-b bg-white dark:bg-gray-800 dark:border-gray-700">
                <div class="flex items-center justify-between mb-4">
                    <h1 class="text-2xl font-bold dark:text-white">"Container Details"</h1>
                    <div class="flex gap-2">
                        <a
                            href=move || format!("/projects/{}", id_val)
                            class="px-3 py-1 text-sm bg-gray-100 hover:bg-gray-200 rounded text-gray-700 transition-colors dark:bg-gray-700 dark:hover:bg-gray-600 dark:text-gray-200"
                        >
                            "Back to Containers"
                        </a>
                    </div>
                </div>
                <ContainerControls container_id=id_val/>
            </div>

            <div class="flex-1 overflow-hidden flex flex-col">
               <div class="flex-none border-b bg-gray-50 dark:bg-gray-900 dark:border-gray-700">
                   <div class="flex gap-4 px-4">
                       <a
                           href=move || format!("/projects/{}/container?page=logs", id_val)
                           class=move || {
                               let active = sub_page.get().as_deref() == Some("logs");
                               format!(
                                   "py-2 px-1 border-b-2 transition-colors text-sm font-medium {}",
                                   if active {
                                       "border-blue-500 text-blue-600 dark:text-blue-400"
                                   } else {
                                       "border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-300"
                                   }
                               )
                           }
                       >
                           "Logs"
                       </a>
                       <a
                           href=move || format!("/projects/{}/container?page=stats", id_val)
                           class=move || {
                               let active = sub_page.get().as_deref() == Some("stats");
                               format!(
                                   "py-2 px-1 border-b-2 transition-colors text-sm font-medium {}",
                                   if active {
                                       "border-blue-500 text-blue-600 dark:text-blue-400"
                                   } else {
                                       "border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-300"
                                   }
                               )
                           }
                       >
                           "Stats"
                       </a>
                       <a
                           href=move || format!("/projects/{}/container?page=attach", id_val)
                           class=move || {
                               let active = sub_page.get().as_deref() == Some("attach");
                               format!(
                                   "py-2 px-1 border-b-2 transition-colors text-sm font-medium {}",
                                   if active {
                                       "border-blue-500 text-blue-600 dark:text-blue-400"
                                   } else {
                                       "border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-300"
                                   }
                               )
                           }
                       >
                           "Terminal"
                       </a>
                   </div>
               </div>

               <div class="flex-1 overflow-auto p-4 bg-gray-100 dark:bg-gray-950">
                   <ContainerSubPages id=id_val/>
               </div>
            </div>
        </div>
    }
}

#[component]
fn ContainerControls(container_id: Uuid) -> impl IntoView {
    let inspect = Resource::new(
        move || container_id,
        |id| async move { inspect_container(id).await },
    );

    let start = ServerAction::<StartContainer>::new();
    let stop = ServerAction::<StopContainer>::new();
    let pause = ServerAction::<PauseContainer>::new();
    let resume = ServerAction::<ResumeContainer>::new();

    view! {
        <Transition fallback=move || view! { <div class="text-sm text-gray-500">"Loading state..."</div> }>
            {move || {
                inspect.get().map(|res| match res {
                    Ok(info) => {
                         let state_opt = info.state.clone();
                         let status = state_opt.as_ref().and_then(|s| s.status.clone()).unwrap_or("unknown".to_string());
                         let running = status == "running";
                         let paused = status == "paused";

                         view! {
                             <div class="flex gap-2 items-center">
                                 <div class=format!("w-3 h-3 rounded-full {}", if running { "bg-green-500" } else { "bg-red-500" })></div>
                                 <span class="text-sm font-medium dark:text-gray-300 uppercase mr-4">{status}</span>

                                 {if !running {
                                     view! {
                                         <ActionForm action=start>
                                             <input type="hidden" name="id" value=container_id.to_string()/>
                                             <button type="submit" class="p-1 hover:bg-gray-100 rounded text-green-600 dark:hover:bg-gray-700">
                                                 // <Icon icon=icondata::BsPlayFill class="w-5 h-5"/>
                                                 "Start"
                                             </button>
                                         </ActionForm>
                                     }.into_any()
                                 } else {
                                     view! {
                                         <ActionForm action=stop>
                                             <input type="hidden" name="id" value=container_id.to_string()/>
                                             <button type="submit" class="p-1 hover:bg-gray-100 rounded text-red-600 dark:hover:bg-gray-700">
                                                 // <Icon icon=icondata::BsStopFill class="w-5 h-5"/>
                                                 "Stop"
                                             </button>
                                         </ActionForm>
                                     }.into_any()
                                 }}

                                 {if running && !paused {
                                     view! {
                                         <ActionForm action=pause>
                                             <input type="hidden" name="id" value=container_id.to_string()/>
                                             <button type="submit" class="p-1 hover:bg-gray-100 rounded text-yellow-600 dark:hover:bg-gray-700">
                                                 // <Icon icon=icondata::BsPauseFill class="w-5 h-5"/>
                                                 "Pause"
                                             </button>
                                         </ActionForm>
                                     }.into_any()
                                 } else if paused {
                                     view! {
                                         <ActionForm action=resume>
                                             <input type="hidden" name="id" value=container_id.to_string()/>
                                             <button type="submit" class="p-1 hover:bg-gray-100 rounded text-yellow-600 dark:hover:bg-gray-700">
                                                 // <Icon icon=icondata::BsPlayFill class="w-5 h-5"/>
                                                 "Resume"
                                             </button>
                                         </ActionForm>
                                     }.into_any()
                                 } else {
                                     ().into_any()
                                 }}
                             </div>
                         }.into_any()
                    }
                    Err(_) => view! { <span class="text-red-500">"Error loading info"</span> }.into_any(),
                })
            }}
        </Transition>
    }
}

#[component]
fn ContainerSubPages(id: Uuid) -> impl IntoView {
    let query = use_query_map();
    let page = move || {
        query
            .get()
            .get("page")
            .map(|s| s.clone())
            .unwrap_or_else(|| "logs".to_string())
    };

    view! {
        <div>
            {move || match page().as_str() {
                "stats" => view! { <ContainerStats container_id=id/> }.into_any(),
                "attach" => view! { <ContainerAttach container_id=id/> }.into_any(),
                _ => view! { <ContainerLogs container_id=id/> }.into_any(),
            }}
        </div>
    }
}

#[derive(Clone, Default)]
struct StatsHistory {
    cpu: Vec<(i64, f64)>,
    memory: Vec<(i64, f64)>,
    network_rx: Vec<(i64, f64)>,
    network_tx: Vec<(i64, f64)>,
    last_cpu: Option<(u64, u64)>, // total_usage, system_usage
}

#[component]
pub fn ContainerStats(container_id: Uuid) -> impl IntoView {
    use crate::components::apex_chart::{ApexChart, ChartSeries};
    use chrono::Utc;

    let socket = use_socket(&format!("/events/container/{container_id}/stats/ws"));
    let message = socket.message;

    let (stats_history, set_stats_history) = signal(StatsHistory::default());

    Effect::new(move |_| {
        message.with(|msg| {
            if let Some(WsMessage::Text(text)) = msg {
                if let Ok(stat) = serde_json::from_str::<serde_json::Value>(text) {
                    // Check if it's a full stat object or a patch (we might need to handle both, but start with assumption of full object or patch that results in full object if we maintained state - wait, the previous code was using json_patch, so we receive PATCHES.
                    // IMPORTANT: The previous code maintained `received_json` and applied patches. We must do the same to get the full state for calculation.
                }
            }
        });
    });

    // Re-implementing the state maintenance from previous code + parsing
    let (current_stats, set_current_stats) = signal(serde_json::Value::Null);

    Effect::new(move |_| {
        message.with(|msg| {
            if let Some(WsMessage::Text(text)) = msg {
                if let Ok(patch) = serde_json::from_str::<json_patch::Patch>(text) {
                    set_current_stats.update(|data| {
                        if let Err(err) = json_patch::patch(data, &patch) {
                            tracing::warn!("Json patch failed {err:?}");
                        }
                    });
                }
            }
        });
    });

    // Effect to calculate metrics when current_stats changes
    Effect::new(move |_| {
        let stats = current_stats.get();
        if !stats.is_null() {
            let now = Utc::now().timestamp_millis();

            set_stats_history.update(|history| {
                // CPU
                if let (Some(cpu_stats), Some(precpu_stats)) =
                    (stats.get("cpu_stats"), stats.get("precpu_stats"))
                {
                    let total_usage = cpu_stats["cpu_usage"]["total_usage"].as_u64().unwrap_or(0);
                    let system_usage = cpu_stats["system_cpu_usage"].as_u64().unwrap_or(0);
                    let online_cpus = cpu_stats["online_cpus"].as_u64().unwrap_or(1);

                    // Use precpu_stats directly from JSON as it is usually provided by Docker API,
                    // OR use our own last_cpu if we trust it more. Docker's precpu is usually reliable.
                    let pre_total_usage = precpu_stats["cpu_usage"]["total_usage"]
                        .as_u64()
                        .unwrap_or(0);
                    let pre_system_usage = precpu_stats["system_cpu_usage"].as_u64().unwrap_or(0);

                    if system_usage > pre_system_usage {
                        let cpu_delta = total_usage.saturating_sub(pre_total_usage) as f64;
                        let system_delta = system_usage.saturating_sub(pre_system_usage) as f64;

                        let mut cpu_percent =
                            (cpu_delta / system_delta) * online_cpus as f64 * 100.0;
                        if cpu_percent.is_nan() {
                            cpu_percent = 0.0;
                        }
                        history.cpu.push((now, cpu_percent));
                    }
                }

                // Memory
                if let Some(memory_stats) = stats.get("memory_stats") {
                    let usage = memory_stats["usage"].as_f64().unwrap_or(0.0);
                    // Convert to MB
                    history.memory.push((now, usage / 1024.0 / 1024.0));
                }

                // Network (eth0)
                if let Some(networks) = stats.get("networks") {
                    if let Some(eth0) = networks.get("eth0") {
                        let rx = eth0["rx_bytes"].as_f64().unwrap_or(0.0);
                        let tx = eth0["tx_bytes"].as_f64().unwrap_or(0.0);
                        // For total bytes, just plotting the raw value will show a steep line.
                        // Usually we want rate. But for simplicity let's plot active value or maybe strict usage.
                        // Actually, plotting raw bytes over time shows the accumulation.
                        // Rate would be better. Let's do Rate if we have prev value.
                        if let Some((last_ts, last_rx)) = history.network_rx.last() {
                            // This is complicated because we might be missing data points.
                            // for now, let's just plot the raw bytes to show activity, or maybe KB?
                            // Actually, let's stick to simple "Usage" (Total Bytes) for now as requested "stats in a graph".
                            // Rate calculation needs reliable intervals.
                            history.network_rx.push((now, rx));
                            history.network_tx.push((now, tx));
                        } else {
                            history.network_rx.push((now, rx));
                            history.network_tx.push((now, tx));
                        }
                    }
                }

                // Keep only last 50 points
                if history.cpu.len() > 50 {
                    history.cpu.remove(0);
                }
                if history.memory.len() > 50 {
                    history.memory.remove(0);
                }
                if history.network_rx.len() > 50 {
                    history.network_rx.remove(0);
                }
                if history.network_tx.len() > 50 {
                    history.network_tx.remove(0);
                }
            });
        }
    });

    let cpu_series = Memo::new(move |_| {
        vec![ChartSeries {
            name: "CPU Usage %".to_string(),
            data: stats_history.get().cpu,
        }]
    });

    let memory_series = Memo::new(move |_| {
        vec![ChartSeries {
            name: "Memory Usage (MB)".to_string(),
            data: stats_history.get().memory,
        }]
    });

    let network_series = Memo::new(move |_| {
        vec![
            ChartSeries {
                name: "Network Rx (Bytes)".to_string(),
                data: stats_history.get().network_rx,
            },
            ChartSeries {
                name: "Network Tx (Bytes)".to_string(),
                data: stats_history.get().network_tx,
            },
        ]
    });

    let common_options = serde_json::json!({
        "chart": {
            "type": "area",
            "animations": { "enabled": false },
            "toolbar": { "show": false },
            "zoom": { "enabled": false }
        },
        "dataLabels": { "enabled": false },
        "stroke": { "curve": "smooth" },
        "xaxis": {
            "type": "datetime",
            "labels": { "datetimeFormatter": { "year": "yyyy", "month": "MMM 'yy", "day": "dd MMM", "hour": "HH:mm" } }
        },
        "tooltip": { "x": { "format": "dd MMM HH:mm:ss" } },
        "theme": { "mode": "dark" }
    });

    view! {
        <div class="p-4 grid grid-cols-1 md:grid-cols-2 gap-4">
             <div class="bg-white dark:bg-gray-800 p-4 rounded shadow">
                <h3 class="text-lg font-bold mb-2 dark:text-white">"CPU Usage"</h3>
                <ApexChart series=cpu_series options=Signal::from(common_options.clone()) height="300"/>
             </div>
             <div class="bg-white dark:bg-gray-800 p-4 rounded shadow">
                <h3 class="text-lg font-bold mb-2 dark:text-white">"Memory Usage"</h3>
                <ApexChart series=memory_series options=Signal::from(common_options.clone()) height="300"/>
             </div>
              <div class="bg-white dark:bg-gray-800 p-4 rounded shadow col-span-1 md:col-span-2">
                <h3 class="text-lg font-bold mb-2 dark:text-white">"Network Traffic"</h3>
                <ApexChart series=network_series options=Signal::from(common_options) height="300"/>
             </div>
        </div>
    }
}

#[component]
pub fn ContainerLogs(container_id: Uuid) -> impl IntoView {
    let socket = use_socket(&format!("/events/container/{container_id}/logs/ws"));
    let message = socket.message;
    let ready_state = socket.ready_state;

    let (logs, set_logs) = signal(Vec::<String>::new());

    Effect::new(move |_| {
        message.with(|msg| {
            if let Some(WsMessage::Binary(bytes)) = msg {
                if let Ok(chunk) = bincode::deserialize::<TtyChunk>(&bytes) {
                    match chunk {
                        TtyChunk::StdOut(bytes) | TtyChunk::StdErr(bytes) => {
                            let s = String::from_utf8_lossy(&bytes).to_string();
                            set_logs.update(|l| l.push(s));
                        }
                        _ => {}
                    }
                }
            }
        });
    });

    view! {
        <div class="bg-black text-white p-4 font-mono text-xs h-full overflow-auto rounded">
            <div class="text-gray-500 mb-2">
                "Status: " {move || ready_state.get().to_string()}
            </div>
            {move || logs.get().into_iter().map(|line| view! { <div>{line}</div> }).collect_view()}
        </div>
    }
}

#[component]
pub fn ContainerAttach(container_id: Uuid) -> impl IntoView {
    use crate::components::terminal::TerminalComponent;
    let url = move || {
        format!(
            "/events/container/{container_id}/attach/ws?command=sh&size_width=80&size_height=24"
        )
    };

    view! {
        <TerminalComponent url=url()/>
    }
}
