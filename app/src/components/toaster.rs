use leptos::prelude::*;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq)]
pub enum ToastVariant {
    Success,
    Error,
    Info,
    Warning,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ToastData {
    pub id: Uuid,
    pub title: String,
    pub variant: ToastVariant,
}

#[derive(Clone)]
pub struct ToasterContext {
    pub toasts: RwSignal<Vec<ToastData>>,
}

impl ToasterContext {
    pub fn toast(&self, title: impl Into<String>, variant: ToastVariant) {
        let id = Uuid::new_v4();
        let title = title.into();
        let toast = ToastData { id, title, variant };

        self.toasts.update(|t| t.push(toast));

        // Auto remove after 3 seconds
        let toasts = self.toasts;
        set_timeout(
            move || {
                toasts.update(|t| t.retain(|x| x.id != id));
            },
            std::time::Duration::from_millis(3000),
        );
    }
}

pub fn provide_toaster() -> ToasterContext {
    let toasts = RwSignal::new(Vec::new());
    let ctx = ToasterContext { toasts };
    provide_context(ctx.clone());
    ctx
}

#[component]
pub fn Toaster(children: Children) -> impl IntoView {
    let ctx = provide_toaster();

    view! {
        {children()}
        <div class="fixed bottom-4 right-4 z-50 flex flex-col gap-2">
            <For
                each=move || ctx.toasts.get()
                key=|t| t.id
                children=move |t| {
                    let variant_classes = match t.variant {
                        ToastVariant::Success => "bg-green-500 text-white",
                        ToastVariant::Error => "bg-red-500 text-white",
                        ToastVariant::Info => "bg-blue-500 text-white",
                        ToastVariant::Warning => "bg-yellow-500 text-black",
                    };
                    view! {
                        <div class=format!("p-4 rounded shadow-lg min-w-[200px] transition-all {}", variant_classes)>
                            <div class="font-bold">{t.title}</div>
                        </div>
                    }
                }
            />
        </div>
    }
}
