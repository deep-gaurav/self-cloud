use leptos::{component, view, Attribute, IntoView};

#[component]
pub fn InputField(#[prop(attrs)] attrs: Vec<(&'static str, Attribute)>) -> impl IntoView {
    view! {
        <input
            class="p-2 border w-full rounded bg-white dark:bg-white/10 dark:border-white/5"
            {..attrs}
        />
    }
}
