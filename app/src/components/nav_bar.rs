use leptos::{component, view, IntoView, SignalGet};
use leptos_icons::Icon;
use leptos_router::{use_router, A};

#[component]
pub fn NavBar() -> impl IntoView {
    let route = use_router();
    view! {
        <nav class="flex items-center gap-1 p-2 flex-wrap">

            <A href="/" class="flex gap-2 px-2 py-1">
                <Icon icon=icondata::BsCloudFog2Fill width="30" height="30"/>
                <div class="text-xl">"SelfCloud"</div>
            </A>
            <div></div>

            {move || {
                let path = route.pathname().get();
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
                                    class="px-2 py-1 dark:hover:bg-white/20 cursor-pointer"
                                    href=element_url
                                >
                                    {element_text}
                                </A>
                            },
                        )
                }
                elements
            }}

        </nav>
        <hr/>
    }
}
