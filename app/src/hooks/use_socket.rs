use leptos::prelude::*;
use wasm_bindgen::{prelude::*, JsCast};
use web_sys::{BinaryType, MessageEvent, WebSocket};

#[derive(Clone, Debug, PartialEq)]
pub enum WsMessage {
    Text(String),
    Binary(Vec<u8>),
}

#[derive(Clone)]
pub struct UseSocketReturn {
    pub message: Signal<Option<WsMessage>>,
    pub ready_state: Signal<u16>,
    pub send_text: std::sync::Arc<dyn Fn(&str) + Send + Sync>,
    pub send_bytes: std::sync::Arc<dyn Fn(&[u8]) + Send + Sync>,
}

// WebSocket is !Send, but we need it in StoredValue which usually requires Send in 0.8
// Since we are in WASM (single thread), this is safe unless we spawn threads.
struct SendWebSocket(WebSocket);
unsafe impl Send for SendWebSocket {}
unsafe impl Sync for SendWebSocket {}

pub fn use_socket(url: &str) -> UseSocketReturn {
    let (message, set_message) = signal(None::<WsMessage>);
    let (ready_state, set_ready_state) = signal(0_u16); // WebSocket.CONNECTING = 0

    let ws_ref = StoredValue::new(None::<SendWebSocket>);

    // Send structure
    // We need clones for the closures
    let send_ws_ref = ws_ref.clone();
    let send_text = std::sync::Arc::new(move |text: &str| {
        send_ws_ref.with_value(|ws| {
            if let Some(ws) = ws {
                let _ = ws.0.send_with_str(text);
            }
        });
    });

    let send_ws_ref_bytes = ws_ref.clone();
    let send_bytes = std::sync::Arc::new(move |bytes: &[u8]| {
        send_ws_ref_bytes.with_value(|ws| {
            if let Some(ws) = ws {
                let _ = ws.0.send_with_u8_array(bytes);
            }
        });
    });

    // Effect to connect
    let url = url.to_string();
    Effect::new(move |_| {
        // In SSR this runs? No, usually check specific cfg.
        // We assume CSR for this hook or check context.
        if cfg!(feature = "ssr") {
            return;
        }

        // Add host prefix if relative
        let final_url = if url.starts_with('/') {
            let loc = web_sys::window().unwrap().location();
            let host = loc.host().unwrap();
            let protocol = if loc.protocol().unwrap() == "https:" {
                "wss:"
            } else {
                "ws:"
            };
            format!("{}//{}/{}", protocol, host, url.trim_start_matches('/'))
        } else {
            url.clone()
        };

        if let Ok(ws) = WebSocket::new(&final_url) {
            ws.set_binary_type(BinaryType::Arraybuffer);
            set_ready_state.set(ws.ready_state());

            // onopen
            let onopen_cb = Closure::<dyn Fn()>::new({
                let ws = ws.clone();
                move || {
                    set_ready_state.set(ws.ready_state());
                }
            });
            ws.set_onopen(Some(onopen_cb.as_ref().unchecked_ref()));
            onopen_cb.forget();

            // onmessage
            let onmessage_cb = Closure::<dyn Fn(MessageEvent)>::new(move |e: MessageEvent| {
                if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
                    set_message.set(Some(WsMessage::Text(txt.into())));
                } else if let Ok(buf) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
                    let array = js_sys::Uint8Array::new(&buf);
                    set_message.set(Some(WsMessage::Binary(array.to_vec())));
                }
            });
            ws.set_onmessage(Some(onmessage_cb.as_ref().unchecked_ref()));
            onmessage_cb.forget();

            // onclose, onerror... ignored for simplicity but should update ready_state
            let onclose_cb = Closure::<dyn Fn()>::new({
                let ws = ws.clone();
                move || {
                    set_ready_state.set(ws.ready_state());
                }
            });
            ws.set_onclose(Some(onclose_cb.as_ref().unchecked_ref()));
            onclose_cb.forget();

            ws_ref.set_value(Some(SendWebSocket(ws)));
        }
    });

    // Cleanup? Effect return?
    // Leptos 0.8 Effect doesn't return cleanup. use on_cleanup.
    on_cleanup(move || {
        ws_ref.with_value(|ws| {
            if let Some(ws) = ws {
                let _ = ws.0.close();
            }
        });
    });

    UseSocketReturn {
        message: message.into(),
        ready_state: ready_state.into(),
        send_text,
        send_bytes,
    }
}
