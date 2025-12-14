use crate::common::{ProcessInfo, SystemStats};
use crate::components::file_browser::FileBrowser;
use crate::components::terminal::TerminalComponent;
use crate::hooks::use_socket::{use_socket, WsMessage};
use leptos::prelude::*;

use leptos_router::components::A;

#[component]
pub fn Dashboard() -> impl IntoView {
    let (stats, set_stats) = signal(None::<SystemStats>);

    let socket = use_socket("/events/system/stats/ws");
    let message = socket.message;
    Effect::new(move |_| {
        message.with(|msg| {
            if let Some(WsMessage::Text(text)) = msg {
                if let Ok(data) = serde_json::from_str::<SystemStats>(text) {
                    set_stats.set(Some(data));
                }
            }
        });
    });

    let (show_processes, set_show_processes) = signal(false);

    view! {
        <div class="p-6 h-full overflow-y-auto bg-gray-50 dark:bg-black text-gray-800 dark:text-gray-100 font-sans">
             <div class="max-w-7xl mx-auto space-y-6">
                 <div class="flex justify-between items-center">
                     <h1 class="text-3xl font-bold bg-clip-text text-transparent bg-gradient-to-r from-blue-500 to-purple-600">"Dashboard"</h1>
                     <A href="/projects" attr:class="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 transition-colors">"Projects"</A>
                 </div>

                 <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
                     <StatsCard
                         title="CPU Usage"
                         value={move || stats.get().map(|s| format!("{:.1}%", s.cpu_usage)).unwrap_or("...".to_string())}
                         percent={move || stats.get().map(|s| s.cpu_usage).unwrap_or(0.0)}
                         color="blue"
                         on_click=move || set_show_processes.set(true)
                     />
                     <StatsCard
                         title="Memory"
                         value={move || stats.get().map(|s| format_bytes(s.used_memory) + " / " + &format_bytes(s.total_memory)).unwrap_or("...".to_string())}
                         percent={move || stats.get().map(|s| (s.used_memory as f32 / s.total_memory as f32) * 100.0).unwrap_or(0.0)}
                         color="purple"
                         on_click=move || set_show_processes.set(true)
                     />
                     <StatsCard
                         title="Swap"
                         value={move || stats.get().map(|s| format_bytes(s.used_swap) + " / " + &format_bytes(s.total_swap)).unwrap_or("...".to_string())}
                         percent={move || stats.get().map(|s| if s.total_swap > 0 { (s.used_swap as f32 / s.total_swap as f32) * 100.0 } else { 0.0 }).unwrap_or(0.0)}
                         color="orange"
                         on_click=move || set_show_processes.set(true)
                     />

                      <div class="bg-white dark:bg-gray-900 p-4 rounded-xl shadow-sm border border-gray-100 dark:border-gray-800">
                          <h3 class="text-sm font-medium text-gray-500 dark:text-gray-400 mb-2">"Storage"</h3>
                          <div class="space-y-3 overflow-y-auto max-h-24 scrollbar-thin">
                              {move || stats.get().map(|s| s.disks.iter().map(|d| {
                                  let percent = if d.total_space > 0 { (d.total_space - d.available_space) as f32 / d.total_space as f32 * 100.0 } else { 0.0 };
                                  view! {
                                      <div>
                                          <div class="flex justify-between text-xs mb-1">
                                              <span class="truncate pr-2" title={d.name.clone()}>{d.name.clone()}</span>
                                              <span>{format!("{:.0}%", percent)}</span>
                                          </div>
                                          <div class="w-full bg-gray-200 dark:bg-gray-800 rounded-full h-1.5 overflow-hidden">
                                              <div class="bg-green-500 h-1.5 rounded-full" style=format!("width: {}%", percent)></div>
                                          </div>
                                      </div>
                                  }
                              }).collect_view())}
                          </div>
                      </div>
                 </div>

                 <div class="grid grid-cols-1 lg:grid-cols-2 gap-6 h-[600px]">
                     <div class="bg-black rounded-xl shadow-lg border border-gray-800 overflow-hidden flex flex-col">
                         <div class="bg-gray-900 p-2 border-b border-gray-800 flex justify-between items-center px-4">
                             <span class="text-xs font-mono text-gray-400">"root@self-cloud:~"</span>
                             <div class="flex gap-1.5">
                                 <div class="w-2.5 h-2.5 rounded-full bg-red-500"></div>
                                 <div class="w-2.5 h-2.5 rounded-full bg-yellow-500"></div>
                                 <div class="w-2.5 h-2.5 rounded-full bg-green-500"></div>
                             </div>
                         </div>
                         <div class="flex-1 relative">
                              <TerminalComponent url="/events/terminal/ws?command=bash&size_width=80&size_height=24"/>
                         </div>
                     </div>

                     <div class="bg-white dark:bg-gray-900 rounded-xl shadow-sm border border-gray-100 dark:border-gray-800 overflow-hidden flex flex-col">
                         <FileBrowser/>
                     </div>
                 </div>
             </div>

             {move || if show_processes.get() {
                 view! { <ProcessModal on_close=move || set_show_processes.set(false) /> }.into_any()
             } else {
                 ().into_any()
             }}
        </div>
    }
}

