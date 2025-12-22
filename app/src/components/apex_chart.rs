use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = ApexCharts)]
    type ApexCharts;

    #[wasm_bindgen(constructor, js_class = "ApexCharts", catch)]
    fn new(element: &web_sys::Element, options: JsValue) -> Result<ApexCharts, JsValue>;

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
    let height_creation = height.clone();
    Effect::new(move |_| {
        let div = document().get_element_by_id(&id_clone);
        if let Some(div) = div {
            // Use get_untracked to prevent re-creation loop when options update
            let mut opts = options.get_untracked();

            // Merge default options with user options
            if let Some(obj) = opts.as_object_mut() {
                if !obj.contains_key("chart") {
                    obj.insert("chart".to_string(), serde_json::json!({}));
                }
                if let Some(chart) = obj.get_mut("chart").and_then(|c| c.as_object_mut()) {
                    chart.insert(
                        "height".to_string(),
                        serde_json::Value::String(
                            height_creation.clone().unwrap_or("350".to_string()),
                        ),
                    );
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

            // serde_wasm_bindgen::to_value converts Maps to ES6 Maps by default, which ApexCharts can't read.
            // We need a Plain Old JavaScript Object (POJO).
            // A reliable way is to serialize to JSON string and parse in JS.
            let options_str = serde_json::to_string(&opts).unwrap();
            let options_js = js_sys::JSON::parse(&options_str).unwrap();

            // Retry mechanism for loading ApexCharts using SendWrapper for StoredValue
            // We use Rc so that the inner type is Clone, satisfying StoredValue::get_value requirements
            let check_closure_ref =
                StoredValue::new(None::<SendWrapper<std::rc::Rc<Closure<dyn FnMut()>>>>);
            let retries = StoredValue::new(0);

            let div_clone = div.clone();

            // Define the closure to check for ApexCharts
            // We need to move check_closure_ref into the closure to clear itself
            let check_fn = move || {
                web_sys::console::log_1(&"Checking for ApexCharts globally...".into());

                let window = web_sys::window().unwrap();
                let has_apex = js_sys::Reflect::has(&window, &"ApexCharts".into()).unwrap_or(false);

                if has_apex {
                    web_sys::console::log_1(&"ApexCharts found, initializing...".into());

                    // Inspect what ApexCharts actually is
                    let apex_val = js_sys::Reflect::get(&window, &"ApexCharts".into()).unwrap();
                    web_sys::console::log_2(&"Window.ApexCharts type:".into(), &apex_val);
                    web_sys::console::log_2(&"Options:".into(), &options_js);

                    match ApexCharts::new(&div_clone, options_js.clone()) {
                        Ok(chart) => {
                            web_sys::console::log_1(&"ApexCharts created".into());
                            chart.render();
                            web_sys::console::log_1(&"ApexCharts rendered".into());
                            chart_ref.set_value(Some(SendWrapper(chart)));

                            // Cleanup check mechanism
                            check_closure_ref.set_value(None);
                        }
                        Err(err) => {
                            web_sys::console::error_2(&"Error creating ApexCharts:".into(), &err);
                            // Do not retry if constructor fails, it likely won't fix itself
                            check_closure_ref.set_value(None);
                        }
                    }
                } else {
                    let r = retries.get_value();
                    if r < 20 {
                        retries.set_value(r + 1);
                        web_sys::console::log_1(
                            &format!("ApexCharts not found, retrying {}...", r).into(),
                        );

                        // Re-schedule
                        if let Some(wrapper) = check_closure_ref.get_value() {
                            // wrapper.0 is Rc<Closure>
                            // we need to get &JsValue from it
                            let closure: &Closure<dyn FnMut()> = &*wrapper.0;
                            let js_val: &web_sys::js_sys::Function =
                                closure.as_ref().unchecked_ref();
                            window.request_animation_frame(js_val).unwrap();
                        }
                    } else {
                        web_sys::console::error_1(
                            &"ApexCharts not found after retries. Script might not be loaded."
                                .into(),
                        );
                        check_closure_ref.set_value(None);
                    }
                }
            };

            let closure = Closure::wrap(Box::new(check_fn) as Box<dyn FnMut()>);
            check_closure_ref.set_value(Some(SendWrapper(std::rc::Rc::new(closure))));

            // Start the check
            if let Some(wrapper) = check_closure_ref.get_value() {
                let closure: &Closure<dyn FnMut()> = &*wrapper.0;
                let js_val: &web_sys::js_sys::Function = closure.as_ref().unchecked_ref();
                web_sys::window()
                    .unwrap()
                    .request_animation_frame(js_val)
                    .unwrap();
            }
        }
    });

    // Effect to handle option updates
    let height_clone = height.clone();
    Effect::new(move |_| {
        // This effect will run when options change
        let mut opts = options.get();
        if let Some(wrapper) = chart_ref.get_value() {
            let chart = &wrapper.0;

            // Merge default options again for updates (in case they changed)
            if let Some(obj) = opts.as_object_mut() {
                // Ensure chart object exists
                if !obj.contains_key("chart") {
                    obj.insert("chart".to_string(), serde_json::json!({}));
                }

                if let Some(chart) = obj.get_mut("chart").and_then(|c| c.as_object_mut()) {
                    // Update height if needed
                    chart.insert(
                        "height".to_string(),
                        serde_json::Value::String(
                            height_clone.clone().unwrap_or("350".to_string()),
                        ),
                    );
                    // Ensure animations stay disabled if not specified
                    if !chart.contains_key("animations") {
                        chart.insert(
                            "animations".to_string(),
                            serde_json::json!({ "enabled": false }),
                        );
                    }
                }
            }

            let options_str = serde_json::to_string(&opts).unwrap();
            let options_js = js_sys::JSON::parse(&options_str).unwrap();

            chart.update_options(options_js);
        }
    });

    Effect::new(move |_| {
        // Access value with .with or .get_value
        if let Some(wrapper) = chart_ref.get_value() {
            let chart = &wrapper.0;
            let s = series.get();

            // Debug logs
            let s_len = s.iter().map(|series| series.data.len()).sum::<usize>();
            web_sys::console::log_1(&format!("Updating series. Total points: {}", s_len).into());
            if s_len > 0 {
                // Log the first point of the first series for verification
                if let Some(first) = s.first() {
                    if let Some(point) = first.data.first() {
                        web_sys::console::log_1(&format!("First data point: {:?}", point).into());
                    }
                }
            }

            let s_str = serde_json::to_string(&s).unwrap();
            let s_js = js_sys::JSON::parse(&s_str).unwrap();
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
