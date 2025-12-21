use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = ApexCharts)]
    type ApexCharts;

    #[wasm_bindgen(constructor, js_class = "ApexCharts")]
    fn new(element: &web_sys::Element, options: JsValue) -> ApexCharts;

    #[wasm_bindgen(method, js_class = "ApexCharts")]
    fn render(this: &ApexCharts);

    #[wasm_bindgen(method, js_class = "ApexCharts", js_name = updateSeries)]
    fn update_series(this: &ApexCharts, new_series: JsValue);

    #[wasm_bindgen(method, js_class = "ApexCharts", js_name = updateOptions)]
    fn update_options(this: &ApexCharts, new_options: JsValue);

    #[wasm_bindgen(method, js_class = "ApexCharts")]
    fn destroy(this: &ApexCharts);
}

#[derive(Clone)]
struct SendWrapper<T>(pub T);
unsafe impl<T> Send for SendWrapper<T> {}
unsafe impl<T> Sync for SendWrapper<T> {}

impl Clone for ApexCharts {
    fn clone(&self) -> Self {
        use wasm_bindgen::JsValue;
        let js_val: &JsValue = self.as_ref();
        js_val.clone().unchecked_into()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ChartSeries {
    pub name: String,
    pub data: Vec<(i64, f64)>, // Timestamp, Value
}

#[component]
pub fn ApexChart(
    #[prop(into)] series: Signal<Vec<ChartSeries>>,
    #[prop(into)] options: Signal<serde_json::Value>,
    #[prop(optional, into)] height: Option<String>,
) -> impl IntoView {
    let id = format!("chart-{}", uuid::Uuid::new_v4());
    // Use SendWrapper to satisfy StoredValue requirements in generic contexts,
    // even though we only access it on the main thread in WASM.
    let chart_ref = StoredValue::new(None::<SendWrapper<ApexCharts>>);

    let id_clone = id.clone();
    Effect::new(move |_| {
        let div = document().get_element_by_id(&id_clone);
        if let Some(div) = div {
            let mut opts = options.get();

            // Merge default options with user options
            if let Some(obj) = opts.as_object_mut() {
                if !obj.contains_key("chart") {
                    obj.insert("chart".to_string(), serde_json::json!({}));
                }
                if let Some(chart) = obj.get_mut("chart").and_then(|c| c.as_object_mut()) {
                    chart.insert(
                        "height".to_string(),
                        serde_json::Value::String(height.clone().unwrap_or("350".to_string())),
                    );
                    // Disable animations for performance if needed, or keep them
                    if !chart.contains_key("animations") {
                        chart.insert(
                            "animations".to_string(),
                            serde_json::json!({ "enabled": false }),
                        );
                    }
                }
                if !obj.contains_key("series") {
                    obj.insert("series".to_string(), serde_json::json!([]));
                }
            }

            let options_js = serde_wasm_bindgen::to_value(&opts).unwrap();
            web_sys::console::log_1(&"Attempting to create ApexCharts".into());

            // Check if ApexCharts is defined (this is hard in Rust wasm-bindgen without using js_sys::Reflect or similar on window)
            // But if the bindgen call fails, it usually throws a JS error.
            // Let's wrap in a try-catch equivalent if possible, or just log before.

            let chart = ApexCharts::new(&div, options_js);
            web_sys::console::log_1(&"ApexCharts created".into());

            chart.render();
            web_sys::console::log_1(&"ApexCharts rendered".into());

            chart_ref.set_value(Some(SendWrapper(chart)));
        }
    });

    Effect::new(move |_| {
        // Access value with .with or .get_value
        if let Some(wrapper) = chart_ref.get_value() {
            let chart = &wrapper.0;
            let s = series.get();
            let s_js = serde_wasm_bindgen::to_value(&s).unwrap();
            chart.update_series(s_js);
        }
    });

    on_cleanup(move || {
        if let Some(wrapper) = chart_ref.get_value() {
            wrapper.0.destroy();
        }
    });

    view! {
        <div id=id class="w-full text-black"></div>
    }
}
