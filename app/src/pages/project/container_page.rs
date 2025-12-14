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
                            href=move || format!("/projects/containers?id={}", id_val)
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
                           href=move || format!("/projects/containers?id={}&page=logs", id_val)
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
                           href=move || format!("/projects/containers?id={}&page=stats", id_val)
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
                           href=move || format!("/projects/containers?id={}&page=attach", id_val)
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

#[component]
pub fn ContainerStats(container_id: Uuid) -> impl IntoView {
    let socket = use_socket(&format!("/events/container/{container_id}/stats/ws"));
    let message = socket.message;
    let ready_state = socket.ready_state;

    let (received_json, set_received_json) = signal(serde_json::Value::Null);

    Effect::new(move |_| {
        message.with(|msg| {
            if let Some(WsMessage::Text(text)) = msg {
                if let Ok(patch) = serde_json::from_str::<json_patch::Patch>(text) {
                    set_received_json.update(|data| {
                        if let Err(err) = json_patch::patch(data, &patch) {
                            tracing::warn!("Json patch failed {err:?}");
                        }
                    });
                }
            }
        });
    });

    view! {
        <div class="bg-white p-2 rounded-md border text-black">
           <div class="p-4">
               <h3 class="text-lg font-bold">"Raw Stats"</h3>
                <div class="text-sm text-gray-500 mb-2">
                    "Status: " {move || ready_state.get().to_string()}
                </div>
               <pre class="text-xs">
                   {move || serde_json::to_string_pretty(&received_json.get()).unwrap_or_default()}
               </pre>
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
