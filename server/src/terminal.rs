use app::common::{AttachParams, TtyChunk};
use axum::{
    extract::{
        ws::{Message, WebSocket},
        Query, WebSocketUpgrade,
    },
    response::Response,
};
use axum_extra::extract::cookie::CookieJar;
use futures::{SinkExt, StreamExt};
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use tracing::{info, warn};

use crate::container_events::ensure_authorized_user;

pub async fn terminal_ws(
    jar: CookieJar,
    Query(params): Query<AttachParams>,
    ws: WebSocketUpgrade,
) -> Result<Response, (axum::http::StatusCode, String)> {
    ensure_authorized_user(jar)?;
    Ok(ws.on_upgrade(move |socket| handle_terminal_socket(socket, params)))
}

async fn handle_terminal_socket(socket: WebSocket, params: AttachParams) {
    let pty_system = native_pty_system();
    let size = PtySize {
        rows: params.size_height as u16,
        cols: params.size_width as u16,
        pixel_width: 0,
        pixel_height: 0,
    };

    let pair = match pty_system.openpty(size) {
        Ok(pair) => pair,
        Err(e) => {
            warn!("Failed to open PTY: {e:?}");
            return;
        }
    };

    let cmd = CommandBuilder::new("bash");
    let mut child = match pair.slave.spawn_command(cmd) {
        Ok(child) => child,
        Err(e) => {
            warn!("Failed to spawn command: {e:?}");
            return;
        }
    };

    let mut reader = match pair.master.try_clone_reader() {
        Ok(reader) => reader,
        Err(e) => {
            warn!("Failed to clone reader: {e:?}");
            return;
        }
    };

    let writer = Arc::new(Mutex::new(match pair.master.take_writer() {
        Ok(writer) => writer,
        Err(e) => {
            warn!("Failed to take writer: {e:?}");
            return;
        }
    }));

    let (mut sender_sock, mut receiver_sock) = socket.split();

    // Spawn a thread to read from PTY and send to WS
    // Since portable_pty reader is blocking, we use spawn_blocking or a dedicated thread.
    // However, we need to send async to WS.
    // Better to use a sync->async channel.
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Vec<u8>>(32);

    std::thread::spawn(move || {
        let mut buf = [0u8; 1024];
        loop {
            match reader.read(&mut buf) {
                Ok(n) if n > 0 => {
                    if tx.blocking_send(buf[..n].to_vec()).is_err() {
                        break;
                    }
                }
                _ => break,
            }
        }
    });

    let send_task = tokio::spawn(async move {
        while let Some(data) = rx.recv().await {
            let chunk = TtyChunk::StdOut(data); // Treat all as StdOut for simplicity
            if let Ok(bytes) = bincode::serialize(&chunk) {
                if sender_sock
                    .send(Message::Binary(bytes.into()))
                    .await
                    .is_err()
                {
                    break;
                }
            }
        }
    });

    // Read from WS and write to PTY
    let writer_clone = writer.clone();
    let recv_task = tokio::spawn(async move {
        while let Some(msg) = receiver_sock.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    let mut w = writer_clone.lock().unwrap();
                    let _ = w.write_all(text.as_bytes());
                }
                Ok(Message::Binary(bin)) => {
                    // Try to deserialize if it's a specific message, or just raw bytes?
                    // Frontend 'xterm.rs' sends raw strings via socket.send_text.
                    // But if it was `container_page.rs` logic: `socket.send(&input);` which calls `send_text`.
                    // So we expect text.
                    // BUT `xterm.rs` calls `send` which is `send_text`.
                    // If we want to support resizing, we might need to parse JSON commands or specific binary formats.
                    // For now, assume simple text input.
                    let mut w = writer_clone.lock().unwrap();
                    let _ = w.write_all(&bin);
                }
                _ => {}
            }
        }
    });

    // Wait for tasks
    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
    }

    // Cleanup
    let _ = child.kill();
}
