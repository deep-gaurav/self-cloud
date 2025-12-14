use leptos::prelude::*;
use leptos_icons::Icon;
use leptos_router::components::A;
use leptos_router::hooks::use_location;

#[component]
pub fn NavBar() -> impl IntoView {
    let location = use_location();
    view! {
        <nav class="flex items-center gap-1 p-2 flex-wrap">

            <A href="/" attr:class="flex gap-2 px-2 py-1">
                <Icon icon=icondata::BsCloudFog2Fill width="30" height="30"/>
                <div class="text-xl">"SelfCloud"</div>
            </A>
            <div></div>

            {move || {
                let path = location.pathname.get();
                let path_splits = path.trim_start_matches("/").split("/");
                let mut url = "/".to_string();
                let mut elements = vec![];
                for path in path_splits {
                    url.push_str(path);
                    let element_url = url.to_string();
                    let element_text = path.to_string();
                    url.push_str("/");
                    elements
                        .push(
                            view! {
                                <div>"/"</div>
                                <A
                                    attr:class="px-2 py-1 dark:hover:bg-white/20 cursor-pointer"
                                    href=element_url
                                >
                                    {element_text}
                                </A>
                            },
                        )
                }
                elements
            }}

            <div class="flex-grow"></div>
            <A href="/settings" attr:class="px-2 py-1 dark:hover:bg-white/20 cursor-pointer flex items-center gap-1">
                 <Icon icon=icondata::IoSettingsSharp width="24" height="24"/>
            </A>

        </nav>
        <hr/>
    }
}
