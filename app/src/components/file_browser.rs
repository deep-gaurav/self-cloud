use crate::file_manager::list_files;
use leptos::prelude::*;

#[component]
pub fn FileBrowser() -> impl IntoView {
    let (current_path, set_current_path) = signal(".".to_string());

    let files = Resource::new(move || current_path.get(), move |path| list_files(path));

    view! {
        <div class="h-full flex flex-col dark:text-gray-200">
            <div class="p-2 border-b dark:border-gray-700 flex gap-2 items-center bg-gray-50 dark:bg-gray-800">
                 <button
                     class="px-2 py-1 bg-white hover:bg-gray-100 border rounded shadow-sm text-sm dark:bg-gray-700 dark:border-gray-600 dark:hover:bg-gray-600"
                     on:click=move |_| {
                         let path = current_path.get();
                         let parent = std::path::Path::new(&path).parent().map(|p| p.to_string_lossy().to_string()).unwrap_or(".".to_string());
                         set_current_path.set(if parent.is_empty() { ".".to_string() } else { parent });
                     }
                 >
                     "⬆ Up"
                 </button>
                 <span class="font-mono text-sm overflow-hidden text-ellipsis px-2 py-1 bg-white dark:bg-gray-900 border dark:border-gray-700 rounded flex-1">
                    {move || current_path.get()}
                 </span>
                 <button
                     class="px-2 py-1 bg-blue-500 text-white rounded hover:bg-blue-600 text-sm"
                     on:click=move |_| files.refetch()
                 >
                     "Refresh"
                 </button>
            </div>
            <div class="flex-1 overflow-auto bg-white dark:bg-gray-950">
                <Suspense fallback=move || view! { <div class="p-4 text-center text-gray-500">"Loading..."</div> }>
                    {move || {
                        files.get().map(|res| match res {
                            Ok(list) => view! {
                                <table class="w-full text-left text-sm border-collapse">
                                    <thead class="sticky top-0 bg-gray-50 dark:bg-gray-800 z-10 shadow-sm">
                                        <tr>
                                            <th class="p-3 font-semibold text-gray-600 dark:text-gray-300">"Name"</th>
                                            <th class="p-3 font-semibold text-gray-600 dark:text-gray-300 w-24">"Size"</th>
                                            // <th class="p-3 font-semibold text-gray-600 dark:text-gray-300 w-24">"Actions"</th>
                                        </tr>
                                    </thead>
                                    <tbody class="divide-y divide-gray-100 dark:divide-gray-800">
                                        {list.into_iter().map(|file| {
                                            view! {
                                                <tr class="hover:bg-blue-50 dark:hover:bg-blue-900/20 transition-colors group">
                                                    <td class="p-2 cursor-pointer"
                                                        on:click={
                                                            let is_dir = file.is_dir;
                                                            let path = file.path.clone();
                                                            move |_| {
                                                                if is_dir {
                                                                    set_current_path.set(path.clone());
                                                                }
                                                            }
                                                        }
                                                    >
                                                        <div class="flex items-center gap-2">
                                                            <span class="text-xl">{if file.is_dir { "📁" } else { "📄" }}</span>
                                                            <span class={if file.is_dir { "font-medium text-blue-600 dark:text-blue-400" } else { "text-gray-700 dark:text-gray-300" }}>
                                                                {file.name}
                                                            </span>
                                                        </div>
                                                    </td>
                                                    <td class="p-2 text-gray-500 font-mono text-xs">
                                                        {if file.is_dir { "-".to_string() } else { format_size(file.size) }}
                                                    </td>
                                                    // <td class="p-2">
                                                        // <div class="opacity-0 group-hover:opacity-100 transition-opacity flex gap-2">
                                                            // <button class="text-red-500 hover:text-red-700" title="Delete">"🗑"</button>
                                                        // </div>
                                                    // </td>
                                                </tr>
                                            }
                                        }).collect_view()}
                                    </tbody>
                                </table>
                            }.into_any(),
                            Err(e) => view! {
                                <div class="p-4 text-red-500 bg-red-50 dark:bg-red-900/20 m-4 rounded border border-red-200 dark:border-red-800">
                                    <div class="font-bold">"Error accessing path"</div>
                                    <div class="text-sm">{e.to_string()}</div>
                                </div>
                            }.into_any(),
                        })
                    }}
                </Suspense>
            </div>
        </div>
    }
}

fn format_size(bytes: u64) -> String {
    const UNIT: u64 = 1024;
    if bytes < UNIT {
        return format!("{} B", bytes);
    }
    let exp = (bytes as f64).ln() / (UNIT as f64).ln();
    // let pre = "KMGTPE".chars().nth(exp as usize - 1).unwrap_or('?');
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
