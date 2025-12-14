use crate::common::TtyChunk;
use crate::hooks::use_socket::{use_socket, WsMessage};
use crate::utils::xterm::Terminal;
use leptos::prelude::*;
use serde::Serialize;
use wasm_bindgen::JsCast;

#[derive(Clone)]
struct SendTerminal(Terminal);
unsafe impl Send for SendTerminal {}
unsafe impl Sync for SendTerminal {}

#[component]
pub fn TerminalComponent<S>(url: S) -> impl IntoView
where
    S: Into<String> + Clone + 'static,
{
    let url = url.clone().into();
    let div_ref: NodeRef<leptos::html::Div> = NodeRef::new();
    let terminal = StoredValue::new(None::<SendTerminal>);

    let socket = use_socket(&url);
    let message = socket.message;
    // let ready_state = socket.ready_state;

    Effect::new(move |_| {
        message.with(|msg| {
            terminal.update_value(|term_wrap| {
                if let Some(wrap) = term_wrap {
                    if let Some(WsMessage::Binary(bytes)) = msg {
                        if let Ok(chunk) = bincode::deserialize::<TtyChunk>(&bytes) {
                            let data = match chunk {
                                TtyChunk::StdOut(b) | TtyChunk::StdErr(b) => b,
                                _ => return,
                            };
                            if !data.is_empty() {
                                let uint8_arr = js_sys::Uint8Array::from(data.as_slice());
                                wrap.0.write(&uint8_arr.into());
                            }
                        }
                    }
                }
            });
        });
    });

    view! {
        <div class="h-full flex flex-col relative w-full">
            <link href="/css/xterm.min.css" rel="stylesheet"/>
            <script
                src="/js/xterm.min.js"
                on:load=move |_| {
                     #[derive(Serialize)]
                     struct TerminalOptions {
                         scrollback: u64,
                         #[serde(rename = "cursorBlink")]
                         cursor_blink: bool,
                         theme: TerminalTheme,
                         #[serde(rename = "fontFamily")]
                         font_family: String,
                     }
                     #[derive(Serialize)]
                     struct TerminalTheme {
                         background: String,
                     }

                    let options = serde_wasm_bindgen::to_value(&TerminalOptions {
                        scrollback: 1000,
                        cursor_blink: true,
                        theme: TerminalTheme { background: "#000000".to_string() },
                        font_family: "monospace".to_string()
                    });

                    if let Ok(options) = options {
                        let term = Terminal::new(&options);
                        if let Some(div) = div_ref.get_untracked() {
                            term.open(&div);

                            let send = socket.send_text.clone();
                            let callback = wasm_bindgen::closure::Closure::wrap(Box::new(move |data: wasm_bindgen::JsValue| {
                                  if let Some(input) = data.as_string() {
                                      send(&input);
                                  }
                             }) as Box<dyn FnMut(wasm_bindgen::JsValue)>);

                             term.onData(callback.as_ref().unchecked_ref());
                             callback.forget();

                             terminal.set_value(Some(SendTerminal(term)));
                        }
                    }
                }
            >
            </script>

            <div node_ref=div_ref class="flex-1 bg-black overflow-hidden pl-2 pt-2"></div>
        </div>
    }
}
