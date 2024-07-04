use std::collections::VecDeque;
use std::sync::Mutex;

use leptos::{
    component, create_effect, create_node_ref, create_resource, create_server_action,
    expect_context, prelude::*, view, For, IntoView, Transition,
};
use leptos_chartistry::IntoInner;
use leptos_chartistry::{
    AspectRatio, AxisMarker, Chart, Line, RotatedLabel, Series, TickLabels, Tooltip, XGridLine,
    XGuideLine, YGridLine, YGuideLine,
};
use leptos_use::{use_interval_fn, utils::Pausable};
use leptos_use::{use_websocket, UseWebsocketReturn};
use serde::{Deserialize, Serialize};
use tracing::warn;
use uuid::Uuid;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};

use crate::api::{
    inspect_container, PauseContainer, ResumeContainer, StartContainer, StopContainer,
};
use crate::common::{AttachParams, TtyChunk};
use crate::utils::xterm::Terminal;
use leptos_icons::Icon;

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

                                <div class="flex-grow"></div>
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
            <div class="h-2"></div>
            <ContainerSubPages id/>

        </Transition>
    }
}
#[component]
pub fn ContainerSubPages(id: Uuid) -> impl IntoView {
    #[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
    enum ContainerPageType {
        Logs,
        Stats,
        Attach,
    }

    #[derive(Clone)]
    struct ContainerPage<'a> {
        name: &'a str,
        icon: icondata::Icon,
        r#type: ContainerPageType,
    }

    let container_sub_pages = Vec::from([
        ContainerPage {
            name: "Logs",
            icon: icondata::OcLogLg,
            r#type: ContainerPageType::Logs,
        },
        ContainerPage {
            name: "Stats",
            icon: icondata::ImStatsDots,
            r#type: ContainerPageType::Stats,
        },
        ContainerPage {
            name: "Attach",
            icon: icondata::IoTerminal,
            r#type: ContainerPageType::Attach,
        },
    ]);
    let (selected_page, set_selected_page) = create_signal(container_sub_pages[0].r#type);

    view! {
        <div class="flex border-black dark:border-white/80 border-b items-end gap-1 px-2">
            <For
                each=move || container_sub_pages.clone()
                key=|page| page.r#type.clone()
                children=move |page| {
                    view! {
                        <button
                            class="p-2 border dark:border-white/80 border-black mb-[-0.05em] rounded-t-lg flex gap-2 items-center"
                            class=(
                                ["border-b-slate-100", "dark:border-b-black", "p-3"],
                                move || page.r#type == selected_page.get(),
                            )

                            on:click=move |_| { set_selected_page.set(page.r#type) }
                        >

                            <Icon icon=page.icon/>

                            {page.name}

                        </button>
                    }
                }
            />

        </div>

        <div class="h-2"></div>

        {move || match selected_page.get() {
            ContainerPageType::Logs => {
                view! { <ContainerLogs id/> }
            }
            ContainerPageType::Stats => {
                view! { <ContainerStats id/> }
            }
            ContainerPageType::Attach => {
                view! { <ContainerAttach id/> }
            }
        }}
    }
}
#[component]
pub fn ContainerStats(id: Uuid) -> impl IntoView {
    #[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
    struct CpuUsage {
        total_usage: u128,
    }

    #[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
    struct CpuStats {
        online_cpus: u32,
        system_cpu_usage: u128,
        cpu_usage: CpuUsage,
    }

    #[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
    struct MemoryStats {
        limit: u128,
        usage: u128,
    }

    #[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
    struct Stats {
        cpu_stats: CpuStats,
        precpu_stats: CpuStats,
        memory_stats: MemoryStats,
        read: chrono::DateTime<chrono::Utc>,
    }

    // Create server signal
    let UseWebsocketReturn {
        // ready_state,
        message,
        // message_bytes,
        // send,
        // send_bytes,
        // open,
        // close,
        ..
    } = use_websocket(&format!("/events/container/{id}/stats/ws"));

    let (stats_vecdq, _set_stats_vecdq) =
        create_signal(std::rc::Rc::new(Mutex::new(VecDeque::with_capacity(30))));
    let (stats_vec, set_stats_vec) = create_signal(Vec::new());

    let (received_json, set_received_json) = create_signal(serde_json::Value::Null);
    create_effect(move |_| {
        let message = message.get();
        if let Some(message) = message {
            let patch = serde_json::from_str::<json_patch::Patch>(&message);
            match patch {
                Ok(patch) => {
                    let mut data = received_json.get_untracked();

                    if let Err(_err) = json_patch::patch(&mut data, &patch) {
                        warn!("Json patch failed")
                    } else {
                        set_received_json.set(data.clone());
                        let stats = serde_json::from_value::<Stats>(data);
                        match stats {
                            Ok(stats) => {
                                let data = stats_vecdq.get_untracked();
                                let lock = data.lock();
                                match lock {
                                    Ok(mut data) => {
                                        if data.len() >= 30 {
                                            data.pop_front();
                                        }
                                        data.push_back(stats);
                                        let data_arranged = data.make_contiguous();
                                        let data_vec = Vec::from(data_arranged);
                                        set_stats_vec.set(data_vec);
                                    }
                                    Err(_) => {
                                        warn!("Cant lock dq");
                                    }
                                }
                            }
                            Err(err) => {
                                warn!("Failed to parse json to stats {err:?}")
                            }
                        }
                    }
                }
                Err(err) => {
                    tracing::warn!("Received data not json-patch {err:?}")
                }
            }
        }
    });

    let cpu_series = Series::new(|data: &Stats| data.read).line(Line::new(|data: &Stats| {
        let cpu_delta =
            data.cpu_stats.cpu_usage.total_usage - data.precpu_stats.cpu_usage.total_usage;
        let system_cpu_delta = data.cpu_stats.system_cpu_usage - data.precpu_stats.system_cpu_usage;

        let usage_perc = ((cpu_delta as f64 / system_cpu_delta as f64) as f64)
            * (data.cpu_stats.online_cpus as f64)
            * 100_f64;
        usage_perc
    }));

    let memory_series = Series::new(|data: &Stats| data.read).line(Line::new(|data: &Stats| {
        let usage_perc =
            ((data.memory_stats.usage as f64 / data.memory_stats.limit as f64) as f64) * 100_f64;
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
                bottom=TickLabels::timestamps()
                // bottom=Legend::end()
                inner=[
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
        <div class="h-6"></div>
        <div class="bg-white p-2 rounded-md border text-black">
            <Chart
                aspect_ratio=AspectRatio::from_env_width(300.0)
                series=memory_series
                data=stats_vec

                // Decorate our chart
                top=RotatedLabel::middle("Memory Usage")
                left=TickLabels::aligned_floats()
                bottom=TickLabels::timestamps()
                // bottom=Legend::end()
                inner=[
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

#[component]
pub fn ContainerLogs(id: Uuid) -> impl IntoView {
    // Create server signal
    let UseWebsocketReturn {
        // ready_state,
        // message,
        message_bytes,
        // send,
        // send_bytes,
        // open,
        // close,
        ..
    } = use_websocket(&format!("/events/container/{id}/logs/ws"));

    let (output, set_output) = create_signal(String::new());
    let div_ref = create_node_ref::<leptos::html::Div>();

    create_effect(move |_| {
        let message = message_bytes.get();
        if let Some(message) = message {
            let chunk = bincode::deserialize::<TtyChunk>(&message);
            match chunk {
                Ok(chunk) => {
                    let string = std::str::from_utf8(chunk.as_ref());
                    if let Ok(string) = string {
                        if let Ok(html) = ansi_to_html::convert(&string) {
                            let mut data = output.get_untracked();
                            data.push_str(&html);
                            if let Some(node) = div_ref.get_untracked() {
                                node.set_inner_html(&data);
                            }
                            set_output.set(data);
                        }
                    }
                }
                Err(err) => {
                    tracing::warn!("Received data not tty-chunk {err:?}")
                }
            }
        }
    });

    view! {
        <div
            _ref=div_ref
            class="bg-white p-2 rounded-md border text-black whitespace-break-spaces max-h-80 overflow-auto"
        ></div>
    }
}

#[component]
pub fn ContainerAttach(id: Uuid) -> impl IntoView {
    let div_ref = create_node_ref::<leptos::html::Div>();

    let (terminal, set_terminal) = create_signal(Option::<std::rc::Rc<Terminal>>::None);

    create_effect(move |_| {
        if let Some(terminal) = terminal.get() {
            let params = AttachParams {
                command: "/usr/bin/bash".to_string(),
                size_height: terminal.rows() as u64,
                size_width: terminal.cols() as u64,
            };
            let params = serde_urlencoded::to_string(&params).unwrap_or_default();
            // Create server signal
            let UseWebsocketReturn {
                // ready_state,
                // message,
                message_bytes,
                send,
                // send_bytes,
                // open,
                // close,
                ..
            } = use_websocket(&format!("/events/container/{id}/attach/ws?{params}"));

            let closure = Closure::wrap(Box::new(move |data: JsValue| {
                let event = data.as_string();
                if let Some(event) = event {
                    send(&event);
                } else {
                    tracing::info!("Data is not string {data:?}");
                }
            }) as Box<dyn Fn(JsValue)>);

            terminal.onData(closure.as_ref().unchecked_ref());
            closure.forget();
            let vt = vt100::Parser::new(terminal.rows() as u16, terminal.cols() as u16, 0);
            // let screen = vt.screen().clone(); TODO: use for diff
            let vt_rc = std::rc::Rc::new(Mutex::new(vt));
            // let screen = std::rc::Rc::new(Mutex::new(screen)); // TODO: use for diff

            create_effect(move |_| {
                let message = message_bytes.get();
                if let Some(message) = message {
                    let chunk = bincode::deserialize::<TtyChunk>(&message);
                    match chunk {
                        Ok(chunk) => {
                            if let Ok(mut vt) = vt_rc.lock() {
                                vt.process(chunk.as_ref());
                                let new_screen = vt.screen().clone();
                                let contents = new_screen.contents_formatted(); // TODO: Remove vt100 or use diff
                                                                                // *screen = new_screen;
                                let contents_str = std::str::from_utf8(&contents);
                                if let Ok(contents_str) = contents_str {
                                    let uint8_array = contents_str.into();
                                    terminal.clear();
                                    terminal.write(&uint8_array);
                                } else {
                                    let uint8_array =
                                        unsafe { js_sys::Uint8Array::view(contents.as_ref()) };
                                    terminal.clear();
                                    terminal.write(&uint8_array);
                                }
                            }
                        }
                        Err(err) => {
                            tracing::warn!("Received data not tty-chunk {err:?}")
                        }
                    }
                }
            });
        }
    });

    view! {
        <link href="/css/xterm.min.css " rel="stylesheet"/>
        <script
            src="/js/xterm.min.js"
            on:load=move |_| {
                #[derive(Serialize)]
                struct TerminalOptions {
                    scrollback: u64,
                }
                let options = serde_wasm_bindgen::to_value(&TerminalOptions { scrollback: 0 });
                if let Ok(options) = options {
                    let terminal = Terminal::new(&options);
                    if let Some(div) = div_ref.get_untracked() {
                        tracing::info!("Open terminal");
                        terminal.open(&div);
                    }
                    use std::rc::Rc;
                    set_terminal.set(Some(Rc::new(terminal)));
                } else {
                    tracing::warn!("Cant convert terminalOptions");
                }
            }
        ></script>

        <div _ref=div_ref class=""></div>
    }
}
