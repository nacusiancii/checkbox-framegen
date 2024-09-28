use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::State,
    response::IntoResponse,
};
use futures::{SinkExt, StreamExt};
use std::sync::{Arc, Mutex};
use crate::{state::AppState, frame::{Frame, ClientMessage}};

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<Mutex<AppState>>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<Mutex<AppState>>) {
    let mut rx = {
        let state = state.lock().unwrap();
        state.tx.subscribe()
    };

    // Send initial I-frame
    send_initial_frame(&mut socket, &state).await;

    // Handle incoming messages (including reconnection requests)
    let state_clone = state.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(message)) = socket.recv().await {
            match message {
                Message::Text(text) => {
                    if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                        handle_client_reconnect(&mut socket, &state_clone, client_msg.last_version).await;
                    }
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    // Stream frames to the client
    let mut send_task = tokio::spawn(async move {
        while let Ok(frame) = rx.recv().await {
            let message = Message::Binary(bincode::serialize(&frame).unwrap());
            if socket.send(message).await.is_err() {
                break;
            }
        }
    });

    // Wait for either incoming or outgoing to finish
    tokio::select! {
        _ = (&mut recv_task) => send_task.abort(),
        _ = (&mut send_task) => recv_task.abort(),
    };
}

async fn send_initial_frame(socket: &mut WebSocket, state: &Arc<Mutex<AppState>>) {
    let state = state.lock().unwrap();
    let initial_frame = Frame::IFrame {
        version: state.version,
        timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
        data: state.state.clone().into_vec(),
    };
    let _ = socket
        .send(Message::Binary(bincode::serialize(&initial_frame).unwrap()))
        .await;
}

async fn handle_client_reconnect(
    socket: &mut WebSocket,
    state: &Arc<Mutex<AppState>>,
    client_version: usize,
) {
    let state = state.lock().unwrap();
    let current_version = state.version;

    if client_version == current_version {
        // Client is up-to-date
        return;
    }

    if current_version - client_version > crate::MAX_CATCHUP_FRAMES || client_version > current_version {
        // Too many missed frames or client version is in the future, send full state
        drop(state); // Unlock the mutex before calling send_initial_frame
        send_initial_frame(&mut socket, &state).await;
    } else {
        // Send missed P-frames
        let missed_frames: Vec<Frame> = state.frame_buffer
            .iter()
            .filter(|frame| match frame {
                Frame::IFrame { version, .. } | Frame::PFrame { version, .. } => *version > client_version
            })
            .cloned()
            .collect();

        for frame in missed_frames {
            let _ = socket
                .send(Message::Binary(bincode::serialize(&frame).unwrap()))
                .await;
        }
    }
}