#[component]
fn StatsCard<F>(
    title: &'static str,
    value: impl IntoView + 'static,
    percent: impl Fn() -> f32 + 'static + Send + Copy,
    color: &'static str,
    on_click: F,
) -> impl IntoView
where
    F: Fn() + 'static + Send + Copy,
{
    view! {
        <div
             class="bg-white dark:bg-gray-900 p-4 rounded-xl shadow-sm border border-gray-100 dark:border-gray-800 cursor-pointer hover:border-blue-500 transition-colors"
             on:click=move |_| on_click()
        >
             <h3 class="text-sm font-medium text-gray-500 dark:text-gray-400 mb-1">{title}</h3>
             <div class="text-2xl font-bold mb-2 dark:text-white">{value}</div>
             <div class="w-full bg-gray-200 dark:bg-gray-800 rounded-full h-2 overflow-hidden">
                 <div
                     class=format!("h-2 rounded-full bg-{}-500 transition-all duration-500", color)
                     style=move || format!("width: {}%", percent())
                 ></div>
             </div>
        </div>
    }
}

#[component]
fn ProcessModal<F>(on_close: F) -> impl IntoView
where
    F: Fn() + 'static + Send + Copy,
{
    let (processes, set_processes) = signal(Vec::<ProcessInfo>::new());
    let socket = use_socket("/events/system/processes/ws");
    let message = socket.message;
    Effect::new(move |_| {
        message.with(|msg| {
            if let Some(WsMessage::Text(text)) = msg {
                if let Ok(data) = serde_json::from_str::<Vec<ProcessInfo>>(text) {
                    set_processes.set(data);
                }
            }
        });
    });

    view! {
        <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm">
            <div class="bg-white dark:bg-gray-900 w-full max-w-4xl h-[80vh] rounded-xl shadow-2xl flex flex-col overflow-hidden border dark:border-gray-700">
                <div class="p-4 border-b dark:border-gray-800 flex justify-between items-center">
                    <h2 class="text-xl font-bold dark:text-white">"Process Explorer"</h2>
                    <button class="text-gray-500 hover:text-gray-700 dark:hover:text-gray-300" on:click=move |_| on_close()>"✕"</button>
                </div>
                <div class="flex-1 overflow-auto bg-gray-50 dark:bg-gray-950 p-2">
                     <table class="w-full text-left text-xs font-mono">
                         <thead class="sticky top-0 bg-gray-200 dark:bg-gray-800 text-gray-700 dark:text-gray-300">
                             <tr>
                                 <th class="p-2">"PID"</th>
                                 <th class="p-2">"Name"</th>
                                 <th class="p-2">"CPU %"</th>
                                 <th class="p-2">"Mem"</th>
                                 <th class="p-2">"User"</th>
                             </tr>
                         </thead>
                         <tbody class="divide-y divide-gray-200 dark:divide-gray-800">
                             {move || processes.get().into_iter().take(100).map(|p| view! {
                                 <tr class="hover:bg-blue-50 dark:hover:bg-blue-900/10">
                                     <td class="p-2 text-gray-500">{p.pid}</td>
                                      <td class="p-2 font-bold text-gray-800 dark:text-gray-200 truncate max-w-[200px]" title={p.name.clone()}>{p.name.clone()}</td>
                                     <td class="p-2 text-blue-600 dark:text-blue-400">{format!("{:.1}", p.cpu_usage)}</td>
                                     <td class="p-2 text-purple-600 dark:text-purple-400">{format_bytes(p.memory)}</td>
                                     <td class="p-2 text-gray-500">{p.user_id.unwrap_or_default()}</td>
                                 </tr>
                             }).collect_view()}
                         </tbody>
                     </table>
                     {move || if processes.get().is_empty() {
                         view! { <div class="p-8 text-center text-gray-500">"Waiting for process data..."</div> }.into_any()
                     } else { ().into_any() }}
                </div>
            </div>
        </div>
    }
}

fn format_bytes(bytes: u64) -> String {
    const UNIT: u64 = 1024;
    if bytes < UNIT {
        return format!("{} B", bytes);
    }
    let exp = (bytes as f64).ln() / (UNIT as f64).ln();
    let pre = match exp as i32 {
        1 => "K",
        2 => "M",
        3 => "G",
        4 => "T",
        5 => "P",
        _ => "?",
    };
    format!(
        "{:.1} {}B",
        (bytes as f64) / (UNIT as f64).powi(exp as i32),
        pre
    )
}
